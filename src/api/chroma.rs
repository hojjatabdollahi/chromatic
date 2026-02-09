// SPDX-License-Identifier: MPL-2.0

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChromaClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    #[serde(rename = "nanosecond heartbeat")]
    pub nanosecond_heartbeat: i64,
}

#[derive(Debug, Clone)]
pub enum ChromaError {
    ConnectionFailed(String),
    RequestFailed(String),
    InvalidResponse(String),
}

impl std::fmt::Display for ChromaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChromaError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ChromaError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            ChromaError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl ChromaClient {
    /// Create a new ChromaDB client
    /// auth_header_type: "authorization" for Bearer token, "x-chroma-token" for X-Chroma-Token header
    pub fn new(base_url: &str, auth_token: &str, auth_header_type: &str) -> Result<Self, ChromaError> {
        let mut headers = HeaderMap::new();
        
        if !auth_token.is_empty() {
            match auth_header_type {
                "x-chroma-token" => {
                    // Use X-Chroma-Token header (token without Bearer prefix)
                    let header_name = HeaderName::from_static("x-chroma-token");
                    let auth_value = HeaderValue::from_str(auth_token)
                        .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;
                    headers.insert(header_name, auth_value);
                }
                _ => {
                    // Default: Use Authorization: Bearer header
                    let auth_value = HeaderValue::from_str(&format!("Bearer {}", auth_token))
                        .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;
                    headers.insert(AUTHORIZATION, auth_value);
                }
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        // Normalize base URL (remove trailing slash)
        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { client, base_url })
    }

    /// Check server health with heartbeat endpoint
    pub async fn heartbeat(&self) -> Result<HeartbeatResponse, ChromaError> {
        let url = format!("{}/api/v1/heartbeat", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChromaError::RequestFailed(format!(
                "Server returned status: {}",
                response.status()
            )));
        }

        response
            .json::<HeartbeatResponse>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<Collection>, ChromaError> {
        let url = format!("{}/api/v1/collections", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChromaError::RequestFailed(format!(
                "Server returned status: {}",
                response.status()
            )));
        }

        response
            .json::<Vec<Collection>>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }
}
