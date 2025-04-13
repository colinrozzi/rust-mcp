// mcp-server/src/resource_extensions.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use mcp_protocol::{
    constants::error_codes,
    messages::JsonRpcMessage,
    types::resource::{ResourceTemplatesListParams, ResourceUnsubscribeParams},
};

use crate::server::Server;

impl Server {
    /// Handle resources/templates/list request
    pub(crate) async fn handle_resources_templates_list(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Check if server is ready
                if self.state().load(std::sync::atomic::Ordering::SeqCst) != mcp_protocol::types::ServerState::Ready as u8 {
                    // Send error response
                    self.transport()
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
                let params: Option<ResourceTemplatesListParams> = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => Some(params),
                        Err(err) => {
                            // Send error response
                            self.transport()
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid template list parameters: {}", err),
                                    None,
                                ))
                                .await?;
                            return Ok(());
                        }
                    },
                    None => None,
                };

                // Get cursor from parameters
                let cursor = params.and_then(|p| p.cursor);
                
                // Get templates from manager with pagination
                let (templates, next_cursor) = self.resource_manager().list_templates(cursor).await;

                // Send response
                self.transport()
                    .send(JsonRpcMessage::response(
                        id,
                        json!({
                            "resourceTemplates": templates,
                            "nextCursor": next_cursor.unwrap_or_default()
                        }),
                    ))
                    .await?;

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for resources/templates/list")),
        }
    }

    /// Handle resources/unsubscribe request
    pub(crate) async fn handle_resources_unsubscribe(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Check if server is ready
                if self.state().load(std::sync::atomic::Ordering::SeqCst) != mcp_protocol::types::ServerState::Ready as u8 {
                    // Send error response
                    self.transport()
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
                let params: ResourceUnsubscribeParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport()
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid resource unsubscribe parameters: {}", err),
                                    None,
                                ))
                                .await?;
                            return Ok(());
                        }
                    },
                    None => {
                        // Send error response
                        self.transport()
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INVALID_PARAMS,
                                "Missing resource unsubscribe parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Unsubscribe from resource
                let client_id = id.to_string(); // Use request ID as client ID for simplicity
                match self.resource_manager().unsubscribe(&client_id, &params.uri).await {
                    Ok(_) => {
                        // Send success response
                        self.transport()
                            .send(JsonRpcMessage::response(
                                id,
                                json!({
                                    "success": true
                                }),
                            ))
                            .await?;
                    }
                    Err(err) => {
                        // Send error response - but this is not critical, so use internal error
                        self.transport()
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INTERNAL_ERROR,
                                &format!("Resource unsubscribe error: {}", err),
                                None,
                            ))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for resources/unsubscribe")),
        }
    }
}
