use crate::types::{CliMessage, SDKControlResponse, UserMessage};
use crate::protocol::{read_stdout_lines, ProtocolPeer};
use crate::error::ExecutorError;
use composer_api_types::{WsEvent, LogType};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use std::process::Stdio;

/// Pinned Claude Code CLI version. Bump deliberately after testing.
pub const CLAUDE_CODE_VERSION: &str = "2.1.66";

pub struct SpawnOptions {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub prompt: String,
    pub working_dir: String,
    pub auto_approve: bool,
    pub resume_session_id: Option<String>,
    /// Message UUID to resume from (for mid-session resume / rollback).
    pub resume_at_message_id: Option<String>,
    /// When true, close stdin after receiving a Result message so the process
    /// exits after one turn. Used by workflow engine steps.
    pub exit_on_result: bool,
}

pub struct AgentProcess {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub cancel_token: CancellationToken,
    pub join_handle: JoinHandle<()>,
    pub peer: Arc<ProtocolPeer>,
    /// Claude Code's own session ID, captured from stdout messages.
    pub claude_session_id: Arc<std::sync::Mutex<Option<String>>>,
    /// Last confirmed message UUID (committed on Result).
    pub last_message_id: Arc<std::sync::Mutex<Option<String>>>,
    /// Path to the plan file written during plan mode (`.claude/plans/*.md`).
    pub plan_file_path: Arc<std::sync::Mutex<Option<String>>>,
}

pub struct AgentProcessManager {
    processes: Arc<DashMap<Uuid, AgentProcess>>,
    event_tx: broadcast::Sender<WsEvent>,
    /// Per-session message queue: if a user sends input while Claude is mid-turn,
    /// the message is queued here and sent when the process is ready.
    pending_messages: Arc<DashMap<Uuid, String>>,
}

impl AgentProcessManager {
    pub fn new(event_tx: broadcast::Sender<WsEvent>) -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            event_tx,
            pending_messages: Arc::new(DashMap::new()),
        }
    }

    pub async fn spawn(&self, opts: SpawnOptions) -> Result<(), ExecutorError> {
        let session_id = opts.session_id;
        let agent_id = opts.agent_id;
        let task_id = opts.task_id;

        let npx_cmd = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };

        let mut args = vec![
            "-y".to_string(),
            format!("@anthropic-ai/claude-code@{}", CLAUDE_CODE_VERSION),
            "--verbose".to_string(),
            "--output-format=stream-json".to_string(),
            "--input-format=stream-json".to_string(),
            "--include-partial-messages".to_string(),
            "--replay-user-messages".to_string(),
        ];

        if opts.auto_approve {
            args.push("--dangerously-skip-permissions".to_string());
        }

        if let Some(ref resume_id) = opts.resume_session_id {
            args.push("--resume".to_string());
            args.push(resume_id.clone());

            // Resume from a specific message UUID (rollback to known-good point)
            if let Some(ref msg_id) = opts.resume_at_message_id {
                args.push("--resume-session-at".to_string());
                args.push(msg_id.clone());
            }
        }

        // Do NOT pass -p; we send the initial prompt via stdin as a UserMessage.
        // This keeps the process alive for multi-turn conversations.

        let mut child = Command::new(npx_cmd)
            .args(&args)
            .current_dir(&opts.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Prevent "nested session" detection when running inside Claude Code
            .env_remove("CLAUDECODE")
            .group_spawn()
            .map_err(|e| ExecutorError::SpawnFailed(e.to_string()))?;

        let stdin = child.inner().stdin.take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stdin".into()))?;
        let stdout = child.inner().stdout.take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stdout".into()))?;
        let stderr = child.inner().stderr.take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stderr".into()))?;

        let peer = Arc::new(ProtocolPeer::new(stdin));
        // Use std::sync::Mutex for state shared with the sync on_message callback.
        // These are only held briefly for simple writes, so no contention risk.
        let claude_session_id: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));
        let last_message_id: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));
        let plan_file_path: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));

        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        let event_tx = self.event_tx.clone();
        let event_tx2 = self.event_tx.clone();
        let event_tx3 = self.event_tx.clone();
        let processes = self.processes.clone();

        let _ = event_tx.send(WsEvent::SessionStarted {
            session_id,
            agent_id,
            task_id,
        });

        // Track the last Result message — used to determine completion when process exits.
        // std::sync::Mutex because it's written from the sync on_message callback.
        let last_result: Arc<std::sync::Mutex<Option<(Option<String>, bool)>>> =
            Arc::new(std::sync::Mutex::new(None));

        // Stdout reader: parse Claude Code JSON output
        let session_id_capture = claude_session_id.clone();
        let last_result_capture = last_result.clone();
        let last_msg_id_capture = last_message_id.clone();
        let plan_file_capture = plan_file_path.clone();
        let exit_on_result = opts.exit_on_result;
        let result_received = Arc::new(tokio::sync::Notify::new());
        let result_received_signal = result_received.clone();
        let stdout_handle = tokio::spawn(async move {
            // Track pending assistant UUID — only committed on Result
            let pending_assistant_uuid: std::sync::Mutex<Option<String>> =
                std::sync::Mutex::new(None);

            read_stdout_lines(
                stdout,
                |msg, raw_line| {
                    // Skip replayed messages from --replay-user-messages during --resume
                    let is_replay = match &msg {
                        CliMessage::User(u) => u.is_replay,
                        CliMessage::Assistant(a) => a.is_replay,
                        _ => false,
                    };
                    if is_replay {
                        return;
                    }

                    // Determine log_type based on message type
                    let log_type = match &msg {
                        CliMessage::ControlRequest { .. }
                        | CliMessage::ControlResponse { .. }
                        | CliMessage::ControlCancelRequest { .. } => LogType::Control,
                        _ => LogType::Stdout,
                    };

                    let _ = event_tx.send(WsEvent::SessionOutput {
                        session_id,
                        log_type,
                        content: raw_line.to_string(),
                    });

                    // Detect AskUserQuestion control requests
                    if let CliMessage::ControlRequest { ref request_id, ref request } = msg {
                        if let crate::types::ControlRequestPayload::CanUseTool { ref tool_name, ref input, .. } = request {
                            if tool_name == "AskUserQuestion" {
                                let _ = event_tx.send(WsEvent::UserQuestionRequested {
                                    session_id,
                                    request_id: request_id.clone(),
                                    questions: input.clone(),
                                    plan_content: None, // populated by session service
                                });
                            }
                        }
                    }

                    // Detect Write tool_use to .claude/plans/*.md for plan file tracking
                    if let CliMessage::Assistant(ref a) = msg {
                        if let Some(content) = a.message.get("content") {
                            if let Some(blocks) = content.as_array() {
                                for block in blocks {
                                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                        if block.get("name").and_then(|n| n.as_str()) == Some("Write") {
                                            if let Some(input) = block.get("input") {
                                                if let Some(file_path) = input.get("file_path").and_then(|p| p.as_str()) {
                                                    if file_path.contains(".claude/plans/") && file_path.ends_with(".md") {
                                                        *plan_file_capture.lock().unwrap() = Some(file_path.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Extract Claude Code session_id synchronously
                    let maybe_sid = match &msg {
                        CliMessage::User(u) => u.session_id.clone(),
                        CliMessage::Assistant(a) => a.session_id.clone(),
                        CliMessage::Result(r) => r.session_id.clone(),
                        _ => None,
                    };
                    if let Some(sid) = maybe_sid {
                        let mut guard = session_id_capture.lock().unwrap();
                        if guard.is_none() {
                            *guard = Some(sid.clone());
                            // Eagerly persist the Claude Code session ID so it survives server crashes
                            let _ = event_tx.send(WsEvent::SessionResumeIdCaptured {
                                session_id,
                                claude_session_id: sid,
                            });
                        }
                    }

                    // Track message UUIDs synchronously for --resume-session-at.
                    // User UUIDs committed immediately, assistant UUIDs on Result.
                    match &msg {
                        CliMessage::User(u) => {
                            if let Some(ref uuid) = u.uuid {
                                *last_msg_id_capture.lock().unwrap() = Some(uuid.clone());
                            }
                            *pending_assistant_uuid.lock().unwrap() = None;
                        }
                        CliMessage::Assistant(a) => {
                            if let Some(ref uuid) = a.uuid {
                                *pending_assistant_uuid.lock().unwrap() = Some(uuid.clone());
                            }
                        }
                        CliMessage::Result(_) => {
                            if let Some(uuid) = pending_assistant_uuid.lock().unwrap().take() {
                                *last_msg_id_capture.lock().unwrap() = Some(uuid);
                            }
                        }
                        _ => {}
                    }

                    // Store the last Result synchronously.
                    if let CliMessage::Result(result) = msg {
                        let is_error = result.is_error.unwrap_or(false);
                        *last_result_capture.lock().unwrap() = Some((result.result, is_error));

                        // For workflow steps: signal that we got the Result.
                        if exit_on_result {
                            result_received_signal.notify_one();
                        }
                    }
                },
                |raw_line| {
                    // Non-JSON lines emitted as Control events
                    let _ = event_tx3.send(WsEvent::SessionOutput {
                        session_id,
                        log_type: LogType::Control,
                        content: raw_line.to_string(),
                    });
                },
            ).await;
        });

        // Stderr reader
        let stderr_handle = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = event_tx2.send(WsEvent::SessionOutput {
                    session_id,
                    log_type: LogType::Stderr,
                    content: line,
                });
            }
        });

        // Send initial prompt via stdin as a UserMessage
        let init_peer = peer.clone();
        let init_prompt = opts.prompt.clone();
        let event_tx4 = self.event_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = init_peer.send_message(&UserMessage::new(init_prompt)).await {
                tracing::error!("Session {} failed to send initial prompt: {}", session_id, e);
                let _ = event_tx4.send(WsEvent::SessionFailed {
                    session_id,
                    error: format!("Failed to send initial prompt: {}", e),
                    claude_session_id: None,
                });
            }
        });

        // Monitor: wait for completion, cancellation, or exit_on_result signal
        let interrupt_peer = peer.clone();
        let event_tx5 = self.event_tx.clone();
        let last_result_monitor = last_result.clone();
        let claude_sid_monitor = claude_session_id.clone();
        let plan_file_monitor = plan_file_path.clone();

        // If exit_on_result is set, spawn a task that waits for the Result signal
        // and then drops stdin to make the process exit. Uses the cancel token
        // so the task is cleaned up if the process exits without a Result.
        if exit_on_result {
            let exit_peer = peer.clone();
            let result_received = result_received.clone();
            let exit_cancel = cancel_token.clone();
            tokio::spawn(async move {
                tokio::select! {
                    _ = result_received.notified() => {
                        tracing::info!("exit_on_result: closing stdin after Result received");
                        exit_peer.close_stdin().await;
                    }
                    _ = exit_cancel.cancelled() => {
                        // Process was cancelled/interrupted — no need to close stdin
                    }
                }
            });
        }

        let monitor_handle = tokio::spawn(async move {
            let was_cancelled = tokio::select! {
                _ = cancel_clone.cancelled() => {
                    tracing::info!("Session {} cancelled, sending graceful interrupt", session_id);
                    if let Err(e) = interrupt_peer.interrupt().await {
                        tracing::warn!("Session {} failed to send interrupt: {}", session_id, e);
                    }
                    match tokio::time::timeout(
                        tokio::time::Duration::from_secs(5),
                        child.wait(),
                    ).await {
                        Ok(_) => {
                            tracing::info!("Session {} exited gracefully after interrupt", session_id);
                        }
                        Err(_) => {
                            tracing::warn!("Session {} did not exit after interrupt, killing process group", session_id);
                            kill_process_group(&mut child).await;
                        }
                    }
                    true
                }
                _ = stdout_handle => {
                    tracing::info!("Session {} stdout closed, waiting for process exit", session_id);
                    false
                }
            };
            // Wait for the process to fully exit (prevents zombies)
            let _ = child.wait().await;
            let _ = stderr_handle.await;

            // Extract claude_session_id and plan_file_path BEFORE removing from the DashMap.
            // This avoids the race where the event listener tries to look it up
            // after the process entry is already removed.
            let captured_claude_sid = claude_sid_monitor.lock().unwrap().clone();
            let captured_plan_file = plan_file_monitor.lock().unwrap().clone();

            // If a plan file was written, read its content and use it as the result summary
            // so that the workflow step output contains the full plan (not a truncated summary).
            let plan_content = captured_plan_file.as_ref().and_then(|path| {
                std::fs::read_to_string(path).ok()
            });

            // Emit completion/failure now that the process has actually exited.
            // If cancelled (interrupted), the session_service handles the Paused state.
            if !was_cancelled {
                match last_result_monitor.lock().unwrap().take() {
                    Some((summary, false)) => {
                        // Prefer plan file content over result summary for plan steps
                        let effective_summary = plan_content.or(summary);
                        let _ = event_tx5.send(WsEvent::SessionCompleted {
                            session_id,
                            result_summary: effective_summary,
                            claude_session_id: captured_claude_sid,
                        });
                    }
                    Some((summary, true)) => {
                        let _ = event_tx5.send(WsEvent::SessionFailed {
                            session_id,
                            error: summary.unwrap_or_else(|| "Unknown error".to_string()),
                            claude_session_id: captured_claude_sid,
                        });
                    }
                    None => {
                        let _ = event_tx5.send(WsEvent::SessionFailed {
                            session_id,
                            error: "Process exited without a result".to_string(),
                            claude_session_id: captured_claude_sid,
                        });
                    }
                }
            }

            // Cancel the token to clean up any detached tasks (e.g., exit_on_result waiter)
            cancel_clone.cancel();
            processes.remove(&session_id);
        });

        self.processes.insert(session_id, AgentProcess {
            session_id,
            agent_id,
            cancel_token,
            join_handle: monitor_handle,
            peer,
            claude_session_id,
            last_message_id,
            plan_file_path,
        });

        Ok(())
    }

    pub async fn interrupt(&self, session_id: Uuid) -> Result<(), ExecutorError> {
        // Cancel the token first without removing the entry — the monitor task
        // will clean up the entry when it finishes.
        if let Some(entry) = self.processes.get(&session_id) {
            entry.cancel_token.cancel();
        }
        // Wait for the monitor task to finish (with timeout) then remove the entry
        if let Some((_, process)) = self.processes.remove(&session_id) {
            let _ = tokio::time::timeout(
                tokio::time::Duration::from_secs(10),
                process.join_handle,
            )
            .await;
        }
        Ok(())
    }

    pub fn is_running(&self, session_id: &Uuid) -> bool {
        self.processes.contains_key(session_id)
    }

    pub fn running_count(&self) -> usize {
        self.processes.len()
    }

    pub async fn send_input(&self, session_id: Uuid, message: String) -> Result<(), ExecutorError> {
        let entry = self.processes.get(&session_id)
            .ok_or_else(|| ExecutorError::NotRunning(session_id.to_string()))?;
        entry.peer.send_message(&UserMessage::new(message)).await
    }

    /// Close stdin on a running process, causing it to exit naturally.
    /// Unlike `interrupt()`, this produces a `SessionCompleted` event
    /// because the monitor task sees a natural exit (not a cancellation).
    pub async fn close_stdin(&self, session_id: Uuid) -> Result<(), ExecutorError> {
        let entry = self.processes.get(&session_id)
            .ok_or_else(|| ExecutorError::NotRunning(session_id.to_string()))?;
        entry.peer.close_stdin().await;
        Ok(())
    }

    /// Queue a message to be sent on the next session resume.
    /// Replaces any previously queued message for this session.
    pub fn queue_message(&self, session_id: Uuid, message: String) {
        self.pending_messages.insert(session_id, message);
    }

    /// Take and remove a queued message for a session, if any.
    pub fn take_queued_message(&self, session_id: &Uuid) -> Option<String> {
        self.pending_messages.remove(session_id).map(|(_, v)| v)
    }

    /// Get the Claude Code session ID for a running process (if captured from stdout).
    pub fn get_claude_session_id(&self, session_id: &Uuid) -> Option<String> {
        if let Some(entry) = self.processes.get(session_id) {
            entry.claude_session_id.lock().unwrap().clone()
        } else {
            None
        }
    }

    /// Get the last confirmed message UUID for --resume-session-at support.
    pub fn get_last_message_id(&self, session_id: &Uuid) -> Option<String> {
        if let Some(entry) = self.processes.get(session_id) {
            entry.last_message_id.lock().unwrap().clone()
        } else {
            None
        }
    }

    /// Get the plan file path captured from a Write tool_use to .claude/plans/*.md.
    pub fn get_plan_file_path(&self, session_id: &Uuid) -> Option<String> {
        if let Some(entry) = self.processes.get(session_id) {
            entry.plan_file_path.lock().unwrap().clone()
        } else {
            None
        }
    }

    /// Send a control response back to Claude Code's stdin (e.g., AskUserQuestion answers).
    pub async fn send_control_response(&self, session_id: Uuid, response: SDKControlResponse) -> Result<(), ExecutorError> {
        let entry = self.processes.get(&session_id)
            .ok_or_else(|| ExecutorError::NotRunning(session_id.to_string()))?;
        entry.peer.send_message(&response).await
    }
}

/// Kill a process group with signal escalation on Unix.
/// On Windows, falls back to a simple kill.
async fn kill_process_group(child: &mut AsyncGroupChild) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::{Pid, getpgid};

        if let Some(pid) = child.inner().id() {
            if let Ok(pgid) = getpgid(Some(Pid::from_raw(pid as i32))) {
                for sig in [Signal::SIGINT, Signal::SIGTERM, Signal::SIGKILL] {
                    tracing::info!("Sending {:?} to process group {}", sig, pgid);
                    if let Err(e) = killpg(pgid, sig) {
                        tracing::warn!("Failed to send {:?} to process group {}: {}", sig, pgid, e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    if child.inner().try_wait().ok().flatten().is_some() {
                        tracing::info!("Process group {} exited after {:?}", pgid, sig);
                        return;
                    }
                }
            }
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}
