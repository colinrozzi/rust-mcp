// mcp-server/src/completion.rs
use anyhow::{anyhow, Result};
use serde_json::json;
use mcp_protocol::{
    constants::error_codes,
    messages::JsonRpcMessage,
    types::completion::{
        CompletionCompleteParams, CompletionInfo, CompletionReference
    },
};

use crate::server::Server;

impl Server {
    /// Handle completion/complete request
    pub(crate) async fn handle_completion_complete(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Request { id, params, .. } => {
                // Parse parameters
                let params: CompletionCompleteParams = match params {
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
                        if let Some(param_name) = extract_parameter_from_uri(uri, &params.argument.name) {
                            match self.resource_manager().get_completions(
                                uri,
                                &param_name,
                                params.argument.value.clone(),
                            ).await {
                                Ok(items) => {
                                    // Convert CompletionItem array to string array for the standard API
                                    let values = items.iter()
                                        .map(|item| item.label.clone())
                                        .collect::<Vec<String>>();
                                    
                                    // Create completion info
                                    let completion_info = CompletionInfo {
                                        values,
                                        total: Some(items.len() as u32),
                                        has_more: false,
                                    };
                                    
                                    // Send response
                                    self.transport()
                                        .send(JsonRpcMessage::response(
                                            id,
                                            json!({
                                                "completion": completion_info
                                            }),
                                        ))
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
                            self.transport()
                                .send(JsonRpcMessage::response(
                                    id,
                                    json!({
                                        "completion": {
                                            "values": [],
                                            "hasMore": false
                                        }
                                    }),
                                ))
                                .await?;
                        }
                    }
                    CompletionReference::Prompt { name: _ } => {
                        // For now, return empty completion for prompts
                        // In a real implementation, you'd have a prompt registry
                        self.transport()
                            .send(JsonRpcMessage::response(
                                id,
                                json!({
                                    "completion": {
                                        "values": [],
                                        "hasMore": false
                                    }
                                }),
                            ))
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
