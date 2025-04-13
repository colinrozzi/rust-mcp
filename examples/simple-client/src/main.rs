use std::sync::Arc;
use anyhow::Result;
use mcp_client::{ClientBuilder, transport::StdioTransport};
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
    
    info!("Starting simple MCP client");
    
    // Path to the server executable
    let server_path = "target/debug/hello-world";
    
    // Create and connect to server
    let (transport, mut receiver) = StdioTransport::new(server_path, vec![]);
    
    let client = Arc::new(ClientBuilder::new("simple-client", "0.1.0")
        .with_transport(transport)
        .build()?);
    
    // Start message handling
    let client_for_handler = client.clone();
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = client_for_handler.handle_message(message).await {
                eprintln!("Error handling message: {}", e);
            }
        }
    });
    
    // Initialize the client
    info!("Initializing connection to server");
    let init_result = client.initialize().await?;
    info!("Connected to: {} v{}", init_result.server_info.name, init_result.server_info.version);
    
    // List available tools
    info!("Requesting available tools");
    let tools = client.list_tools().await?;
    info!("Available tools: {}", tools.tools.len());
    for tool in &tools.tools {
        info!("Tool: {} - {}", tool.name, tool.description.as_deref().unwrap_or(""));
    }
    
    // Call the hello tool
    if tools.tools.iter().any(|t| t.name == "hello") {
        info!("Calling 'hello' tool");
        let result = client.call_tool("hello", &json!({
            "name": "MCP User"
        })).await?;
        
        // Display the result
        info!("Tool result:");
        for content in result.content {
            match content {
                mcp_protocol::types::tool::ToolContent::Text { text } => {
                    info!("{}", text);
                },
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
