// mcp-server/src/lib.rs
pub mod server;
pub mod transport;
pub mod tools;
pub mod resources;

pub use server::{Server, ServerBuilder};
pub use transport::Transport;
