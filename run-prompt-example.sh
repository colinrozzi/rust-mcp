#!/bin/bash
# Make this script executable with: chmod +x run-prompt-example.sh

# Build the prompt server and client
echo "Building prompt server and client..."
cargo build --package prompt-server --package prompt-client

# Run the prompt server and client in a pipeline
echo "Running prompt server with client..."
echo "Output will be in prompt-server.log and the terminal"
cargo run --package prompt-server | cargo run --package prompt-client

echo "Test completed. Check prompt-server.log for server logs."
