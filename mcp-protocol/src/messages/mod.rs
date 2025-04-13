// mcp-protocol/src/messages/mod.rs
pub mod base;
pub mod lifecycle;

pub use base::JsonRpcMessage;
pub use lifecycle::*;
