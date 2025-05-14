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
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_writer(move || -> Box<dyn io::Write> {
            Box::new(io::BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("sampling-server.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    debug!("Logging initialized");

    info!("Starting sampling-server MCP server");

    debug!("Initializing server");

    // Create server with stdio transport
    let server = ServerBuilder::new("sampling-server", "0.1.0")
        .with_transport(StdioTransport::new())
        // Add a tool that uses sampling
        .with_tool(
            "ask-llm",
            Some("Asks an LLM for information"),
            json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the LLM"
                    }
                },
                "required": ["question"]
            }),
            |args| {
                debug!("Ask LLM tool called with args: {:?}", args);

                let question = args
                    .get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Tell me about yourself");

                // Since we can't directly access the transport from here,
                // we'll just return a simple response
                let content = vec![
                    ToolContent::Text {
                        text: format!("Question: {}", question),
                    },
                    ToolContent::Text {
                        text: "This tool would normally use sampling to get an answer from an LLM."
                            .to_string(),
                    },
                ];

                let result = ToolCallResult {
                    content,
                    is_error: Some(false),
                };

                Ok(result)
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
