// SPDX-License-Identifier: MPL-2.0

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version for ChromaDB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiVersion {
    V1,
    #[default]
    V2,
}

impl ApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChromaClient {
    client: reqwest::Client,
    base_url: String,
    api_version: ApiVersion,
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

/// Server info combining version and heartbeat
#[derive(Debug, Clone, Default)]
pub struct ServerInfo {
    pub version: String,
    pub heartbeat_ns: i64,
    pub api_version: String,
}

/// Tenant information from ChromaDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub name: String,
}

/// Database information from ChromaDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub id: String,
    pub name: String,
    pub tenant: String,
}

/// A document stored in a collection
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Document {
    pub id: String,
    #[serde(default)]
    pub document: Option<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request body for getting documents
#[derive(Debug, Clone, Serialize)]
pub struct GetDocumentsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    pub include: Vec<String>,
}

/// Response from getting documents
#[derive(Debug, Clone, Deserialize)]
pub struct GetDocumentsResponse {
    pub ids: Vec<String>,
    #[serde(default)]
    pub documents: Option<Vec<Option<String>>>,
    #[serde(default)]
    pub metadatas: Option<Vec<Option<HashMap<String, serde_json::Value>>>>,
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
    pub fn new(base_url: &str, auth_token: &str, auth_header_type: &str, api_version: ApiVersion) -> Result<Self, ChromaError> {
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

        Ok(Self { client, base_url, api_version })
    }

    /// Detect API version by trying v2 first, then falling back to v1
    pub async fn detect_api_version(base_url: &str, auth_token: &str, auth_header_type: &str) -> Result<ApiVersion, ChromaError> {
        // Try v2 first
        let client_v2 = Self::new(base_url, auth_token, auth_header_type, ApiVersion::V2)?;
        if client_v2.heartbeat().await.is_ok() {
            return Ok(ApiVersion::V2);
        }

        // Try v1
        let client_v1 = Self::new(base_url, auth_token, auth_header_type, ApiVersion::V1)?;
        if client_v1.heartbeat().await.is_ok() {
            return Ok(ApiVersion::V1);
        }

        Err(ChromaError::ConnectionFailed("Could not connect to server with v1 or v2 API".to_string()))
    }

    /// Get the API version prefix
    fn api_prefix(&self) -> String {
        format!("{}/api/{}", self.base_url, self.api_version.as_str())
    }

    /// Check server health with heartbeat endpoint
    pub async fn heartbeat(&self) -> Result<HeartbeatResponse, ChromaError> {
        let url = format!("{}/heartbeat", self.api_prefix());
        
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

    /// Get server version
    pub async fn get_version(&self) -> Result<String, ChromaError> {
        let url = format!("{}/version", self.api_prefix());
        
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

        // Version endpoint returns a plain string (with quotes)
        let version: String = response
            .json()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))?;
        
        Ok(version)
    }

    /// Get combined server info (version + heartbeat)
    pub async fn get_server_info(&self) -> Result<ServerInfo, ChromaError> {
        let version = self.get_version().await?;
        let heartbeat = self.heartbeat().await?;
        
        Ok(ServerInfo {
            version,
            heartbeat_ns: heartbeat.nanosecond_heartbeat,
            api_version: self.api_version.as_str().to_string(),
        })
    }

    /// Check if a tenant exists
    pub async fn get_tenant(&self, tenant: &str) -> Result<Tenant, ChromaError> {
        let url = format!("{}/tenants/{}", self.api_prefix(), tenant);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Tenant '{}' not found: {} - {}",
                tenant, status, body
            )));
        }

        response
            .json::<Tenant>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }

    /// Check if a database exists within a tenant
    pub async fn get_database(&self, tenant: &str, database: &str) -> Result<Database, ChromaError> {
        let url = match self.api_version {
            ApiVersion::V1 => format!(
                "{}/databases/{}?tenant={}",
                self.api_prefix(), database, tenant
            ),
            ApiVersion::V2 => format!(
                "{}/tenants/{}/databases/{}",
                self.api_prefix(), tenant, database
            ),
        };
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Database '{}' not found in tenant '{}': {} - {}",
                database, tenant, status, body
            )));
        }

        response
            .json::<Database>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }

    /// Validate that both tenant and database exist
    pub async fn validate_tenant_database(&self, tenant: &str, database: &str) -> Result<(), ChromaError> {
        self.get_tenant(tenant).await?;
        self.get_database(tenant, database).await?;
        Ok(())
    }

    /// List all databases for a tenant
    pub async fn list_databases(&self, tenant: &str) -> Result<Vec<Database>, ChromaError> {
        let url = match self.api_version {
            ApiVersion::V1 => format!(
                "{}/databases?tenant={}",
                self.api_prefix(), tenant
            ),
            ApiVersion::V2 => format!(
                "{}/tenants/{}/databases",
                self.api_prefix(), tenant
            ),
        };
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Failed to list databases for tenant '{}': {} - {}",
                tenant, status, body
            )));
        }

        response
            .json::<Vec<Database>>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }

    /// Create a new tenant
    pub async fn create_tenant(&self, tenant: &str) -> Result<Tenant, ChromaError> {
        let url = format!("{}/tenants", self.api_prefix());
        
        let body = serde_json::json!({ "name": tenant });
        
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Failed to create tenant '{}': {} - {}",
                tenant, status, body
            )));
        }

        // Return the tenant info - some servers return empty response on create
        Ok(Tenant { name: tenant.to_string() })
    }

    /// Create a new database within a tenant
    pub async fn create_database(&self, tenant: &str, database: &str) -> Result<Database, ChromaError> {
        let url = match self.api_version {
            ApiVersion::V1 => format!(
                "{}/databases?tenant={}",
                self.api_prefix(), tenant
            ),
            ApiVersion::V2 => format!(
                "{}/tenants/{}/databases",
                self.api_prefix(), tenant
            ),
        };
        
        let body = serde_json::json!({ "name": database });
        
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Failed to create database '{}' in tenant '{}': {} - {}",
                database, tenant, status, body
            )));
        }

        // Return the database info
        Ok(Database { 
            id: String::new(), // ID may be assigned by server
            name: database.to_string(),
            tenant: tenant.to_string(),
        })
    }

    /// Check what's missing (tenant, database, or both) and return detailed info
    pub async fn check_tenant_database_status(&self, tenant: &str, database: &str) -> (bool, bool) {
        let tenant_exists = self.get_tenant(tenant).await.is_ok();
        let database_exists = if tenant_exists {
            self.get_database(tenant, database).await.is_ok()
        } else {
            false
        };
        (tenant_exists, database_exists)
    }

    /// List all collections for a specific tenant and database
    pub async fn list_collections(&self, tenant: &str, database: &str) -> Result<Vec<Collection>, ChromaError> {
        let url = match self.api_version {
            ApiVersion::V1 => format!(
                "{}/databases/{}/collections?tenant={}",
                self.api_prefix(), database, tenant
            ),
            ApiVersion::V2 => format!(
                "{}/tenants/{}/databases/{}/collections",
                self.api_prefix(), tenant, database
            ),
        };
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Server returned status: {} - {}",
                status, body
            )));
        }

        response
            .json::<Vec<Collection>>()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))
    }

    /// Get documents from a collection
    pub async fn get_documents(
        &self,
        collection_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
        tenant: &str,
        database: &str,
    ) -> Result<Vec<Document>, ChromaError> {
        let url = match self.api_version {
            ApiVersion::V1 => format!(
                "{}/databases/{}/collections/{}/get?tenant={}",
                self.api_prefix(), database, collection_id, tenant
            ),
            ApiVersion::V2 => format!(
                "{}/tenants/{}/databases/{}/collections/{}/get",
                self.api_prefix(), tenant, database, collection_id
            ),
        };
        
        let request = GetDocumentsRequest {
            ids: None,
            limit: limit.or(Some(100)), // Default limit
            offset,
            include: vec!["documents".to_string(), "metadatas".to_string()],
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ChromaError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ChromaError::RequestFailed(format!(
                "Server returned status: {} - {}",
                status, body
            )));
        }

        let result: GetDocumentsResponse = response
            .json()
            .await
            .map_err(|e| ChromaError::InvalidResponse(e.to_string()))?;

        // Convert the response into a Vec<Document>
        let documents: Vec<Document> = result
            .ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let document = result
                    .documents
                    .as_ref()
                    .and_then(|docs| docs.get(i).cloned().flatten());
                let metadata = result
                    .metadatas
                    .as_ref()
                    .and_then(|metas| metas.get(i).cloned().flatten());
                Document {
                    id,
                    document,
                    metadata,
                }
            })
            .collect();

        Ok(documents)
    }
}
