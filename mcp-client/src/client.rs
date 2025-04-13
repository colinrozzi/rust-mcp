// mcp-client/src/client.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

use mcp_protocol::{
    constants::{methods, PROTOCOL_VERSION},
    messages::{InitializeParams, InitializeResult, JsonRpcMessage},
    types::{
        tool::{ToolCallParams, ToolCallResult, ToolsListResult},
        ClientInfo,
    },
};

use crate::transport::Transport;

/// MCP client state
#[derive(Debug, Clone, PartialEq)]
enum ClientState {
    Created,
    Initializing,
    Ready,
    ShuttingDown,
}

/// Represents a pending request waiting for a response
struct PendingRequest {
    response_tx: mpsc::Sender<Result<JsonRpcMessage>>,
}

/// MCP client builder
pub struct ClientBuilder {
    name: String,
    version: String,
    transport: Option<Box<dyn Transport>>,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            transport: None,
        }
    }

    /// Set the transport to use
    pub fn with_transport<T: Transport>(mut self, transport: T) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    /// Build the client
    pub fn build(self) -> Result<Client> {
        let transport = self
            .transport
            .ok_or_else(|| anyhow!("Transport is required"))?;

        Ok(Client {
            name: self.name,
            version: self.version,
            transport,
            state: Arc::new(RwLock::new(ClientState::Created)),
            next_id: Arc::new(Mutex::new(1)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            initialized_result: Arc::new(RwLock::new(None)),
        })
    }
}

/// MCP client
pub struct Client {
    name: String,
    version: String,
    transport: Box<dyn Transport>,
    state: Arc<RwLock<ClientState>>,
    next_id: Arc<Mutex<i64>>,
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    initialized_result: Arc<RwLock<Option<InitializeResult>>>,
}

impl Client {
    /// Initialize the client
    pub async fn initialize(&self) -> Result<InitializeResult> {
        // Check if we're already initialized
        {
            let state = self.state.read().await;
            if *state != ClientState::Created {
                return Err(anyhow!("Client already initialized"));
            }
        }

        // Update state to initializing
        {
            let mut state = self.state.write().await;
            *state = ClientState::Initializing;
        }

        // Start the transport
        self.transport.start().await?;

        // Create initialize parameters
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: Default::default(),
            client_info: ClientInfo {
                name: self.name.clone(),
                version: self.version.clone(),
            },
        };

        // Send initialize request
        let id = self.next_request_id().await?;
        let response = self
            .send_request(methods::INITIALIZE, Some(json!(params)), id.to_string())
            .await?;

        match response {
            JsonRpcMessage::Response { result, error, .. } => {
                if let Some(error) = error {
                    return Err(anyhow!(
                        "Initialize error: {} (code: {})",
                        error.message,
                        error.code
                    ));
                }

                if let Some(result) = result {
                    let result: InitializeResult = serde_json::from_value(result)?;

                    // Store the result
                    {
                        let mut initialized = self.initialized_result.write().await;
                        *initialized = Some(result.clone());
                    }

                    // Send initialized notification
                    self.transport
                        .send(JsonRpcMessage::notification(methods::INITIALIZED, None))
                        .await?;

                    // Update state to ready
                    {
                        let mut state = self.state.write().await;
                        *state = ClientState::Ready;
                    }

                    return Ok(result);
                }

                Err(anyhow!("Invalid initialize response"))
            }
            _ => Err(anyhow!("Invalid response type")),
        }
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<ToolsListResult> {
        // Check if we're initialized
        {
            let state = self.state.read().await;
            if *state != ClientState::Ready {
                return Err(anyhow!("Client not initialized"));
            }
        }

        // Send tools/list request
        let id = self.next_request_id().await?;
        let response = self
            .send_request(methods::TOOLS_LIST, None, id.to_string())
            .await?;

        match response {
            JsonRpcMessage::Response { result, error, .. } => {
                if let Some(error) = error {
                    return Err(anyhow!(
                        "List tools error: {} (code: {})",
                        error.message,
                        error.code
                    ));
                }

                if let Some(result) = result {
                    let result: ToolsListResult = serde_json::from_value(result)?;
                    return Ok(result);
                }

                Err(anyhow!("Invalid list tools response"))
            }
            _ => Err(anyhow!("Invalid response type")),
        }
    }

    /// Call a tool on the server
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolCallResult> {
        // Check if we're initialized
        {
            let state = self.state.read().await;
            if *state != ClientState::Ready {
                return Err(anyhow!("Client not initialized"));
            }
        }

        // Create tool call parameters
        let params = ToolCallParams {
            name: name.to_string(),
            arguments: arguments.clone(),
        };

        // Send tools/call request
        let id = self.next_request_id().await?;
        let response = self
            .send_request(methods::TOOLS_CALL, Some(json!(params)), id.to_string())
            .await?;

        match response {
            JsonRpcMessage::Response { result, error, .. } => {
                if let Some(error) = error {
                    return Err(anyhow!(
                        "Tool call error: {} (code: {})",
                        error.message,
                        error.code
                    ));
                }

                if let Some(result) = result {
                    let result: ToolCallResult = serde_json::from_value(result)?;
                    return Ok(result);
                }

                Err(anyhow!("Invalid tool call response"))
            }
            _ => Err(anyhow!("Invalid response type")),
        }
    }

    /// Shutdown the client
    pub async fn shutdown(&self) -> Result<()> {
        // Check if we're initialized
        {
            let state = self.state.read().await;
            if *state != ClientState::Ready {
                return Err(anyhow!("Client not initialized"));
            }
        }

        // Update state to shutting down
        {
            let mut state = self.state.write().await;
            *state = ClientState::ShuttingDown;
        }

        // Close the transport
        self.transport.close().await?;

        Ok(())
    }
    
    /// Refresh the list of available prompts
    pub async fn refresh_prompts(&self) -> Result<serde_json::Value> {
        // Check if we're initialized
        {
            let state = self.state.read().await;
            if *state != ClientState::Ready {
                return Err(anyhow!("Client not initialized"));
            }
        }
        
        // Send prompts/list request
        let id = self.next_request_id().await?;
        let response = self
            .send_request(methods::PROMPTS_LIST, None, id.to_string())
            .await?;
        
        match response {
            JsonRpcMessage::Response { result, error, .. } => {
                if let Some(error) = error {
                    return Err(anyhow!(
                        "List prompts error: {} (code: {})",
                        error.message,
                        error.code
                    ));
                }
                
                if let Some(result) = result {
                    return Ok(result);
                }
                
                Err(anyhow!("Invalid list prompts response"))
            }
            _ => Err(anyhow!("Invalid response type")),
        }
    }

    /// Get the next request ID
    pub async fn next_request_id(&self) -> Result<i64> {
        let mut id = self.next_id.lock().await;
        let current = *id;
        *id += 1;
        Ok(current)
    }

    /// Send a request and wait for a response
    pub async fn send_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        id: String,
    ) -> Result<JsonRpcMessage> {
        // Create request
        let request = JsonRpcMessage::request(id.clone().into(), method, params);

        // Create response channel
        let (tx, mut rx) = mpsc::channel(1);

        // Register pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id.clone(), PendingRequest { response_tx: tx });
        }

        // Send request
        self.transport.send(request).await?;

        // Wait for response
        match rx.recv().await {
            Some(result) => {
                // Remove pending request
                let mut pending = self.pending_requests.write().await;
                pending.remove(&id);

                result
            }
            None => Err(anyhow!("Failed to receive response")),
        }
    }

    /// Handle a received message
    pub async fn handle_message(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Response { ref id, .. } => {
                // Get id as string
                let id = match id {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => return Err(anyhow!("Invalid response ID type")),
                };

                // Find pending request
                let pending = {
                    let pending = self.pending_requests.read().await;
                    match pending.get(&id) {
                        Some(req) => req.response_tx.clone(),
                        None => return Err(anyhow!("No pending request for ID: {}", id)),
                    }
                };

                // Send response
                if let Err(e) = pending.send(Ok(message)).await {
                    Err(anyhow!("Failed to send response: {}", e))
                } else {
                    Ok(())
                }
            }
            JsonRpcMessage::Notification { method, params, .. } => {
                // Handle notification
                match method.as_str() {
                    // Handle prompt list changed notification
                    methods::PROMPTS_LIST_CHANGED => {
                        // Emit a debug message about the change
                        tracing::debug!("Received notification: prompts list changed");
                        
                        // We could trigger a refresh of the prompts list here
                        // but we'll skip it for now to avoid complexity with clones
                        Ok(())
                    },
                    // Handle resource updated notification
                    methods::RESOURCES_UPDATED => {
                        // Extract the resource URI if available
                        if let Some(params) = params {
                            if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                                tracing::debug!("Received notification: resource updated - URI: {}", uri);
                            }
                        }
                        Ok(())
                    },
                    // Add other handlers for specific notifications here
                    _ => {
                        tracing::debug!("Unhandled notification: {}", method);
                        Ok(())
                    }
                }
            }
            JsonRpcMessage::Request { .. } => {
                // Implement request handling if needed
                tracing::debug!("Unhandled server request");
                Ok(())
            }
        }
    }
}
