// mcp-protocol/src/version.rs
use serde::{Deserialize, Serialize};

/// Error returned when protocol versions don't match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMismatchError {
    pub supported: Vec<String>,
    pub requested: String,
}

/// Check if a protocol version is supported
pub fn is_supported_version(version: &str) -> bool {
    // For now, only support the current version
    version == crate::constants::PROTOCOL_VERSION
}

/// Get information for a version mismatch error
pub fn version_mismatch_error(requested: &str) -> VersionMismatchError {
    VersionMismatchError {
        supported: vec![crate::constants::PROTOCOL_VERSION.to_string()],
        requested: requested.to_string(),
    }
}
