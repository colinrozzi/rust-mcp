use anyhow::Result;
use mcp_protocol::{
    types::{
        resource::{Resource, ResourceTemplate},
        completion::CompletionItem,
        prompt::{Prompt, PromptArgument, PromptMessage, PromptMessageContent},
    }
};
use mcp_server::{ServerBuilder, transport::StdioTransport};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use std::sync::Arc;
use tracing::{info, debug, Level};
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
                .open("completion-server.log")
                .unwrap()))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");
    
    debug!("Logging initialized");
    
    info!("Starting completion-server MCP server");
    
    debug!("Initializing server");
    
    // Create server with stdio transport
    let mut server_builder = ServerBuilder::new("completion-server", "0.1.0")
        .with_transport(StdioTransport::new());
    
    // Create a resource manager
    let resource_manager = Arc::new(mcp_server::resources::ResourceManager::new());
    server_builder = server_builder.with_resource_manager(resource_manager.clone());
    
    // Register a file URI template with completions
    let template = ResourceTemplate {
        uri_template: "file:///{project}/{filename}".to_string(),
        name: "Project Files".to_string(),
        description: Some("Access files within a project".to_string()),
        mime_type: Some("application/octet-stream".to_string()),
        annotations: None,
    };
    
    // Register template with a simple expander
    resource_manager.register_template(template.clone(), |template_uri, params| {
        let mut result = template_uri;
        
        for (name, value) in params {
            result = result.replace(&format!("{{{}}}", name), &value);
        }
        
        Ok(result)
    });
    
    // Register completion provider for the template
    resource_manager.register_completion_provider(&template.uri_template, move |_, param_name, value| {
        debug!("Getting completions for parameter: {} with value: {:?}", param_name, value);
        
        match param_name.as_str() {
            "project" => {
                // Provide a list of example projects
                let projects = vec!["backend", "frontend", "common", "tools", "docs"];
                
                let filtered = if let Some(prefix) = value {
                    projects.into_iter()
                        .filter(|p| p.starts_with(&prefix))
                        .collect::<Vec<&str>>()
                } else {
                    projects
                };
                
                let items = filtered.into_iter()
                    .map(|p| CompletionItem {
                        label: p.to_string(),
                        detail: Some(format!("Project directory: {}", p)),
                        documentation: None,
                    })
                    .collect();
                
                Ok(items)
            },
            "filename" => {
                // Provide some example filenames based on the project
                let filenames = match value {
                    Some(val) if val.starts_with("main") => {
                        vec!["main.rs", "main.go", "main.py", "main.js"]
                    },
                    Some(val) if val.starts_with("config") => {
                        vec!["config.json", "config.yaml", "config.toml"]
                    },
                    Some(val) => {
                        // Filter by prefix
                        vec!["api.rs", "utils.rs", "models.rs", "config.rs", "main.rs"].into_iter()
                            .filter(|f| f.starts_with(&val))
                            .collect()
                    },
                    None => {
                        vec!["api.rs", "utils.rs", "models.rs", "config.rs", "main.rs"]
                    }
                };
                
                let items = filenames.into_iter()
                    .map(|f| CompletionItem {
                        label: f.to_string(),
                        detail: Some(format!("File: {}", f)),
                        documentation: None,
                    })
                    .collect();
                
                Ok(items)
            },
            _ => {
                // Unknown parameter
                Ok(vec![])
            }
        }
    });
    
    // Register a simple resource
    let resource = Resource {
        uri: "file:///backend/main.rs".to_string(),
        name: "Main Entry Point".to_string(),
        description: Some("Backend service entry point".to_string()),
        mime_type: Some("text/x-rust".to_string()),
        annotations: None,
        size: None,
    };
    
    resource_manager.register_resource(resource, || {
        let content = mcp_protocol::types::resource::ResourceContent {
            uri: "file:///backend/main.rs".to_string(),
            mime_type: "text/x-rust".to_string(),
            text: Some("fn main() {\n    println!(\"Hello from the backend!\");\n}".to_string()),
            blob: None,
        };
        Ok(vec![content])
    });
    
    // Also register a prompt with parameters for completion
    let prompt_manager = Arc::new(mcp_server::prompts::PromptManager::new());
    server_builder = server_builder.with_prompt_manager(prompt_manager.clone());
    
    // Create a prompt
    let prompt = Prompt {
        name: "code_review".to_string(),
        description: Some("Review code in a specific programming language".to_string()),
        arguments: Some(vec![
            PromptArgument {
                name: "language".to_string(),
                description: Some("Programming language".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "focus".to_string(),
                description: Some("Review focus area".to_string()),
                required: Some(false),
            }
        ]),
        annotations: None,
    };
    
    // Register the prompt
    prompt_manager.register_prompt(prompt, |params| {
        let language = params.as_ref()
            .and_then(|p| p.get("language"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
            
        let focus = params.as_ref()
            .and_then(|p| p.get("focus"))
            .cloned()
            .unwrap_or_else(|| "general".to_string());
        
        let message = PromptMessage {
            role: "system".to_string(),
            content: PromptMessageContent::Text { 
                text: format!("Reviewing {} code with focus on {}", language, focus)
            },
        };
        
        Ok(vec![message])
    });
    
    // Register language completion provider for the prompt
    prompt_manager.register_completion_provider(
        "code_review",
        "language",
        |_, prefix| {
            let languages = vec![
                "python".to_string(), 
                "javascript".to_string(), 
                "typescript".to_string(), 
                "rust".to_string(), 
                "go".to_string(), 
                "java".to_string(), 
                "c#".to_string(), 
                "c++".to_string(), 
                "php".to_string(), 
                "ruby".to_string(), 
                "kotlin".to_string(), 
                "swift".to_string()
            ];
            
            let filtered_langs = if let Some(prefix) = prefix {
                languages.into_iter()
                    .filter(|lang| lang.starts_with(&prefix.to_lowercase()))
                    .collect()
            } else {
                languages
            };
            
            Ok(filtered_langs)
        }
    );
    
    // Build the server
    let server = server_builder.build()?;
    
    info!("Server initialized. Waiting for client connection...");
    
    debug!("Starting server run loop");
    // Run server (blocks until shutdown)
    server.run().await?;
    
    info!("Server shutting down");
    
    Ok(())
}