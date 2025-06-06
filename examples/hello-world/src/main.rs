use anyhow::Result;
use modelcontextprotocol_server::mcp_protocol::types::tool::{ToolCallResult, ToolContent};
use modelcontextprotocol_server::{transport::StdioTransport, ServerBuilder};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use tracing::{debug, info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to a file

    // Setup subscriber with file writer - no ANSI colors for file
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_writer(move || -> Box<dyn io::Write> {
            Box::new(io::BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("hello-server.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    debug!("Logging initialized");

    info!("Starting hello-world MCP server");

    debug!("Initializing server");

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
                debug!("Hello tool called with args: {:?}", args);

                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");

                debug!("Greeting {}", name);

                let content = vec![ToolContent::Text {
                    text: format!("Hello, {}!", name),
                }];

                Ok(ToolCallResult {
                    content,
                    is_error: Some(false),
                })
            },
        )
        .build()?;

    info!("Server initialized. Waiting for client connection...");

    debug!("Starting server run loop");
    // Run server (blocks until shutdown)
    server.run().await?;

    info!("Server shutting down");

    Ok(())
}
