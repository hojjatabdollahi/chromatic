// SPDX-License-Identifier: MPL-2.0

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

/// A single server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ServerConfig {
    /// Display name for this configuration
    pub name: String,
    /// ChromaDB server URL (e.g., http://localhost:8000)
    pub server_url: String,
    /// Authentication token for the ChromaDB server
    pub auth_token: String,
    /// Authentication header type: "authorization" (Bearer) or "x-chroma-token"
    pub auth_header_type: String,
    /// Tenant name (default: default_tenant)
    pub tenant: String,
    /// Database name (default: default_database)
    pub database: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: String::from("Local Server"),
            server_url: String::from("http://localhost:8000"),
            auth_token: String::new(),
            auth_header_type: String::from("authorization"),
            tenant: String::from("default_tenant"),
            database: String::from("default_database"),
        }
    }
}

impl ServerConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 3]
pub struct Config {
    /// List of server configurations
    pub servers: Vec<ServerConfig>,
    /// Index of the currently active server configuration
    pub active_server: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            servers: vec![ServerConfig::default()],
            active_server: 0,
        }
    }
}

impl Config {
    /// Get the currently active server configuration
    pub fn active_config(&self) -> &ServerConfig {
        // active_server should always be valid, but fallback to first if needed
        if self.active_server < self.servers.len() {
            &self.servers[self.active_server]
        } else if !self.servers.is_empty() {
            &self.servers[0]
        } else {
            // This should never happen as we always ensure at least one server
            panic!("No servers configured - this should never happen")
        }
    }

    /// Get the currently active server configuration (mutable)
    pub fn active_config_mut(&mut self) -> &mut ServerConfig {
        // Ensure we have at least one server
        if self.servers.is_empty() {
            self.servers.push(ServerConfig::default());
            self.active_server = 0;
        }
        // Ensure active_server is in bounds
        if self.active_server >= self.servers.len() {
            self.active_server = 0;
        }
        &mut self.servers[self.active_server]
    }

    /// Add a new server configuration and return its index
    pub fn add_server(&mut self, config: ServerConfig) -> usize {
        self.servers.push(config);
        self.servers.len() - 1
    }

    /// Remove a server configuration by index
    pub fn remove_server(&mut self, index: usize) -> bool {
        if self.servers.len() <= 1 {
            // Don't allow removing the last server
            return false;
        }
        if index < self.servers.len() {
            self.servers.remove(index);
            // Adjust active_server if needed
            if self.active_server >= self.servers.len() {
                self.active_server = self.servers.len() - 1;
            } else if self.active_server > index {
                self.active_server -= 1;
            }
            true
        } else {
            false
        }
    }

    /// Set the active server by index
    /// Note: Named with underscore suffix to avoid conflict with CosmicConfigEntry derive macro
    pub fn switch_active_server(&mut self, index: usize) -> bool {
        if index < self.servers.len() {
            self.active_server = index;
            true
        } else {
            false
        }
    }
}
