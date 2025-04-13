use std::sync::Arc;
use anyhow::Result;
use mcp_client::{ClientBuilder, transport::StdioTransport};
use mcp_protocol::types::completion::{CompletionReference, CompletionArgument, CompleteRequest};
use tracing::{info, Level};
use std::fs::OpenOptions;
use std::io;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to a file
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_writer(move || -> Box<dyn io::Write> {
            Box::new(io::BufWriter::new(OpenOptions::new()
                .create(true)
                .append(true)
                .open("completion-client.log")
                .unwrap()))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");
    
    info!("Starting completion client");
    
    // Path to the server executable
    let server_path = "../../target/debug/completion-server";
    
    // Create and connect to server
    let (transport, mut receiver) = StdioTransport::new(server_path, vec![]);
    
    let client = Arc::new(ClientBuilder::new("completion-client", "0.1.0")
        .with_transport(transport)
        .build()?);
    
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
    info!("Connected to: {} v{}", init_result.server_info.name, init_result.server_info.version);
    
    // Get available resource templates
    info!("Requesting resource templates");
    let templates_result = client.list_resource_templates().await?;
    if templates_result.resource_templates.is_empty() {
        info!("No resource templates available");
    } else {
        info!("Available resource templates:");
        for template in &templates_result.resource_templates {
            info!("Template: {} - {}", template.uri_template, template.name);
        }
    }
    
    // Try completion for the file template
    let template_uri = "file:///{project}/{filename}";
    
    // First, try completion for the 'project' parameter
    info!("Requesting completion for 'project' parameter");
    let project_completion_request = CompleteRequest {
        r#ref: CompletionReference::Resource {
            uri: template_uri.to_string(),
        },
        argument: CompletionArgument {
            name: "project".to_string(),
            value: "b".to_string(),  // Should match 'backend'
        },
    };
    
    let project_completion = client.complete(project_completion_request).await?;
    info!("Project completion results:");
    for value in &project_completion.completion.values {
        info!("  - {}", value);
    }
    
    // Then, try completion for the 'filename' parameter
    info!("Requesting completion for 'filename' parameter");
    let filename_completion_request = CompleteRequest {
        r#ref: CompletionReference::Resource {
            uri: template_uri.to_string(),
        },
        argument: CompletionArgument {
            name: "filename".to_string(),
            value: "m".to_string(),  // Should match 'main.rs' etc.
        },
    };
    
    let filename_completion = client.complete(filename_completion_request).await?;
    info!("Filename completion results:");
    for value in &filename_completion.completion.values {
        info!("  - {}", value);
    }
    
    // Now try completion for a prompt parameter
    info!("Requesting completion for 'language' parameter in prompt");
    let prompt_completion_request = CompleteRequest {
        r#ref: CompletionReference::Prompt {
            name: "code_review".to_string(),
        },
        argument: CompletionArgument {
            name: "language".to_string(),
            value: "py".to_string(),
        },
    };
    
    let prompt_completion = client.complete(prompt_completion_request).await?;
    info!("Prompt parameter completion results:");
    if prompt_completion.completion.values.is_empty() {
        info!("  No completion values returned");
    } else {
        for value in &prompt_completion.completion.values {
            info!("  - {}", value);
        }
    }
    
    // Shutdown
    info!("Shutting down client");
    client.shutdown().await?;
    
    Ok(())
}