use anyhow::Result;
use mcp_protocol::{
    types::{
        sampling::{CreateMessageParams, Message, MessageContent},
        tool::{ToolCallResult, ToolContent}
    }
};
use mcp_server::{ServerBuilder, transport::StdioTransport};
use serde_json::json;
use std::fs::OpenOptions;
use std::io;
use tokio::time::sleep;
use tokio::time::Duration;
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
                .open("sampling-server.log")
                .unwrap()))
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
                
                let question = args.get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Tell me about yourself");
                
                // Create content for the tool response
                let content = vec![
                    ToolContent::Text {
                        text: format!("Asking LLM: \"{}\"", question)
                    }
                ];
                
                // Simple response for now, will be replaced with sampling in handle_message
                let result = ToolCallResult {
                    content,
                    is_error: Some(false)
                };
                
                Ok(result)
            }
        )
        .build()?;
    
    // Get a handle to the transport for sampling requests
    let transport = server.transport().clone();
    let state = server.state().clone();
    
    // Spawn a task to simulate sampling requests
    tokio::spawn(async move {
        // Wait for server to be initialized
        while state.load(std::sync::atomic::Ordering::SeqCst) != mcp_protocol::types::ServerState::Ready as u8 {
            sleep(Duration::from_millis(100)).await;
        }
        
        debug!("Server is ready, can now send sampling requests");
        
        // Wait a bit to ensure client is also ready
        sleep(Duration::from_secs(2)).await;
        
        // Create a sampling request
        let params = CreateMessageParams {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: MessageContent::Text {
                        text: "Tell me about the Model Context Protocol".to_string()
                    }
                }
            ],
            model_preferences: None,
            system_prompt: Some("You are a helpful assistant".to_string()),
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            context: None,
        };
        
        // Use the JSON-RPC protocol to send a sampling request
        let request = mcp_protocol::messages::JsonRpcMessage::request(
            json!("sampling-1"),
            mcp_protocol::constants::methods::SAMPLING_CREATE_MESSAGE,
            Some(serde_json::to_value(params).unwrap())
        );
        
        // Send the request to the client
        debug!("Sending sampling request");
        if let Err(e) = transport.send(request).await {
            debug!("Error sending sampling request: {:?}", e);
        }
        
        debug!("Sampling request sent");
    });
    
    info!("Server initialized. Waiting for client connection...");
    
    debug!("Starting server run loop");
    // Run server (blocks until shutdown)
    server.run().await?;
    
    info!("Server shutting down");
    
    Ok(())
}
