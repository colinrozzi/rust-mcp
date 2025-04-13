// mcp-client/src/transport/stdio.rs
use anyhow::Result;
use async_trait::async_trait;
use mcp_protocol::messages::JsonRpcMessage;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

/// Transport implementation that uses stdio to communicate with a child process
pub struct StdioTransport {
    child_process: Arc<Mutex<Option<Child>>>,
    tx: mpsc::Sender<JsonRpcMessage>,
    command: String,
    args: Vec<String>,
}

impl StdioTransport {
    /// Create a new stdio transport with the given command and arguments
    pub fn new(command: &str, args: Vec<String>) -> (Self, mpsc::Receiver<JsonRpcMessage>) {
        let (tx, rx) = mpsc::channel(100);

        let transport = Self {
            child_process: Arc::new(Mutex::new(None)),
            tx,
            command: command.to_string(),
            args,
        };

        (transport, rx)
    }
}

#[async_trait]
impl super::Transport for StdioTransport {
    async fn start(&self) -> Result<()> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to get stdout");
        let stdin = child.stdin.take().expect("Failed to get stdin");

        let mut guard = self.child_process.lock().await;
        *guard = Some(child);
        drop(guard);

        let stdin = Arc::new(Mutex::new(stdin));
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                match serde_json::from_str::<JsonRpcMessage>(&line) {
                    Ok(message) => {
                        if tx.send(message).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        tracing::error!("Failed to parse JSON-RPC message: {}", err);
                    }
                }

                line.clear();
            }
        });

        Ok(())
    }

    async fn send(&self, message: JsonRpcMessage) -> Result<()> {
        let guard = self.child_process.lock().await;
        let child = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Child process not started"))?;

        let stdin = child
            .stdin
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;

        let serialized = serde_json::to_string(&message)?;
        let mut stdin = tokio::process::ChildStdin::from_std(stdin.try_clone()?)?;

        stdin.write_all(serialized.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut guard = self.child_process.lock().await;

        if let Some(mut child) = guard.take() {
            // Close stdin to signal EOF
            drop(child.stdin.take());

            // Wait for a short time for the process to exit gracefully
            match tokio::time::timeout(std::time::Duration::from_secs(1), child.wait()).await {
                Ok(Ok(_)) => return Ok(()),
                _ => {
                    // If it doesn't exit, kill it
                    child.kill()?;
                    child.wait()?;
                }
            }
        }

        Ok(())
    }
}
