use anyhow::Result;
use modelcontextprotocol_client::mcp_protocol::{
    constants::methods,
    types::{
        prompt::{PromptGetParams, PromptsListParams},
        ClientInfo,
    },
};
use modelcontextprotocol_client::{transport::StdioTransport, ClientBuilder};
use std::collections::HashMap;
use tracing::{info, Level};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set default subscriber");

    // Create client info
    let _client_info = ClientInfo {
        name: "prompt-client-example".to_string(),
        version: "0.1.0".to_string(),
    };

    // Create transport
    let (transport, _rx) = StdioTransport::new(
        "cargo",
        vec![
            "run".to_string(),
            "--package".to_string(),
            "prompt-server".to_string(),
        ],
    );

    // Create client using the builder
    let client = ClientBuilder::new("prompt-client-example", "0.1.0")
        .with_transport(transport)
        .build()?;

    // Connect to server
    info!("Connecting to server...");
    client.initialize().await?;
    info!("Connected to server");

    // List available prompts
    info!("Listing available prompts...");
    let id = client.next_request_id().await?;
    let response = client
        .send_request(
            methods::PROMPTS_LIST,
            Some(serde_json::to_value(PromptsListParams { cursor: None })?),
            id.to_string(),
        )
        .await?;

    let prompts_list: serde_json::Value = match response {
        modelcontextprotocol_client::mcp_protocol::messages::JsonRpcMessage::Response {
            result,
            ..
        } => {
            if let Some(result) = result {
                serde_json::from_value(result)?
            } else {
                serde_json::Value::Null
            }
        }
        _ => serde_json::Value::Null,
    };

    if prompts_list.is_null() {
        info!("No prompts available");
        return Ok(());
    }

    info!(
        "Available prompts: {}",
        serde_json::to_string_pretty(&prompts_list)?
    );

    // Get prompts array from the response
    let prompts = prompts_list["prompts"].as_array().unwrap();

    // Use code review prompt
    if let Some(_prompt) = prompts
        .iter()
        .find(|p| p["name"].as_str().unwrap() == "code_review")
    {
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
        let id = client.next_request_id().await?;
        let response = client
            .send_request(
                methods::PROMPTS_GET,
                Some(serde_json::to_value(params)?),
                id.to_string(),
            )
            .await?;

        let prompt_content: serde_json::Value = match response {
            modelcontextprotocol_client::mcp_protocol::messages::JsonRpcMessage::Response {
                result,
                ..
            } => {
                if let Some(result) = result {
                    serde_json::from_value(result)?
                } else {
                    serde_json::Value::Null
                }
            }
            _ => serde_json::Value::Null,
        };

        info!(
            "Code review prompt content: {}",
            serde_json::to_string_pretty(&prompt_content)?
        );
    }

    // Use translate prompt
    if let Some(_prompt) = prompts
        .iter()
        .find(|p| p["name"].as_str().unwrap() == "translate")
    {
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
        let id = client.next_request_id().await?;
        let response = client
            .send_request(
                methods::PROMPTS_GET,
                Some(serde_json::to_value(params)?),
                id.to_string(),
            )
            .await?;

        let prompt_content: serde_json::Value = match response {
            modelcontextprotocol_client::mcp_protocol::messages::JsonRpcMessage::Response {
                result,
                ..
            } => {
                if let Some(result) = result {
                    serde_json::from_value(result)?
                } else {
                    serde_json::Value::Null
                }
            }
            _ => serde_json::Value::Null,
        };

        info!(
            "Translation prompt content: {}",
            serde_json::to_string_pretty(&prompt_content)?
        );
    }

    // Shutdown client
    info!("Shutting down client...");
    client.shutdown().await?;
    info!("Client shut down");

    Ok(())
}
