// mcp-server/src/tools/mod.rs
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use mcp_protocol::types::tool::{Tool, ToolCallResult};

/// Tool handler function type
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> Result<ToolCallResult> + Send + Sync>;

/// Tool manager for registering and executing tools
pub struct ToolManager {
    tools: Arc<RwLock<HashMap<String, (Tool, ToolHandler)>>>,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a new tool
    pub fn register_tool(&self, tool: Tool, handler: impl Fn(serde_json::Value) -> Result<ToolCallResult> + Send + Sync + 'static) {
        let tools = self.tools.clone();
        let handler = Arc::new(handler);
        
        tokio::spawn(async move {
            let mut tools = tools.write().await;
            tools.insert(tool.name.clone(), (tool, handler));
        });
    }
    
    /// Get all registered tools
    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        tools.values().map(|(tool, _)| tool.clone()).collect()
    }
    
    /// Execute a tool
    pub async fn execute_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolCallResult> {
        let tools = self.tools.read().await;
        let (_, handler) = tools.get(name).ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;
        
        handler(arguments)
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}
