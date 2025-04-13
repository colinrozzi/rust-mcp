// mcp-server/src/prompts.rs
use anyhow::{anyhow, Result};
use mcp_protocol::types::prompt::{Prompt, PromptGetResult, PromptMessage};
use std::collections::HashMap;
use std::sync::{RwLock};
use tokio::sync::broadcast;

/// Handler type for generating prompt messages
pub type PromptHandler = Box<dyn Fn(Option<HashMap<String, String>>) -> Result<Vec<PromptMessage>> + Send + Sync>;

/// Manages prompts for the MCP server
pub struct PromptManager {
    /// Map of prompt name to prompt definition
    prompts: RwLock<HashMap<String, Prompt>>,
    
    /// Map of prompt name to prompt handler
    handlers: RwLock<HashMap<String, PromptHandler>>,
    
    /// Sender for update notifications
    update_tx: broadcast::Sender<()>,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new() -> Self {
        let (update_tx, _) = broadcast::channel(100);
        
        Self {
            prompts: RwLock::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
            update_tx,
        }
    }
    
    /// Register a prompt with the manager
    pub fn register_prompt(
        &self,
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
        let _ = self.update_tx.send(());
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
        
        // Validate arguments against the prompt definition
        self.validate_arguments(&prompt, &arguments)?;
        
        // Get handler and execute it
        let messages = {
            let handlers = self.handlers.read().unwrap();
            if let Some(handler) = handlers.get(name) {
                // Execute handler
                handler(arguments.clone())?                
            } else {
                return Err(anyhow!("Handler not found for prompt: {}", name));
            }
        };
        
        // Construct result
        let result = PromptGetResult {
            description: prompt.description,
            messages,
        };
        
        Ok(result)
    }
    
    /// Subscribe to prompt list updates
    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<()> {
        self.update_tx.subscribe()
    }
    
    /// Add an annotation to a prompt
    pub async fn add_annotation(&self, name: &str, key: &str, value: serde_json::Value) -> Result<()> {
        let mut prompts = self.prompts.write().unwrap();
        
        if let Some(prompt) = prompts.get_mut(name) {
            // Initialize annotations if not present
            if prompt.annotations.is_none() {
                prompt.annotations = Some(HashMap::new());
            }
            
            // Add or update annotation
            if let Some(annotations) = &mut prompt.annotations {
                annotations.insert(key.to_string(), value);
            }
            
            // Notify of update
            let _ = self.update_tx.send(());
            
            Ok(())
        } else {
            Err(anyhow!("Prompt not found: {}", name))
        }
    }
    
    /// Get an annotation from a prompt
    pub async fn get_annotation(&self, name: &str, key: &str) -> Result<Option<serde_json::Value>> {
        let prompts = self.prompts.read().unwrap();
        
        if let Some(prompt) = prompts.get(name) {
            if let Some(annotations) = &prompt.annotations {
                Ok(annotations.get(key).cloned())
            } else {
                Ok(None)
            }
        } else {
            Err(anyhow!("Prompt not found: {}", name))
        }
    }
    
    /// Validate prompt arguments against the prompt definition
    fn validate_arguments(
        &self,
        prompt: &Prompt,
        arguments: &Option<HashMap<String, String>>
    ) -> Result<()> {
        // Check for required arguments
        if let Some(prompt_args) = &prompt.arguments {
            for arg in prompt_args {
                if arg.required.unwrap_or(false) {
                    match arguments {
                        Some(args) => {
                            if !args.contains_key(&arg.name) {
                                return Err(anyhow!("Missing required argument: {}", arg.name));
                            }
                            
                            // Check for empty values
                            if let Some(value) = args.get(&arg.name) {
                                if value.trim().is_empty() {
                                    return Err(anyhow!("Required argument cannot be empty: {}", arg.name));
                                }
                            }
                        },
                        None => return Err(anyhow!("Missing required arguments")),
                    }
                }
            }
            
            // Check for unexpected arguments
            if let Some(args) = arguments {
                for arg_name in args.keys() {
                    if !prompt_args.iter().any(|a| &a.name == arg_name) {
                        return Err(anyhow!("Unexpected argument: {}", arg_name));
                    }
                }
            }
        }
        
        Ok(())
    }
}
