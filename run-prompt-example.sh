#!/bin/bash
# Make this script executable with: chmod +x run-prompt-example.sh

# Build the prompt server and client
echo "Building prompt server and client..."
cargo build --package prompt-server --package prompt-client

# Run the prompt server in the background (with a timeout)
echo "Starting prompt server in the background..."
cargo run --package prompt-server > prompt-server.log 2>&1 &
SERVER_PID=$!

# Give the server a moment to start up
sleep 2

# Run the prompt client
echo "Running prompt client..."
cargo run --package prompt-client

# Clean up the server process
kill $SERVER_PID 2>/dev/null

echo "Test completed. Check prompt-server.log for server logs."
