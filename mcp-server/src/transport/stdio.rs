// mcp-server/src/transport/stdio.rs
use anyhow::Result;
use async_trait::async_trait;
use mcp_protocol::messages::JsonRpcMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::debug;

/// Transport implementation that uses stdio to communicate with the client
#[derive(Clone)]
pub struct StdioTransport;

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl super::Transport for StdioTransport {
    async fn start(&self, message_tx: mpsc::Sender<JsonRpcMessage>) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        tokio::spawn(async move {
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                debug!("Received line: {}", line);
                match serde_json::from_str::<JsonRpcMessage>(&line) {
                    Ok(message) => {
                        if message_tx.send(message).await.is_err() {
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
        let mut stdout = tokio::io::stdout();
        let serialized = serde_json::to_string(&message)?;

        debug!("Sending message: {}", serialized);
        stdout.write_all(serialized.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // No need to do anything special for stdio
        Ok(())
    }
    
    fn box_clone(&self) -> Box<dyn super::Transport> {
        Box::new(self.clone())
    }
}
