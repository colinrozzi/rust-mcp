// mcp-server/src/completion_handler.rs
use anyhow::{anyhow, Result};
use mcp_protocol::{
    constants::error_codes,
    messages::JsonRpcMessage,
    types::completion::{CompleteRequest, CompleteResponse, CompletionReference, CompletionResult},
};
use serde_json::json;

use crate::server::Server;

impl Server {
    /// Handle completion/complete request
    pub(crate) async fn handle_completion_complete(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Parse parameters
                let params: CompleteRequest = match params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(params) => params,
                        Err(err) => {
                            // Send error response
                            self.transport()
                                .send(JsonRpcMessage::error(
                                    id,
                                    error_codes::INVALID_PARAMS,
                                    &format!("Invalid completion parameters: {}", err),
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
                                "Missing completion parameters",
                                None,
                            ))
                            .await?;
                        return Ok(());
                    }
                };

                // Based on the reference type, dispatch to the correct handler
                match &params.r#ref {
                    CompletionReference::Resource { uri } => {
                        // This is for resource template completion
                        // Extract parameter name from URI template
                        // This is a simple implementation - in reality you'd need more robust parsing
                        if let Some(param_name) =
                            extract_parameter_from_uri(uri, &params.argument.name)
                        {
                            match self
                                .resource_manager()
                                .get_completions(
                                    uri,
                                    &param_name,
                                    Some(params.argument.value.clone()),
                                )
                                .await
                            {
                                Ok(items) => {
                                    // Convert CompletionItem array to string array for the standard API
                                    let values = items
                                        .iter()
                                        .map(|item| item.label.clone())
                                        .collect::<Vec<String>>();

                                    // Create completion result
                                    let completion_result = CompletionResult {
                                        values,
                                        total: Some(items.len()),
                                        has_more: false,
                                    };

                                    // Create response
                                    let response = CompleteResponse {
                                        completion: completion_result,
                                    };

                                    // Send response
                                    self.transport()
                                        .send(JsonRpcMessage::response(id, json!(response)))
                                        .await?;
                                }
                                Err(err) => {
                                    // Send error response
                                    self.transport()
                                        .send(JsonRpcMessage::error(
                                            id,
                                            error_codes::INTERNAL_ERROR,
                                            &format!("Completion error: {}", err),
                                            None,
                                        ))
                                        .await?;
                                }
                            }
                        } else {
                            // Parameter not found in URI template
                            // Create empty completion result
                            let completion_result = CompletionResult {
                                values: vec![],
                                total: Some(0),
                                has_more: false,
                            };

                            // Create response
                            let response = CompleteResponse {
                                completion: completion_result,
                            };

                            self.transport()
                                .send(JsonRpcMessage::response(id, json!(response)))
                                .await?;
                        }
                    }
                    CompletionReference::Prompt { name } => {
                        // Check if we have a completion provider for this prompt
                        let prompt_manager = self.prompt_manager();
                        if let Ok(completions) = prompt_manager
                            .get_completions(
                                name,
                                &params.argument.name,
                                Some(params.argument.value.clone()),
                            )
                            .await
                        {
                            // Create completion result
                            let completion_result = CompletionResult {
                                values: completions,
                                total: None,
                                has_more: false,
                            };

                            // Create response
                            let response = CompleteResponse {
                                completion: completion_result,
                            };

                            self.transport()
                                .send(JsonRpcMessage::response(id, json!(response)))
                                .await?;
                            return Ok(());
                        }

                        // Prompt not found or parameter not found, return empty result
                        let completion_result = CompletionResult {
                            values: vec![],
                            total: Some(0),
                            has_more: false,
                        };

                        // Create response
                        let response = CompleteResponse {
                            completion: completion_result,
                        };

                        self.transport()
                            .send(JsonRpcMessage::response(id, json!(response)))
                            .await?;
                    }
                }

                Ok(())
            }
            _ => Err(anyhow!("Expected request message for completion/complete")),
        }
    }
}

/// Helper function to extract a parameter from a URI template
/// This is a very simple implementation and would need to be more robust in a real system
fn extract_parameter_from_uri(uri: &str, param_name: &str) -> Option<String> {
    // Look for {param_name} in the URI
    if uri.contains(&format!("{{{}}}", param_name)) {
        Some(param_name.to_string())
    } else {
        None
    }
}
