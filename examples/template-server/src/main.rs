use anyhow::Result;
use modelcontextprotocol_server::mcp_protocol::types::{
    completion::CompletionItem,
    resource::ResourceContent,
    tool::{ToolCallResult, ToolContent},
};
use modelcontextprotocol_server::{transport::StdioTransport, ServerBuilder};
use serde_json::json;
use std::collections::HashMap;
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
                    .open("template-server.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    info!("Starting template-server MCP server");

    // Create server with stdio transport
    let server = ServerBuilder::new("template-server", "0.1.0")
        .with_transport(StdioTransport::new())
        // Add a simple resource
        .with_resource(
            "file:///project/sample.txt",
            "Sample File",
            Some("A sample text file"),
            Some("text/plain"),
            None,
            || {
                let content = ResourceContent {
                    uri: "file:///project/sample.txt".to_string(),
                    mime_type: "text/plain".to_string(),
                    text: Some("This is a sample text file.".to_string()),
                    blob: None,
                };
                Ok(vec![content])
            },
        )
        // Add a file template
        .with_template(
            "file:///{path}",
            "File Access",
            Some("Access a file by path"),
            Some("application/octet-stream"),
            |_template, params| {
                if let Some(path) = params.get("path") {
                    Ok(format!("file:///{}", path))
                } else {
                    Err(anyhow::anyhow!("Missing path parameter"))
                }
            },
        )
        // Add a database record template
        .with_template(
            "db:///{database}/{table}/{id}",
            "Database Record",
            Some("Access a database record by ID"),
            Some("application/json"),
            |_template, params| {
                let database = params.get("database").cloned().unwrap_or_default();
                let table = params.get("table").cloned().unwrap_or_default();
                let id = params.get("id").cloned().unwrap_or_default();

                Ok(format!("db:///{}/{}/{}", database, table, id))
            },
        )
        // Add a completion provider for the database template
        .with_template_completion(
            "db:///{database}/{table}/{id}",
            |_template, param_name, _value| {
                debug!("Completion requested for parameter: {}", param_name);

                match param_name.as_str() {
                    "database" => {
                        // Return a list of available databases
                        let items = vec![
                            CompletionItem {
                                label: "customers".to_string(),
                                detail: Some("Customer database".to_string()),
                                documentation: None,
                            },
                            CompletionItem {
                                label: "products".to_string(),
                                detail: Some("Product database".to_string()),
                                documentation: None,
                            },
                            CompletionItem {
                                label: "orders".to_string(),
                                detail: Some("Order database".to_string()),
                                documentation: None,
                            },
                        ];
                        Ok(items)
                    }
                    "table" => {
                        // Return tables based on database
                        let items = vec![
                            CompletionItem {
                                label: "users".to_string(),
                                detail: Some("Users table".to_string()),
                                documentation: None,
                            },
                            CompletionItem {
                                label: "accounts".to_string(),
                                detail: Some("Accounts table".to_string()),
                                documentation: None,
                            },
                        ];
                        Ok(items)
                    }
                    "id" => {
                        // Return some example IDs
                        let items = vec![
                            CompletionItem {
                                label: "1001".to_string(),
                                detail: Some("User ID 1001".to_string()),
                                documentation: None,
                            },
                            CompletionItem {
                                label: "1002".to_string(),
                                detail: Some("User ID 1002".to_string()),
                                documentation: None,
                            },
                        ];
                        Ok(items)
                    }
                    _ => Ok(vec![]),
                }
            },
        )
        // Add a tool to expand templates
        .with_tool(
            "expand-template",
            Some("Expand a URI template with parameters"),
            json!({
                "type": "object",
                "properties": {
                    "template": {
                        "type": "string",
                        "description": "URI template to expand"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "Parameters to use for expansion"
                    }
                },
                "required": ["template", "parameters"]
            }),
            |args| {
                debug!("Expand template tool called with args: {:?}", args);

                let template = args
                    .get("template")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing template parameter"))?;

                let parameters = args
                    .get("parameters")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameters object"))?;

                // Convert parameters to HashMap<String, String>
                let mut param_map = HashMap::new();
                for (key, value) in parameters {
                    if let Some(value_str) = value.as_str() {
                        param_map.insert(key.clone(), value_str.to_string());
                    }
                }

                // Simple expansion logic (a real implementation would use the resource manager)
                let mut result = template.to_string();
                for (key, value) in param_map {
                    result = result.replace(&format!("{{{}}}", key), &value);
                }

                debug!("Expanded template: {}", result);

                let content = vec![ToolContent::Text {
                    text: format!("Expanded URI: {}", result),
                }];

                Ok(ToolCallResult {
                    content,
                    is_error: Some(false),
                })
            },
        )
        .build()?;

    info!("Server initialized. Waiting for client connection...");

    // Run server (blocks until shutdown)
    server.run().await?;

    info!("Server shutting down");

    Ok(())
}
