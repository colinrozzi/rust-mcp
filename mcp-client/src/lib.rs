// mcp-client/src/lib.rs
pub mod client;
pub mod transport;

pub use client::{Client, ClientBuilder};
pub use transport::Transport;

pub use mcp_protocol;
