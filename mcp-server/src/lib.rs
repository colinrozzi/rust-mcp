// mcp-server/src/lib.rs
pub mod server;
pub mod transport;
pub mod tools;
pub mod resources;
pub mod prompts;
mod completion;
mod resource_extensions;
mod server_prompts;

pub use server::{Server, ServerBuilder};
pub use transport::Transport;
