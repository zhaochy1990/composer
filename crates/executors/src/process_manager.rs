use crate::types::CliMessage;
use crate::protocol::read_stdout_lines;
use crate::error::ExecutorError;
use composer_api_types::{WsEvent, LogType};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use std::process::Stdio;

pub struct SpawnOptions {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub prompt: String,
    pub working_dir: String,
    pub auto_approve: bool,
    pub resume_session_id: Option<String>,
}

pub struct AgentProcess {
    pub session_id: Uuid,
    pub agent_id: Uuid,
    pub cancel_token: CancellationToken,
    pub join_handle: JoinHandle<()>,
}

pub struct AgentProcessManager {
    processes: Arc<DashMap<Uuid, AgentProcess>>,
    event_tx: broadcast::Sender<WsEvent>,
}

impl AgentProcessManager {
    pub fn new(event_tx: broadcast::Sender<WsEvent>) -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            event_tx,
        }
    }

    pub async fn spawn(&self, opts: SpawnOptions) -> Result<(), ExecutorError> {
        let session_id = opts.session_id;
        let agent_id = opts.agent_id;
        let task_id = opts.task_id;

        let npx_cmd = if cfg!(target_os = "windows") { "npx.cmd" } else { "npx" };

        let mut args = vec![
            "-y".to_string(),
            "@anthropic-ai/claude-code@latest".to_string(),
            "--verbose".to_string(),
            "--output-format=stream-json".to_string(),
            "--input-format=stream-json".to_string(),
        ];

        if opts.auto_approve {
            args.push("--dangerously-skip-permissions".to_string());
        }

        if let Some(ref resume_id) = opts.resume_session_id {
            args.push("--resume".to_string());
            args.push(resume_id.clone());
        }

        args.push("-p".to_string());
        args.push(opts.prompt.clone());

        let mut child = Command::new(npx_cmd)
            .args(&args)
            .current_dir(&opts.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ExecutorError::SpawnFailed(e.to_string()))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stdout".into()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stderr".into()))?;

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

        // Stdout reader: parse Claude Code JSON output
        let stdout_handle = tokio::spawn(async move {
            read_stdout_lines(
                stdout,
                |msg, raw_line| {
                    let _ = event_tx.send(WsEvent::SessionOutput {
                        session_id,
                        log_type: LogType::Stdout,
                        content: raw_line.to_string(),
                    });

                    if let CliMessage::Result(result) = msg {
                        let is_error = result.is_error.unwrap_or(false);
                        if is_error {
                            let _ = event_tx.send(WsEvent::SessionFailed {
                                session_id,
                                error: result.result.unwrap_or_else(|| "Unknown error".to_string()),
                            });
                        } else {
                            let _ = event_tx.send(WsEvent::SessionCompleted {
                                session_id,
                                result_summary: result.result,
                            });
                        }
                    }
                },
                |raw_line| {
                    // Fix #12: Emit non-JSON lines as Control events
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

        // Monitor: wait for completion or cancellation
        // Move child into the monitor task so kill_on_drop doesn't fire prematurely
        let monitor_handle = tokio::spawn(async move {
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    tracing::info!("Session {} cancelled, killing process", session_id);
                    let _ = child.kill().await;
                }
                _ = stdout_handle => {
                    tracing::info!("Session {} stdout closed, waiting for process exit", session_id);
                }
            }
            // Wait for the process to fully exit (prevents zombies)
            let _ = child.wait().await;
            let _ = stderr_handle.await;
            processes.remove(&session_id);
        });

        self.processes.insert(session_id, AgentProcess {
            session_id,
            agent_id,
            cancel_token,
            join_handle: monitor_handle,
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
                tokio::time::Duration::from_secs(5),
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
}
