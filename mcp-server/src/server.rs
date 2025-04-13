// mcp-server/src/server.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use mcp_protocol::{
    constants::{error_codes, methods, PROTOCOL_VERSION},
    messages::{InitializeParams, InitializeResult, JsonRpcMessage},
    types::{
        tool::{Tool, ToolCallParams, ToolCallResult},
        ServerInfo, ServerState,
    },
    version::{is_supported_version, version_mismatch_error},
};

use crate::tools::ToolManager;
use crate::transport::Transport;

/// MCP server builder
pub struct ServerBuilder {
    name: String,
    version: String,
    transport: Option<Box<dyn Transport>>,
    tool_manager: Option<Arc<ToolManager>>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            transport: None,
            tool_manager: None,
        }
    }

    /// Set the transport to use
    pub fn with_transport<T: Transport>(mut self, transport: T) -> Self {
        self.transport = Some(Box::new(transport));
        self
    }

    /// Set the tool manager
    pub fn with_tool_manager(mut self, tool_manager: Arc<ToolManager>) -> Self {
        self.tool_manager = Some(tool_manager);
        self
    }

    /// Register a tool (creates a tool manager if not already set)
    pub fn with_tool(
        mut self,
        name: &str,
        description: Option<&str>,
        input_schema: serde_json::Value,
        handler: impl Fn(serde_json::Value) -> Result<ToolCallResult> + Send + Sync + 'static,
    ) -> Self {
        // Create tool manager if not already set
        if self.tool_manager.is_none() {
            self.tool_manager = Some(Arc::new(ToolManager::new()));
        }

        // Create tool
        let tool = Tool {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            input_schema,
            annotations: None,
        };

        // Register tool
        let tool_manager = self.tool_manager.as_ref().unwrap();
        tool_manager.register_tool(tool, handler);

        self
    }

    /// Build the server
    pub fn build(self) -> Result<Server> {
        let transport = self
            .transport
            .ok_or_else(|| anyhow!("Transport is required"))?;

        Ok(Server {
            name: self.name,
            version: self.version,
            transport,
            tool_manager: self
                .tool_manager
                .unwrap_or_else(|| Arc::new(ToolManager::new())),
            state: Arc::new(AtomicU8::new(ServerState::Created as u8)),
        })
    }
}

/// MCP server
pub struct Server {
    name: String,
    version: String,
    transport: Box<dyn Transport>,
    tool_manager: Arc<ToolManager>,
    state: Arc<AtomicU8>,
}

impl Server {
    /// Get the server capabilities
    fn get_capabilities(&self) -> HashMap<String, bool> {
        let mut capabilities = HashMap::new();
        capabilities.insert("listChanged".to_string(), true);
        capabilities
    }

    /// Get the server info
    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            name: self.name.clone(),
            version: self.version.clone(),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Parse initialize parameters
                let params: InitializeParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid initialize parameters: {}", err),
                                    None,
                                ))
                                .await?;
                            return Ok(());
                        }
                    },
                    None => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INVALID_PARAMS,
                                "Missing initialize parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Validate protocol version
                if !is_supported_version(&params.protocol_version) {
                    // Send error response
                    self.transport
                        .send(JsonRpcMessage::error(
                            id,
                            error_codes::INVALID_PARAMS,
                            "Unsupported protocol version",
                            Some(json!(version_mismatch_error(&params.protocol_version))),
                        ))
                        .await?;
                    return Ok(());
                }

                // Update server state
                self.state
                    .store(ServerState::Initializing as u8, Ordering::SeqCst);

                // Create server capabilities
                let mut capabilities = HashMap::new();
                capabilities.insert("tools".to_string(), Some(self.get_capabilities()));

                // Create initialize result
                let result = InitializeResult {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    capabilities: Default::default(),
                    server_info: self.get_server_info(),
                    instructions: None,
                };

                // Send initialize response
                self.transport
                    .send(JsonRpcMessage::response(id, json!(result)))
                    .await?;

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for initialize")),
        }
    }

    /// Handle initialized notification
    async fn handle_initialized(&self) -> Result<()> {
        // Update server state
        self.state.store(ServerState::Ready as u8, Ordering::SeqCst);

        // No response needed for notifications
        Ok(())
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, .. } => {
                // Check if server is ready
                if self.state.load(Ordering::SeqCst) != ServerState::Ready as u8 {
                    // Send error response
                    self.transport
                        .send(JsonRpcMessage::error(
                            id,
                            error_codes::SERVER_NOT_INITIALIZED,
                            "Server not initialized",
                            None,
                        ))
                        .await?;
                    return Ok(());
                }

                // Get tools from manager
                let tools = self.tool_manager.list_tools().await;

                // Send response
                self.transport
                    .send(JsonRpcMessage::response(
                        id,
                        json!({
                            "tools": tools,
                            "nextCursor": null
                        }),
                    ))
                    .await?;

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for tools/list")),
        }
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Check if server is ready
                if self.state.load(Ordering::SeqCst) != ServerState::Ready as u8 {
                    // Send error response
                    self.transport
                        .send(JsonRpcMessage::error(
                            id,
                            error_codes::SERVER_NOT_INITIALIZED,
                            "Server not initialized",
                            None,
                        ))
                        .await?;
                    return Ok(());
                }

                // Parse tool call parameters
                let params: ToolCallParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid tool call parameters: {}", err),
                                    None,
                                ))
                                .await?;
                            return Ok(());
                        }
                    },
                    None => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INVALID_PARAMS,
                                "Missing tool call parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Execute tool
                match self
                    .tool_manager
                    .execute_tool(&params.name, params.arguments)
                    .await
                {
                    Ok(result) => {
                        // Send response
                        self.transport
                            .send(JsonRpcMessage::response(id, json!(result)))
                            .await?;
                    }
                    Err(err) => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INTERNAL_ERROR,
                                &format!("Tool execution error: {}", err),
                                None,
                            ))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for tools/call")),
        }
    }

    /// Handle incoming messages
    async fn handle_message(&self, message: JsonRpcMessage) -> Result<()> {
        match &message.clone() {
            JsonRpcMessage::Request { method, .. } => {
                match method.as_str() {
                    methods::INITIALIZE => self.handle_initialize(message).await?,
                    methods::TOOLS_LIST => self.handle_tools_list(message).await?,
                    methods::TOOLS_CALL => self.handle_tools_call(message).await?,
                    _ => {
                        if let JsonRpcMessage::Request { id, .. } = message {
                            // Method not found
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::METHOD_NOT_FOUND,
                                    &format!("Method not found: {}", method),
                                    None,
                                ))
                                .await?;
                        }
                    }
                }
            }
            JsonRpcMessage::Notification { method, .. } => match method.as_str() {
                methods::INITIALIZED => self.handle_initialized().await?,
                _ => {
                    tracing::debug!("Unhandled notification: {}", method);
                }
            },
            _ => {
                // Not sure what to do with responses from the client
                tracing::debug!("Unexpected message type from client");
            }
        }

        Ok(())
    }

    /// Start the server and run until shutdown
    pub async fn run(&self) -> Result<()> {
        // Create message channel
        let (tx, mut rx) = mpsc::channel::<JsonRpcMessage>(100);

        // Start transport
        self.transport.start(tx).await?;

        // Process messages
        while let Some(message) = rx.recv().await {
            if let Err(err) = self.handle_message(message).await {
                tracing::error!("Error handling message: {}", err);
            }
        }

        // Update state
        self.state
            .store(ServerState::ShuttingDown as u8, Ordering::SeqCst);

        // Close transport
        self.transport.close().await?;

        Ok(())
    }

    /// Get a reference to the tool manager
    pub fn tool_manager(&self) -> &Arc<ToolManager> {
        &self.tool_manager
    }
}
