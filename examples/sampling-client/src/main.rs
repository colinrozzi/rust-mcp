use anyhow::Result;
use mcp_client::{ClientBuilder, transport::StdioTransport};
use mcp_protocol::types::sampling::{CreateMessageParams, CreateMessageResult, Message, MessageContent};
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default subscriber");

    // Create client using the builder
    let mut client = ClientBuilder::new("sampling-client-example", "0.1.0")
        .with_sampling() // Enable sampling capability
        .with_transport(StdioTransport::new())
        .build()?;

    // Register sampling callback
    client.register_sampling_callback(Box::new(|params| {
        info!("Received sampling request: {:?}", params);
        
        // In a real implementation, this would call an actual LLM API
        // For this example, we just echo back the user's message
        
        let mut response_text = String::from("You said: ");
        
        // Find the last user message content
        for message in &params.messages {
            if message.role == "user" {
                if let MessageContent::Text { text } = &message.content {
                    response_text.push_str(text);
                    break;
                }
            }
        }
        
        // Create response
        let result = CreateMessageResult {
            role: "assistant".to_string(),
            content: MessageContent::Text {
                text: response_text
            },
            model: Some("echo-model-1.0".to_string()),
            stop_reason: Some("content_length".to_string()),
            metadata: None,
        };
        
        Ok(result)
    })).await?;

    // Connect to server
    info!("Connecting to server...");
    client.initialize().await?;
    info!("Connected to server");

    // Wait for requests from the server
    info!("Waiting for sampling requests...");
    
    // In a real implementation, we would have a proper message loop
    // For this example, we'll just sleep and wait for requests
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
