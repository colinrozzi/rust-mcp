# Rust MCP Framework

A modular Rust implementation of the Model Context Protocol (MCP), which enables seamless integration between LLM applications and external data sources and tools.

## Protocol Specification

This implementation follows the MCP specification, which can be found at:

- Local path: `/users/colinrozzi/work/mcp-servers/modelcontextprotocol/docs/specification`
- Current implemented version: `/users/colinrozzi/work/mcp-servers/modelcontextprotocol/docs/specification/2024-11-05`

> **Note**: This implementation is currently targeting the 2024-11-05 version of the specification. We plan to implement the newer 2025-03-26 specification in the future, but the inspector currently only supports the 2024 version. All code is designed with forward compatibility in mind.

The specification defines the wire format, message types, capabilities, and features that make up the Model Context Protocol. This library implements the core functionality as described in the specification.

## Project Structure

This repository is organized as a Cargo workspace with three main crates:

1. **mcp-protocol**: Core protocol definitions and types
2. **mcp-client**: Client implementation for connecting to MCP servers
3. **mcp-server**: Server implementation for exposing resources and tools

## Features

- JSON-RPC 2.0 messaging
- Support for stdio transport
- Protocol version negotiation
- Capability negotiation
- Tool registration and execution

## Example Usage

### Server Example

```rust
// Create a simple MCP server with a "hello" tool
let server = ServerBuilder::new("hello-world", "0.1.0")
    .with_transport(StdioTransport::new())
    .with_tool(
        "hello",
        Some("Say hello to someone"),
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the person to greet"
                }
            },
            "required": ["name"]
        }),
        |args| {
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("world");
            
            let content = vec![
                ToolContent::Text {
                    text: format!("Hello, {}!", name)
                }
            ];
            
            Ok(ToolCallResult {
                content,
                is_error: Some(false)
            })
        }
    )
    .build()?;

// Run the server (blocks until shutdown)
server.run().await?;
```

### Client Example

```rust
// Create a client that connects to an MCP server
let (transport, receiver) = StdioTransport::new("path/to/server", vec![]);

let client = ClientBuilder::new("my-client", "0.1.0")
    .with_transport(transport)
    .build()?;

// Start message handling
let client_clone = client.clone();
tokio::spawn(async move {
    while let Ok(message) = receiver.recv().await {
        if let Err(e) = client_clone.handle_message(message).await {
            eprintln!("Error handling message: {}", e);
        }
    }
});

// Initialize the client
let init_result = client.initialize().await?;
println!("Connected to: {} v{}", init_result.server_info.name, init_result.server_info.version);

// List available tools
let tools = client.list_tools().await?;
for tool in &tools.tools {
    println!("Tool: {} - {}", tool.name, tool.description.as_deref().unwrap_or(""));
}

// Call a tool
let result = client.call_tool("hello", &json!({
    "name": "MCP User"
})).await?;

// Process the result
for content in result.content {
    match content {
        ToolContent::Text { text } => {
            println!("{}", text);
        },
        _ => {
            println!("Received non-text content");
        }
    }
}
```

## Examples

The repository includes working examples:

1. **hello-world**: A simple MCP server that provides a "hello" tool
2. **simple-client**: A client that connects to the hello-world server

To run the examples:

```bash
# First, build and run the server
cargo run --package hello-world

# In another terminal, run the client
cargo run --package simple-client
```

## Getting Started

1. Add the crates to your `Cargo.toml`:

```toml
[dependencies]
mcp-protocol = { git = "https://github.com/your-username/rust-mcp" }
mcp-client = { git = "https://github.com/your-username/rust-mcp" }
mcp-server = { git = "https://github.com/your-username/rust-mcp" }
```

2. Build your MCP server or client using the provided builder patterns.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
