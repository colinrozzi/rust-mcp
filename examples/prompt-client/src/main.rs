use anyhow::Result;
use mcp_client::{transport::StdioTransport, Client};
use mcp_protocol::{
    constants::{methods, PROTOCOL_VERSION},
    types::{ClientInfo, prompt::{PromptGetParams, PromptsListParams}},
};
use std::collections::HashMap;
use tracing::{debug, info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default subscriber");

    // Create client info
    let client_info = ClientInfo {
        name: "prompt-client-example".to_string(),
        version: "0.1.0".to_string(),
    };

    // Create transport
    let transport = StdioTransport::new();

    // Create client
    let mut client = Client::new(transport, client_info, PROTOCOL_VERSION.to_string())?;

    // Connect to server
    info!("Connecting to server...");
    client.initialize().await?;
    info!("Connected to server");

    // List available prompts
    info!("Listing available prompts...");
    let response = client
        .request(
            methods::PROMPTS_LIST,
            Some(serde_json::to_value(PromptsListParams { cursor: None })?),
        )
        .await?;
    
    let prompts_list: serde_json::Value = serde_json::from_value(response)?;
    info!("Available prompts: {}", serde_json::to_string_pretty(&prompts_list)?);
    
    // Get prompts array from the response
    let prompts = prompts_list["prompts"].as_array().unwrap();
    
    // Use code review prompt
    if let Some(prompt) = prompts.iter().find(|p| p["name"].as_str().unwrap() == "code_review") {
        info!("Using code review prompt...");
        
        // Sample code to review
        let code = r#"
fn factorial(n: u64) -> u64 {
    if n == 0 {
        return 1;
    }
    n * factorial(n - 1)
}
        "#;
        
        // Create arguments
        let mut args = HashMap::new();
        args.insert("code".to_string(), code.to_string());
        args.insert("language".to_string(), "Rust".to_string());
        
        let params = PromptGetParams {
            name: "code_review".to_string(),
            arguments: Some(args),
        };
        
        // Get prompt content
        let response = client
            .request(methods::PROMPTS_GET, Some(serde_json::to_value(params)?))
            .await?;
        
        let prompt_content: serde_json::Value = serde_json::from_value(response)?;
        info!("Code review prompt content: {}", serde_json::to_string_pretty(&prompt_content)?);
    }
    
    // Use translate prompt
    if let Some(prompt) = prompts.iter().find(|p| p["name"].as_str().unwrap() == "translate") {
        info!("Using translate prompt...");
        
        // Text to translate
        let text = "Hello, world! How are you today?";
        
        // Create arguments
        let mut args = HashMap::new();
        args.insert("text".to_string(), text.to_string());
        args.insert("source_language".to_string(), "English".to_string());
        args.insert("target_language".to_string(), "Spanish".to_string());
        
        let params = PromptGetParams {
            name: "translate".to_string(),
            arguments: Some(args),
        };
        
        // Get prompt content
        let response = client
            .request(methods::PROMPTS_GET, Some(serde_json::to_value(params)?))
            .await?;
        
        let prompt_content: serde_json::Value = serde_json::from_value(response)?;
        info!("Translation prompt content: {}", serde_json::to_string_pretty(&prompt_content)?);
    }
    
    // Shutdown client
    info!("Shutting down client...");
    client.shutdown().await?;
    info!("Client shut down");
    
    Ok(())
}
