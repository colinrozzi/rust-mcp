// mcp-server/src/lib.rs
pub mod server;
pub mod transport;
pub mod tools;
pub mod resources;
mod completion;
mod resource_extensions;

pub use server::{Server, ServerBuilder};
pub use transport::Transport;
