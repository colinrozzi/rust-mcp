// mcp-server/src/resources/mod.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::{RwLock, broadcast};
use mcp_protocol::types::resource::{Resource, ResourceContent};

/// Resource content provider function type
pub type ResourceContentProvider = Arc<dyn Fn() -> Result<Vec<ResourceContent>> + Send + Sync>;

/// Resource manager for registering and accessing resources
pub struct ResourceManager {
    resources: Arc<RwLock<HashMap<String, (Resource, ResourceContentProvider)>>>,
    subscriptions: Arc<RwLock<HashMap<String, HashSet<String>>>>, // Maps resource URI to set of client IDs
    update_tx: broadcast::Sender<String>, // Channel for notifying resource updates
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        let (update_tx, _) = broadcast::channel(100);
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            update_tx,
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
    
    /// Get all registered resources
    pub async fn list_resources(&self) -> Vec<Resource> {
        let resources = self.resources.read().await;
        resources.values().map(|(resource, _)| resource.clone()).collect()
    }
    
    /// Get a specific resource's content
    pub async fn get_resource_content(&self, uri: &str) -> Result<Vec<ResourceContent>> {
        let resources = self.resources.read().await;
        let (_, content_provider) = resources
            .get(uri)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?;
        
        content_provider()
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
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
