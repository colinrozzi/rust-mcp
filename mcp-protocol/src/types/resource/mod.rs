// mcp-protocol/src/types/resource/mod.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a resource that can be accessed by the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// URI that uniquely identifies the resource
    pub uri: String,
    
    /// Human-readable name of the resource
    pub name: String,
    
    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Optional MIME type of the resource content
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Optional size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    
    /// Optional custom annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, serde_json::Value>>,
}

/// Content of a resource, which can be either text or binary data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// URI that uniquely identifies the resource
    pub uri: String,
    
    /// MIME type of the resource content
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    
    /// Text content (used for text resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    
    /// Binary content encoded as base64 (used for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// Parameters for listing resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Result of listing resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResult {
    /// List of available resources
    pub resources: Vec<Resource>,
    
    /// Optional cursor for the next page of results
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Parameters for reading a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadParams {
    /// URI of the resource to read
    pub uri: String,
}

/// Result of reading a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadResult {
    /// Contents of the resource
    pub contents: Vec<ResourceContent>,
}

/// Parameters for subscribing to a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSubscribeParams {
    /// URI of the resource to subscribe to
    pub uri: String,
}

/// Parameters for a resource update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdatedParams {
    /// URI of the updated resource
    pub uri: String,
}
