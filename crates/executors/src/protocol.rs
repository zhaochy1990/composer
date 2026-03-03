use crate::types::CliMessage;
use crate::error::ExecutorError;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct ProtocolPeer {
    pub stdin: Arc<Mutex<ChildStdin>>,
}

impl ProtocolPeer {
    pub fn new(stdin: ChildStdin) -> Self {
        Self {
            stdin: Arc::new(Mutex::new(stdin)),
        }
    }

    pub async fn send_message<T: serde::Serialize>(&self, msg: &T) -> Result<(), ExecutorError> {
        let json = serde_json::to_string(msg)?;
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(json.as_bytes()).await.map_err(ExecutorError::Io)?;
        stdin.write_all(b"\n").await.map_err(ExecutorError::Io)?;
        stdin.flush().await.map_err(ExecutorError::Io)?;
        Ok(())
    }
}

pub async fn read_stdout_lines(
    stdout: ChildStdout,
    mut on_message: impl FnMut(CliMessage, &str),
) {
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<CliMessage>(trimmed) {
            Ok(msg) => on_message(msg, trimmed),
            Err(_) => {
                tracing::trace!("Non-JSON stdout: {}", trimmed);
            }
        }
    }
}
