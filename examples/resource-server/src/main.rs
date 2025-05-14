use anyhow::Result;
use modelcontextprotocol_server::mcp_protocol::types::{
    resource::ResourceContent,
    tool::{ToolCallResult, ToolContent},
};
use modelcontextprotocol_server::{transport::StdioTransport, ServerBuilder};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use tracing::{debug, info, Level};
use tracing_subscriber::fmt;

const FILE_CONTENT: &str = r#"
# Sample Markdown File

This is a sample markdown file that can be accessed as a resource.

## Features

- Easy to read
- Contains sample content
- Provides context for language models

## Usage

This file can be used to test the MCP resource functionality.
"#;

const CODE_CONTENT: &str = r#"
fn main() {
    println!("Hello from Rust!");
    
    // This is a sample Rust code file
    // that demonstrates resource access
    let message = "Resources in MCP";
    println!("Testing {}", message);
}
"#;

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
                    .open("resource-server.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    debug!("Logging initialized");

    info!("Starting resource-server MCP server");

    debug!("Initializing server");

    // Create server with stdio transport
    let server = ServerBuilder::new("resource-server", "0.1.0")
        .with_transport(StdioTransport::new())
        // Add a resource - markdown file
        .with_resource(
            "file:///sample/readme.md",
            "README.md",
            Some("Sample markdown file for testing"),
            Some("text/markdown"),
            Some(FILE_CONTENT.len() as u64),
            || {
                Ok(vec![ResourceContent {
                    uri: "file:///sample/readme.md".to_string(),
                    mime_type: "text/markdown".to_string(),
                    text: Some(FILE_CONTENT.to_string()),
                    blob: None,
                }])
            },
        )
        // Add another resource - code file
        .with_resource(
            "file:///sample/main.rs",
            "main.rs",
            Some("Sample Rust code file"),
            Some("text/x-rust"),
            Some(CODE_CONTENT.len() as u64),
            || {
                Ok(vec![ResourceContent {
                    uri: "file:///sample/main.rs".to_string(),
                    mime_type: "text/x-rust".to_string(),
                    text: Some(CODE_CONTENT.to_string()),
                    blob: None,
                }])
            },
        )
        // Add a tool that uses resources
        .with_tool(
            "get_file_contents",
            Some("Retrieve the contents of a file"),
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file"
                    }
                },
                "required": ["path"]
            }),
            |args| {
                debug!("get_file_contents tool called with args: {:?}", args);

                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

                debug!("Fetching file: {}", path);

                let content = match path {
                    "readme.md" => FILE_CONTENT,
                    "main.rs" => CODE_CONTENT,
                    _ => "File not found",
                };

                let content = vec![ToolContent::Text {
                    text: format!("File contents:\n\n{}", content),
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
