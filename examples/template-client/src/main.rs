use anyhow::Result;
use std::sync::Arc;
use mcp_client::{ClientBuilder, transport::StdioTransport};
use mcp_protocol::types::{
    resource::ResourceTemplatesListParams,
    tool::ToolContent,
};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to a file
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(move || -> Box<dyn io::Write> {
            Box::new(io::BufWriter::new(OpenOptions::new()
                .create(true)
                .append(true)
                .open("template-client.log")
                .unwrap()))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");
    
    info!("Starting template client");
    
    // Path to the server executable
    let server_path = "../../target/debug/template-server";
    
    // Create and connect to server
    let (transport, mut receiver) = StdioTransport::new(server_path, vec![]);
    
    let client = Arc::new(ClientBuilder::new("template-client", "0.1.0")
        .with_transport(transport)
        .build()?);
    
    // Start message handling
    let client_for_handler = client.clone();
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = client_for_handler.handle_message(message).await {
                let err_msg = format!("Error handling message: {}", e);
                tracing::error!("{}", err_msg);
            }
        }
    });
    
    // Initialize the client
    info!("Initializing connection to server");
    let init_result = client.initialize().await?;
    info!("Connected to: {} v{}", init_result.server_info.name, init_result.server_info.version);
    
    // List resources
    info!("Listing resources");
    let resources_result = client.send_request(
        "resources/list", 
        None, 
        "list_resources".to_string()
    ).await?;
    
    if let mcp_protocol::messages::JsonRpcMessage::Response { result, .. } = resources_result {
        if let Some(result) = result {
            info!("Resources: {}", result);
        }
    }
    
    // List templates
    info!("Listing resource templates");
    let templates_params = ResourceTemplatesListParams {
        cursor: None,
    };
    
    let templates_result = client.send_request(
        "resources/templates/list",
        Some(json!(templates_params)),
        "list_templates".to_string()
    ).await?;
    
    if let mcp_protocol::messages::JsonRpcMessage::Response { result, .. } = templates_result {
        if let Some(result) = result {
            info!("Templates: {}", result);
        }
    }
    
    // Get completions for a template parameter
    info!("Requesting completions for database parameter");
    let completion_params = mcp_protocol::types::completion::CompletionCompleteParams {
        r#ref: mcp_protocol::types::completion::CompletionReference::Resource {
            uri: "db:///{database}/{table}/{id}".to_string(),
        },
        argument: mcp_protocol::types::completion::CompletionArgument {
            name: "database".to_string(),
            value: None,
        },
    };
    
    let completion_result = client.send_request(
        "completion/complete",
        Some(json!(completion_params)),
        "get_completions".to_string()
    ).await?;
    
    if let mcp_protocol::messages::JsonRpcMessage::Response { result, .. } = completion_result {
        if let Some(result) = result {
            info!("Completions: {}", result);
        }
    }
    
    // Call the expand-template tool
    info!("Calling expand-template tool");
    let template_expansion = client.call_tool(
        "expand-template", 
        &json!({
            "template": "db:///{database}/{table}/{id}",
            "parameters": {
                "database": "customers",
                "table": "users",
                "id": "1001"
            }
        })
    ).await?;
    
    info!("Tool result:");
    for content in template_expansion.content {
        match content {
            ToolContent::Text { text } => {
                info!("{}", text);
            },
            _ => {
                info!("Received non-text content");
            }
        }
    }
    
    // Shutdown
    info!("Shutting down client");
    client.shutdown().await?;
    
    info!("Client finished successfully");
    
    Ok(())
}
