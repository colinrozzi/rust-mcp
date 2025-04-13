// mcp-server/src/sampling.rs
use anyhow::{anyhow, Result};
use mcp_protocol::types::sampling::{CreateMessageParams, CreateMessageResult, Message, MessageContent};
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
        let callback = {
            let cb = self.create_message_callback.lock().await;
            match &*cb {
                Some(cb) => cb.clone(),
                None => return Err(anyhow!("No create message callback registered")),
            }
        };
        
        // Call the callback
        callback(params)
    }
}

impl Default for SamplingManager {
    fn default() -> Self {
        Self::new()
    }
}
