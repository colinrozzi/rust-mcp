// mcp-server/src/resources/mod.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::{RwLock, broadcast};
use mcp_protocol::types::resource::{
    Resource, ResourceContent, ResourceTemplate, 
    CompletionItem
};

const DEFAULT_PAGE_SIZE: usize = 50;

/// Resource content provider function type
pub type ResourceContentProvider = Arc<dyn Fn() -> Result<Vec<ResourceContent>> + Send + Sync>;

/// Template completion provider function type
pub type TemplateCompletionProvider = Arc<dyn Fn(String, String, Option<String>) -> Result<Vec<CompletionItem>> + Send + Sync>;

/// Template expansion function type
pub type TemplateExpanderFn = Arc<dyn Fn(String, HashMap<String, String>) -> Result<String> + Send + Sync>;

/// Resource manager for registering and accessing resources
pub struct ResourceManager {
    resources: Arc<RwLock<HashMap<String, (Resource, ResourceContentProvider)>>>,
    templates: Arc<RwLock<HashMap<String, (ResourceTemplate, TemplateExpanderFn)>>>,
    subscriptions: Arc<RwLock<HashMap<String, HashSet<String>>>>, // Maps resource URI to set of client IDs
    update_tx: broadcast::Sender<String>, // Channel for notifying resource updates
    completion_providers: Arc<RwLock<HashMap<String, TemplateCompletionProvider>>>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        let (update_tx, _) = broadcast::channel(100);
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
            completion_providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a new resource
    pub fn register_resource(
        &self, 
        resource: Resource, 
        content_provider: impl Fn() -> Result<Vec<ResourceContent>> + Send + Sync + 'static
    ) {
        let resources = self.resources.clone();
        let content_provider = Arc::new(content_provider);
        
        tokio::spawn(async move {
            let mut resources = resources.write().await;
            resources.insert(resource.uri.clone(), (resource, content_provider));
        });
    }
    
    /// Get registered resources with pagination
    pub async fn list_resources(&self, cursor: Option<String>) -> (Vec<Resource>, Option<String>) {
        let resources = self.resources.read().await;
        let all_resources: Vec<Resource> = resources.values().map(|(resource, _)| resource.clone()).collect();
        
        // If we have a cursor, find its position
        let start_pos = match cursor {
            Some(cursor) => {
                // Find the index of the resource after the cursor
                let pos = all_resources.iter().position(|r| r.uri == cursor);
                pos.map(|p| p + 1).unwrap_or(0)
            },
            None => 0,
        };
        
        // Get a page of resources
        let end_pos = std::cmp::min(start_pos + DEFAULT_PAGE_SIZE, all_resources.len());
        let page = all_resources[start_pos..end_pos].to_vec();
        
        // Set the next cursor if there are more resources
        let next_cursor = if end_pos < all_resources.len() {
            Some(all_resources[end_pos - 1].uri.clone())
        } else {
            None
        };
        
        (page, next_cursor)
    }
    
    /// Get a specific resource's content
    pub async fn get_resource_content(&self, uri: &str) -> Result<Vec<ResourceContent>> {
        // First check if this is a direct resource
        let resources = self.resources.read().await;
        if let Some((_, content_provider)) = resources.get(uri) {
            return content_provider();
        }
        
        // If not a direct resource, check if it matches a template
        let templates = self.templates.read().await;
        for (template_uri, (_, _expander)) in templates.iter() {
            // Check if the URI could be from this template (simple prefix check)
            // In a real implementation, you'd want a more sophisticated matching algorithm
            if uri.starts_with(template_uri.split('{').next().unwrap_or("")) {
                // Try to find a resource provider for the expanded URI
                if let Some((_, content_provider)) = resources.get(uri) {
                    return content_provider();
                }
            }
        }
        
        Err(anyhow!("Resource not found: {}", uri))
    }
    
    /// Register a template
    pub fn register_template(
        &self,
        template: ResourceTemplate,
        expander: impl Fn(String, HashMap<String, String>) -> Result<String> + Send + Sync + 'static,
    ) {
        let templates = self.templates.clone();
        let expander = Arc::new(expander);
        
        tokio::spawn(async move {
            let mut templates = templates.write().await;
            templates.insert(template.uri_template.clone(), (template, expander));
        });
    }
    
    /// Register a completion provider for a template parameter
    pub fn register_completion_provider(
        &self,
        template_uri: &str,
        provider: impl Fn(String, String, Option<String>) -> Result<Vec<CompletionItem>> + Send + Sync + 'static,
    ) {
        let providers = self.completion_providers.clone();
        let template_uri = template_uri.to_string();
        let provider = Arc::new(provider);
        
        tokio::spawn(async move {
            let mut providers = providers.write().await;
            providers.insert(template_uri, provider);
        });
    }
    
    /// Get completion items for a template parameter
    pub async fn get_completions(
        &self,
        template_uri: &str,
        parameter: &str,
        value: Option<String>,
    ) -> Result<Vec<CompletionItem>> {
        let providers = self.completion_providers.read().await;
        
        if let Some(provider) = providers.get(template_uri) {
            return provider(template_uri.to_string(), parameter.to_string(), value);
        }
        
        // Return empty results if no provider is registered
        Ok(Vec::new())
    }
    
    /// Get all registered templates with pagination
    pub async fn list_templates(&self, cursor: Option<String>) -> (Vec<ResourceTemplate>, Option<String>) {
        let templates = self.templates.read().await;
        let all_templates: Vec<ResourceTemplate> = templates.values().map(|(template, _)| template.clone()).collect();
        
        // If we have a cursor, find its position
        let start_pos = match cursor {
            Some(cursor) => {
                // Find the index of the template after the cursor
                let pos = all_templates.iter().position(|t| t.uri_template == cursor);
                pos.map(|p| p + 1).unwrap_or(0)
            },
            None => 0,
        };
        
        // Get a page of templates
        let end_pos = std::cmp::min(start_pos + DEFAULT_PAGE_SIZE, all_templates.len());
        let page = all_templates[start_pos..end_pos].to_vec();
        
        // Set the next cursor if there are more templates
        let next_cursor = if end_pos < all_templates.len() {
            Some(all_templates[end_pos - 1].uri_template.clone())
        } else {
            None
        };
        
        (page, next_cursor)
    }
    
    /// Subscribe to resource updates
    pub async fn subscribe(&self, client_id: &str, uri: &str) -> Result<()> {
        // Check if resource exists
        {
            let resources = self.resources.read().await;
            if !resources.contains_key(uri) {
                return Err(anyhow::anyhow!("Resource not found: {}", uri));
            }
        }
        
        // Add subscription
        let mut subscriptions = self.subscriptions.write().await;
        let subscribers = subscriptions.entry(uri.to_string()).or_insert_with(HashSet::new);
        subscribers.insert(client_id.to_string());
        
        Ok(())
    }
    
    /// Unsubscribe from resource updates
    pub async fn unsubscribe(&self, client_id: &str, uri: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        if let Some(subscribers) = subscriptions.get_mut(uri) {
            subscribers.remove(client_id);
            if subscribers.is_empty() {
                subscriptions.remove(uri);
            }
        }
        
        Ok(())
    }
    
    /// Update a resource and notify subscribers
    pub async fn update_resource(
        &self, 
        resource: Resource, 
        content_provider: impl Fn() -> Result<Vec<ResourceContent>> + Send + Sync + 'static
    ) -> Result<()> {
        // Update resource
        {
            let mut resources = self.resources.write().await;
            resources.insert(resource.uri.clone(), (resource.clone(), Arc::new(content_provider)));
        }
        
        // Notify subscribers
        let _ = self.update_tx.send(resource.uri.clone());
        
        Ok(())
    }
    
    /// Get a channel for subscribing to resource updates
    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<String> {
        self.update_tx.subscribe()
    }
    
    /// Parse template parameters from a URI
    /// This is a simple implementation - a production version would need more robust parsing
    pub fn parse_template_parameters(&self, template: &str, uri: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Extract template parts - this is a very simple implementation
        // A real implementation would parse RFC 6570 URI templates properly
        let template_parts: Vec<&str> = template.split('{')
            .flat_map(|part| part.split('}')).collect();
        
        let mut uri_cursor = uri;
        
        for (i, part) in template_parts.iter().enumerate() {
            if i % 2 == 0 {
                // This is a literal part
                if uri_cursor.starts_with(part) {
                    uri_cursor = &uri_cursor[part.len()..];
                }
            } else {
                // This is a parameter name
                let param_name = *part;
                
                // Find the next literal part, if any
                let next_literal = if i + 1 < template_parts.len() {
                    template_parts[i + 1]
                } else {
                    ""
                };
                
                // Extract the parameter value
                let param_value = if next_literal.is_empty() {
                    uri_cursor.to_string()
                } else if let Some(pos) = uri_cursor.find(next_literal) {
                    let value = &uri_cursor[..pos];
                    uri_cursor = &uri_cursor[pos + next_literal.len()..];
                    value.to_string()
                } else {
                    uri_cursor.to_string()
                };
                
                params.insert(param_name.to_string(), param_value);
            }
        }
        
        params
    }
    
    /// Expand a template with parameters
    pub async fn expand_template(&self, template_uri: &str, params: HashMap<String, String>) -> Result<String> {
        let templates = self.templates.read().await;
        
        if let Some((_, expander)) = templates.get(template_uri) {
            return expander(template_uri.to_string(), params);
        }
        
        // Fallback to simple expansion if no custom expander is registered
        let mut result = template_uri.to_string();
        
        for (name, value) in params {
            result = result.replace(&format!("{{{}}}", name), &value);
        }
        
        Ok(result)
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
