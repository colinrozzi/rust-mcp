// mcp-protocol/src/types/completion/mod.rs
use serde::{Deserialize, Serialize};

/// Reference to a prompt or resource for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompletionReference {
    /// Reference to a prompt
    #[serde(rename = "ref/prompt")]
    Prompt {
        /// Name of the prompt
        name: String,
    },
    /// Reference to a resource
    #[serde(rename = "ref/resource")]
    Resource {
        /// URI of the resource
        uri: String,
    },
}

/// Argument to complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionArgument {
    /// Name of the argument
    pub name: String,
    
    /// Current value of the argument
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Parameters for the completion/complete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCompleteParams {
    /// Reference to what is being completed
    pub r#ref: CompletionReference,
    
    /// Argument being completed
    pub argument: CompletionArgument,
}

/// Completion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCompleteResult {
    /// Completion information
    pub completion: CompletionInfo,
}

/// Completion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionInfo {
    /// List of completion values, sorted by relevance
    pub values: Vec<String>,
    
    /// Total number of available matches (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    
    /// Flag indicating if there are more results available
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

/// A single completion item with more details
/// This is from our previous implementation and is maintained
/// for backward compatibility with our resource template completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// The completion label to display
    pub label: String,
    
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Value to insert if selected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
}
