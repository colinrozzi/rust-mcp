// mcp-server/src/server.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use mcp_protocol::{
    constants::{error_codes, methods, PROTOCOL_VERSION},
    messages::{InitializeParams, InitializeResult, JsonRpcMessage, ServerCapabilities},
    types::{
        tool::{Tool, ToolCallParams, ToolCallResult},
        resource::{Resource, ResourceContent, ResourcesListParams, ResourceReadParams, ResourceSubscribeParams},
        ServerInfo, ServerState,
    },
    version::{is_supported_version, version_mismatch_error},
};

use crate::tools::ToolManager;
use crate::resources::ResourceManager;
use crate::transport::Transport;

/// MCP server builder
pub struct ServerBuilder {
    name: String,
    version: String,
    transport: Option<Box<dyn Transport>>,
    tool_manager: Option<Arc<ToolManager>>,
    resource_manager: Option<Arc<ResourceManager>>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            transport: None,
            tool_manager: None,
            resource_manager: None,
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

    /// Set the resource manager
    pub fn with_resource_manager(mut self, resource_manager: Arc<ResourceManager>) -> Self {
        self.resource_manager = Some(resource_manager);
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

    /// Register a resource (creates a resource manager if not already set)
    pub fn with_resource(
        mut self,
        uri: &str,
        name: &str,
        description: Option<&str>,
        mime_type: Option<&str>,
        size: Option<u64>,
        content_provider: impl Fn() -> Result<Vec<ResourceContent>> + Send + Sync + 'static,
    ) -> Self {
        // Create resource manager if not already set
        if self.resource_manager.is_none() {
            self.resource_manager = Some(Arc::new(ResourceManager::new()));
        }

        // Create resource
        let resource = Resource {
            uri: uri.to_string(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            mime_type: mime_type.map(|s| s.to_string()),
            size,
            annotations: None,
        };

        // Register resource
        let resource_manager = self.resource_manager.as_ref().unwrap();
        resource_manager.register_resource(resource, content_provider);

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
            resource_manager: self
                .resource_manager
                .unwrap_or_else(|| Arc::new(ResourceManager::new())),
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
    resource_manager: Arc<ResourceManager>,
    state: Arc<AtomicU8>,
}

impl Server {
    /// Get the tool capabilities
    fn get_tool_capabilities(&self) -> HashMap<String, bool> {
        let mut capabilities = HashMap::new();
        capabilities.insert("listChanged".to_string(), true);
        capabilities
    }

    /// Get the resource capabilities
    fn get_resource_capabilities(&self) -> HashMap<String, bool> {
        let mut capabilities = HashMap::new();
        capabilities.insert("listChanged".to_string(), true);
        capabilities.insert("subscribe".to_string(), true);
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

                // Get capabilities
                let tools_capabilities = self.get_tool_capabilities();
                let resources_capabilities = self.get_resource_capabilities();
                
                // Create initialize result
                let result = InitializeResult {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    capabilities: ServerCapabilities {
                        tools: Some(tools_capabilities),
                        resources: Some(resources_capabilities),
                        ..Default::default()
                    },
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
                            "nextCursor": ""
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

    /// Handle resources/list request
    async fn handle_resources_list(&self, message: JsonRpcMessage) -> Result<()> {
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

                // Parse parameters (optional)
                let _params: Option<ResourcesListParams> = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => Some(params),
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid resource list parameters: {}", err),
                                    None,
                                ))
                                .await?;
                            return Ok(());
                        }
                    },
                    None => None,
                };

                // Get resources from manager
                let resources = self.resource_manager.list_resources().await;

                // Send response
                self.transport
                    .send(JsonRpcMessage::response(
                        id,
                        json!({
                            "resources": resources,
                            "nextCursor": ""
                        }),
                    ))
                    .await?;

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for resources/list")),
        }
    }

    /// Handle resources/read request
    async fn handle_resources_read(&self, message: JsonRpcMessage) -> Result<()> {
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

                // Parse parameters
                let params: ResourceReadParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid resource read parameters: {}", err),
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
                                "Missing resource read parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Read resource
                match self.resource_manager.get_resource_content(&params.uri).await {
                    Ok(contents) => {
                        // Send response
                        self.transport
                            .send(JsonRpcMessage::response(
                                id,
                                json!({
                                    "contents": contents
                                }),
                            ))
                            .await?;
                    }
                    Err(err) => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::RESOURCE_NOT_FOUND,
                                &format!("Resource not found: {}", err),
                                Some(json!({
                                    "uri": params.uri
                                })),
                            ))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for resources/read")),
        }
    }

    /// Handle resources/subscribe request
    async fn handle_resources_subscribe(&self, message: JsonRpcMessage) -> Result<()> {
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

                // Parse parameters
                let params: ResourceSubscribeParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid resource subscribe parameters: {}", err),
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
                                "Missing resource subscribe parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Subscribe to resource
                let client_id = id.to_string(); // Use request ID as client ID for simplicity
                match self.resource_manager.subscribe(&client_id, &params.uri).await {
                    Ok(_) => {
                        // Send success response
                        self.transport
                            .send(JsonRpcMessage::response(
                                id,
                                json!({
                                    "success": true
                                }),
                            ))
                            .await?;
                    }
                    Err(err) => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::RESOURCE_NOT_FOUND,
                                &format!("Resource subscription error: {}", err),
                                Some(json!({
                                    "uri": params.uri
                                })),
                            ))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for resources/subscribe")),
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
                    methods::RESOURCES_LIST => self.handle_resources_list(message).await?,
                    methods::RESOURCES_READ => self.handle_resources_read(message).await?,
                    methods::RESOURCES_SUBSCRIBE => self.handle_resources_subscribe(message).await?,
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

        // Set up resource update listener
        let update_rx = self.resource_manager.subscribe_to_updates();
        let transport = self.transport.box_clone();
        
        // Spawn a task to handle resource updates
        tokio::spawn(async move {
            let mut update_rx = update_rx;
            while let Ok(uri) = update_rx.recv().await {
                // Send notification
                let _ = transport
                    .send(JsonRpcMessage::notification(
                        methods::RESOURCES_UPDATED,
                        Some(json!({ "uri": uri })),
                    ))
                    .await;
            }
        });

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

    /// Get a reference to the resource manager
    pub fn resource_manager(&self) -> &Arc<ResourceManager> {
        &self.resource_manager
    }
}
