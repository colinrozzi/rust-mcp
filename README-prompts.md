# MCP Prompts Feature

This documentation describes the Prompts feature implementation in the Rust MCP project.

## Overview

The Prompts feature in the Model Context Protocol (MCP) allows servers to expose prompt templates to clients. These templates can be discovered, retrieved, and customized with arguments to create structured messages for language models.

## Implementation

The implementation consists of the following components:

1. **Protocol Types**: Defined in `mcp-protocol/src/types/prompt.rs`
2. **Prompt Manager**: Implemented in `mcp-server/src/prompts.rs`
3. **Server Handlers**: Implemented in `mcp-server/src/server_prompts.rs`
4. **Example Server**: Demonstrates prompt registration in `examples/prompt-server`
5. **Example Client**: Shows how to use prompts from a client in `examples/prompt-client`

## Features

The implementation supports:

- Registering prompts with customizable arguments
- Listing available prompts with pagination
- Retrieving prompt content with argument substitution
- Notifications when the available prompts change

## Usage

### Server-side

To register a prompt on your MCP server:

```rust
let server = ServerBuilder::new("my-server", "1.0.0")
    .with_transport(StdioTransport::new())
    .with_prompt(
        "my_prompt",                               // Name
        Some("Description of my prompt"),         // Description
        Some(vec![                                // Arguments
            PromptArgument {
                name: "arg1".to_string(),
                description: Some("First argument".to_string()),
                required: Some(true),
            },
        ]),
        |args| {
            // Generate prompt messages based on arguments
            let arg1 = args.and_then(|a| a.get("arg1").cloned())
                .unwrap_or_default();
            
            // Return prompt messages
            Ok(vec![
                PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent::Text {
                        text: format!("My prompt with {}", arg1),
                    },
                },
            ])
        }
    )
    .build()?;
```

### Client-side

To use prompts from a client:

```rust
// List available prompts
let response = client
    .request(
        methods::PROMPTS_LIST,
        Some(serde_json::to_value(PromptsListParams { cursor: None })?),
    )
    .await?;

// Use a prompt
let mut args = HashMap::new();
args.insert("arg1".to_string(), "value1".to_string());

let params = PromptGetParams {
    name: "my_prompt".to_string(),
    arguments: Some(args),
};

let response = client
    .request(methods::PROMPTS_GET, Some(serde_json::to_value(params)?))
    .await?;

// Process the returned prompt messages
```

## Running the Example

To run the prompt example:

```bash
# Make the script executable
chmod +x run-prompt-example.sh

# Run the example
./run-prompt-example.sh
```

This will build and run both the prompt server and client, demonstrating the full interaction.
