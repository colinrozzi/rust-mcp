// mcp-server/src/prompts.rs
use anyhow::{anyhow, Result};
use mcp_protocol::types::prompt::{Prompt, PromptGetResult, PromptMessage};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{self, Receiver, Sender};

/// Handler type for generating prompt messages
pub type PromptHandler = Box<dyn Fn(Option<HashMap<String, String>>) -> Result<Vec<PromptMessage>> + Send + Sync>;

/// Manages prompts for the MCP server
pub struct PromptManager {
    /// Map of prompt name to prompt definition
    prompts: RwLock<HashMap<String, Prompt>>,
    
    /// Map of prompt name to prompt handler
    handlers: RwLock<HashMap<String, PromptHandler>>,
    
    /// Sender for update notifications
    update_tx: Sender<()>,
    
    /// Receiver for update notifications (cloned for subscribers)
    update_rx: RwLock<Receiver<()>>,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new() -> Arc<Self> {
        let (tx, rx) = mpsc::channel(100);
        
        Arc::new(Self {
            prompts: RwLock::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
            update_tx: tx,
            update_rx: RwLock::new(rx),
        })
    }
    
    /// Register a prompt with the manager
    pub fn register_prompt(
        self: &Arc<Self>,
        prompt: Prompt,
        handler: impl Fn(Option<HashMap<String, String>>) -> Result<Vec<PromptMessage>> + Send + Sync + 'static,
    ) {
        let name = prompt.name.clone();
        
        // Add prompt to registry
        {
            let mut prompts = self.prompts.write().unwrap();
            prompts.insert(name.clone(), prompt);
        }
        
        // Add handler to registry
        {
            let mut handlers = self.handlers.write().unwrap();
            handlers.insert(name, Box::new(handler));
        }
        
        // Notify of update
        let _ = self.update_tx.try_send(());
    }
    
    /// List all registered prompts with optional pagination
    pub async fn list_prompts(&self, cursor: Option<String>) -> (Vec<Prompt>, Option<String>) {
        let prompts = self.prompts.read().unwrap();
        
        // Get all prompts in a vector
        let mut prompt_list: Vec<Prompt> = prompts.values().cloned().collect();
        
        // Sort by name for consistent ordering
        prompt_list.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Simple pagination implementation
        if let Some(cursor) = cursor {
            if !cursor.is_empty() {
                // Skip items before the cursor
                prompt_list = prompt_list
                    .into_iter()
                    .skip_while(|p| p.name != cursor)
                    .skip(1) // Skip the cursor item itself
                    .collect();
            }
        }
        
        // For simplicity, we'll return at most 50 items per page
        let page_size = 50;
        let next_cursor = if prompt_list.len() > page_size {
            // If we have more than page_size, return the next cursor
            prompt_list[page_size - 1].name.clone()
        } else {
            // No more pages
            return (prompt_list, None);
        };
        
        // Return the current page and the next cursor
        (prompt_list.into_iter().take(page_size).collect(), Some(next_cursor))
    }
    
    /// Get a prompt by name and generate its content with the provided arguments
    pub async fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, String>>) -> Result<PromptGetResult> {
        // Get prompt definition
        let prompt = {
            let prompts = self.prompts.read().unwrap();
            prompts.get(name).cloned().ok_or_else(|| anyhow!("Prompt not found: {}", name))?
        };
        
        // Get handler
        let handler = {
            let handlers = self.handlers.read().unwrap();
            handlers.get(name).cloned().ok_or_else(|| anyhow!("Handler not found for prompt: {}", name))?
        };
        
        // Validate required arguments
        if let Some(prompt_args) = &prompt.arguments {
            for arg in prompt_args {
                if arg.required.unwrap_or(false) {
                    if let Some(args) = &arguments {
                        if !args.contains_key(&arg.name) {
                            return Err(anyhow!("Missing required argument: {}", arg.name));
                        }
                    } else {
                        return Err(anyhow!("Missing required argument: {}", arg.name));
                    }
                }
            }
        }
        
        // Generate messages using handler
        let messages = handler(arguments)?;
        
        // Construct result
        let result = PromptGetResult {
            description: prompt.description,
            messages,
        };
        
        Ok(result)
    }
    
    /// Subscribe to prompt list updates
    pub fn subscribe_to_updates(&self) -> Receiver<()> {
        let (tx, rx) = mpsc::channel(100);
        
        // Clone the sender to use in the task
        let update_tx = tx.clone();
        
        // Create a task to forward updates
        tokio::spawn(async move {
            loop {
                // Forward update notifications
                let _ = update_tx.send(()).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        
        rx
    }
}
