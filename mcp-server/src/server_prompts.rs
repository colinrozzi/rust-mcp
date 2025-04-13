// mcp-server/src/server_prompts.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::atomic::Ordering;

use mcp_protocol::{
    constants::error_codes,
    messages::JsonRpcMessage,
    types::prompt::{PromptGetParams, PromptsListParams},
    types::ServerState,
};

use crate::server::Server;

impl Server {
    /// Handle prompts/list request
    pub(crate) async fn handle_prompts_list(&self, message: JsonRpcMessage) -> Result<()> {
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
                let params: Option<PromptsListParams> = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => Some(params),
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid prompt list parameters: {}", err),
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
                
                // Get prompts from manager with pagination
                let (prompts, next_cursor) = self.prompt_manager.list_prompts(cursor).await;

                // Send response
                self.transport
                    .send(JsonRpcMessage::response(
                        id,
                        json!({
                            "prompts": prompts,
                            "nextCursor": next_cursor.unwrap_or_default()
                        }),
                    ))
                    .await?;

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for prompts/list")),
        }
    }
    
    /// Handle prompts/get request
    pub(crate) async fn handle_prompts_get(&self, message: JsonRpcMessage) -> Result<()> {
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
                let params: PromptGetParams = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid prompt get parameters: {}", err),
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
                                "Missing prompt get parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Get prompt content
                match self.prompt_manager.get_prompt(&params.name, params.arguments).await {
                    Ok(result) => {
                        // Send response
                        self.transport
                            .send(JsonRpcMessage::response(
                                id,
                                json!(result),
                            ))
                            .await?;
                    }
                    Err(err) => {
                        // Send error response
                        self.transport
                            .send(JsonRpcMessage::error(
                                id,
                                error_codes::INVALID_PARAMS,
                                &format!("Prompt error: {}", err),
                                Some(json!({
                                    "name": params.name
                                })),
                            ))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for prompts/get")),
        }
    }
}
