// mcp-client/src/transport/mod.rs
pub mod stdio;

use async_trait::async_trait;
use anyhow::Result;
use mcp_protocol::messages::JsonRpcMessage;

/// Transport trait for sending and receiving MCP messages
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Start the transport (listening for incoming messages)
    async fn start(&self) -> Result<()>;
    
    /// Send a message to the server
    async fn send(&self, message: JsonRpcMessage) -> Result<()>;
    
    /// Close the transport
    async fn close(&self) -> Result<()>;
}

pub use stdio::StdioTransport;
