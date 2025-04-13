// mcp-server/src/sampling.rs
use anyhow::{anyhow, Result};
use mcp_protocol::types::sampling::{CreateMessageParams, CreateMessageResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Callback type for the sampling create message
pub type CreateMessageCallback = Box<dyn Fn(&CreateMessageParams) -> Result<CreateMessageResult> + Send + Sync>;

/// Sampling manager that handles requests for LLM sampling
pub struct SamplingManager {
    create_message_callback: Arc<Mutex<Option<CreateMessageCallback>>>,
}

impl SamplingManager {
    /// Create a new sampling manager
    pub fn new() -> Self {
        Self {
            create_message_callback: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Register a create message callback
    pub fn register_create_message_callback(&self, callback: CreateMessageCallback) {
        let mut cb = self.create_message_callback.blocking_lock();
        *cb = Some(callback);
    }
    
    /// Create a message using the registered callback
    pub async fn create_message(&self, params: &CreateMessageParams) -> Result<CreateMessageResult> {
        // Get the callback and invoke it with the lock
        let cb = self.create_message_callback.lock().await;
        if cb.is_none() {
            return Err(anyhow!("No create message callback registered"));
        }
        
        // We can't clone the Box<dyn Fn...>, so we'll invoke it while we have the lock
        let callback_ref = cb.as_ref().unwrap();
        callback_ref(params)
    }
}

impl Default for SamplingManager {
    fn default() -> Self {
        Self::new()
    }
}
