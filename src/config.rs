// SPDX-License-Identifier: MPL-2.0

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    /// ChromaDB server URL (e.g., http://localhost:8000)
    pub server_url: String,
    /// Authentication token for the ChromaDB server
    pub auth_token: String,
    /// Authentication header type: "authorization" (Bearer) or "x-chroma-token"
    pub auth_header_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: String::from("http://localhost:8000"),
            auth_token: String::new(),
            auth_header_type: String::from("authorization"),
        }
    }
}
