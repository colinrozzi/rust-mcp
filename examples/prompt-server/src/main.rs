use anyhow::Result;
use modelcontextprotocol_server::mcp_protocol::types::prompt::{
    PromptArgument, PromptMessage, PromptMessageContent,
};
use modelcontextprotocol_server::{transport::StdioTransport, ServerBuilder};
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
                    .open("prompt-server.log")
                    .unwrap(),
            ))
        })
        .with_ansi(true) // Enable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set default tracing subscriber");

    debug!("Logging initialized");

    info!("Starting prompt-server MCP server");

    debug!("Initializing server");

    // Create server with stdio transport
    let server = ServerBuilder::new("prompt-server", "0.1.0")
        .with_transport(StdioTransport::new())
        // Add a code review prompt
        .with_prompt(
            "code_review",
            Some("Asks the AI model to analyze code quality and suggest improvements"),
            Some(vec![
                PromptArgument {
                    name: "code".to_string(),
                    description: Some("The code to review".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "language".to_string(),
                    description: Some("The programming language of the code".to_string()),
                    required: Some(false),
                },
            ]),
            |args| {
                debug!("Code review prompt called with args: {:?}", args);

                // Get code from arguments
                let code = if let Some(args) = &args {
                    args.get("code")
                        .cloned()
                        .unwrap_or_else(|| "// No code provided".to_string())
                } else {
                    "// No code provided".to_string()
                };

                // Get language from arguments (optional)
                let language = if let Some(args) = &args {
                    args.get("language")
                        .cloned()
                        .unwrap_or_else(|| "the provided".to_string())
                } else {
                    "the provided".to_string()
                };

                // Create prompt messages
                let messages = vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent::Text {
                        text: format!(
                            "Please review {} code and suggest improvements:\n\n```\n{}\n```",
                            language, code
                        ),
                    },
                }];

                Ok(messages)
            },
        )
        // Add a language translation prompt
        .with_prompt(
            "translate",
            Some("Translates text from one language to another"),
            Some(vec![
                PromptArgument {
                    name: "text".to_string(),
                    description: Some("The text to translate".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "source_language".to_string(),
                    description: Some("The source language".to_string()),
                    required: Some(false),
                },
                PromptArgument {
                    name: "target_language".to_string(),
                    description: Some("The target language".to_string()),
                    required: Some(true),
                },
            ]),
            |args| {
                debug!("Translation prompt called with args: {:?}", args);

                if args.is_none() {
                    return Err(anyhow::anyhow!("Missing required arguments"));
                }

                let args = args.unwrap();

                // Get required arguments
                let text = args
                    .get("text")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' argument"))?;

                let target_language = args
                    .get("target_language")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'target_language' argument"))?;

                // Get optional source language
                let source_language = args
                    .get("source_language")
                    .cloned()
                    .unwrap_or_else(|| "the source language".to_string());

                // Create prompt messages
                let messages = vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent::Text {
                        text: format!(
                            "Translate the following text from {} to {}:\n\n{}",
                            source_language, target_language, text
                        ),
                    },
                }];

                Ok(messages)
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
