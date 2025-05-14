use anyhow::Result;
use modelcontextprotocol_client::{transport::StdioTransport, ClientBuilder};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to a file

    // Setup subscriber with file writer - no ANSI colors for file
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(move || -> Box<dyn io::Write> {
            Box::new(io::BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("simple-client.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    info!("Starting simple MCP client");

    // Path to the server executable
    let server_path = "../../target/debug/hello-world";

    // Create and connect to server
    let (transport, mut receiver) = StdioTransport::new(server_path, vec![]);

    let client = Arc::new(
        ClientBuilder::new("simple-client", "0.1.0")
            .with_transport(transport)
            .build()?,
    );

    // Start message handling
    let client_for_handler = client.clone();
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = client_for_handler.handle_message(message).await {
                // Log errors to a separate writer to avoid stdout
                let err_msg = format!("Error handling message: {}", e);
                tracing::error!("{}", err_msg);
            }
        }
    });

    // Initialize the client
    info!("Initializing connection to server");
    let init_result = client.initialize().await?;
    info!(
        "Connected to: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );

    // List available tools
    info!("Requesting available tools");
    let tools = client.list_tools().await?;
    info!("Available tools: {}", tools.tools.len());
    for tool in &tools.tools {
        info!(
            "Tool: {} - {}",
            tool.name,
            tool.description.as_deref().unwrap_or("")
        );
    }

    // Call the hello tool
    if tools.tools.iter().any(|t| t.name == "hello") {
        info!("Calling 'hello' tool");
        let result = client
            .call_tool(
                "hello",
                &json!({
                    "name": "MCP User"
                }),
            )
            .await?;

        // Display the result
        info!("Tool result:");
        for content in result.content {
            match content {
                modelcontextprotocol_client::mcp_protocol::types::tool::ToolContent::Text {
                    text,
                } => {
                    info!("{}", text);
                }
                _ => {
                    info!("Received non-text content");
                }
            }
        }
    } else {
        info!("'hello' tool not available");
    }

    // Shutdown
    info!("Shutting down client");
    client.shutdown().await?;

    Ok(())
}
