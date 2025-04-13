# MCP Prompts Feature

This documentation describes the Prompts feature implementation in the Rust MCP (Model Context Protocol) project.

## Overview

The Prompts feature in the Model Context Protocol (MCP) allows servers to expose prompt templates to clients. These templates can be discovered, retrieved, and customized with arguments to create structured messages for language models.

## Implementation

The implementation consists of the following components:

1. **Protocol Types**: Defined in `mcp-protocol/src/types/prompt.rs`
2. **Prompt Manager**: Implemented in `mcp-server/src/prompts.rs`
3. **Server Handlers**: Implemented in `mcp-server/src/server_prompts.rs`
4. **Example Server**: Demonstrates prompt registration in `examples/prompt-server`
5. **Example Client**: Shows how to use prompts from a client in `examples/prompt-client`

## Key Features

- **Arguments Validation**: Prompt arguments are validated for required fields and unexpected values
- **Content Types Support**: Messages can contain text, images, or embedded resources
- **Annotations**: Prompts can have metadata associated with them via annotations
- **Pagination**: Large sets of prompts can be paginated for efficient retrieval
- **Update Notifications**: Clients can be notified when the prompt list changes
- **Completion Integration**: Argument values can be suggested through the completion API

## Usage

### Server-side

To register a prompt on your MCP server:

```rust
// Create a server builder
let server = ServerBuilder::new("my-server", "1.0.0")
    .with_transport(StdioTransport::new())
    // Register a prompt
    .with_prompt(
        "code_review",                        // Name
        Some("Reviews code for issues"),      // Description
        Some(vec![                            // Arguments
            PromptArgument {
                name: "code".to_string(),
                description: Some("Code to review".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "language".to_string(),
                description: Some("Programming language".to_string()),
                required: Some(false),
            },
        ]),
        |arguments| {
            // Generate messages based on arguments
            let code = if let Some(args) = &arguments {
                args.get("code")
                    .cloned()
                    .unwrap_or_default()
            } else {
                "// No code provided".to_string()
            };
            
            // Create the prompt message
            let message = PromptMessage {
                role: "user".to_string(),
                content: PromptMessageContent::Text {
                    text: format!("Please review this code:\n\n```\n{}\n```", code),
                },
            };
            
            Ok(vec![message])
        }
    )
    .build()?;
```

### Client-side

To use prompts from a client:

```rust
// List available prompts
let id = client.next_request_id().await?;
let response = client
    .send_request(
        methods::PROMPTS_LIST,
        Some(serde_json::to_value(PromptsListParams { cursor: None })?),
        id.to_string(),
    )
    .await?;

// Extract prompts from response
let prompts_list: serde_json::Value = match response {
    JsonRpcMessage::Response { result, .. } => {
        if let Some(result) = result {
            serde_json::from_value(result)?
        } else {
            return Err(anyhow!("Invalid response"));
        }
    }
    _ => return Err(anyhow!("Invalid response type")),
};

// Use a specific prompt
let name = "code_review";
let mut args = HashMap::new();
args.insert("code".to_string(), "fn main() { println!(\"Hello\"); }".to_string());
args.insert("language".to_string(), "Rust".to_string());

let params = PromptGetParams {
    name: name.to_string(),
    arguments: Some(args),
};

let id = client.next_request_id().await?;
let response = client
    .send_request(methods::PROMPTS_GET, Some(serde_json::to_value(params)?), id.to_string())
    .await?;

// Process the prompt result
// ...
```

## Advanced Features

### Annotations

Annotations allow attaching metadata to prompts:

```rust
// Add an annotation to a prompt
prompt_manager.add_annotation(
    "code_review", 
    "version", 
    serde_json::json!("1.0.0")
).await?;

// Retrieve an annotation
let version = prompt_manager.get_annotation(
    "code_review", 
    "version"
).await?;
```

### Embedded Resources

Prompt messages can include embedded resources:

```rust
// Create a message with an embedded resource
PromptMessage {
    role: "user".to_string(),
    content: PromptMessageContent::Resource {
        resource: EmbeddedResource {
            uri: "resource://example".to_string(),
            mime_type: "text/plain".to_string(),
            text: Some("Resource content".to_string()),
            data: None,
        }
    },
}
```

### Image Content

Prompt messages can include images:

```rust
// Create a message with an image
PromptMessage {
    role: "user".to_string(),
    content: PromptMessageContent::Image {
        data: base64_encoded_image_data,
        mime_type: "image/png".to_string(),
    },
}
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
