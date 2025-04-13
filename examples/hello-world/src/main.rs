use anyhow::Result;
use mcp_protocol::types::tool::{ToolCallResult, ToolContent};
use mcp_server::{ServerBuilder, transport::StdioTransport};
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");
    
    info!("Starting hello-world MCP server");
    
    // Create server with stdio transport
    let server = ServerBuilder::new("hello-world", "0.1.0")
        .with_transport(StdioTransport::new())
        .with_tool(
            "hello",
            Some("Say hello to someone"),
            json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the person to greet"
                    }
                },
                "required": ["name"]
            }),
            |args| {
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                
                let content = vec![
                    ToolContent::Text {
                        text: format!("Hello, {}!", name)
                    }
                ];
                
                Ok(ToolCallResult {
                    content,
                    is_error: Some(false)
                })
            }
        )
        .build()?;
    
    info!("Server initialized. Waiting for client connection...");
    
    // Run server (blocks until shutdown)
    server.run().await?;
    
    info!("Server shutting down");
    
    Ok(())
}
