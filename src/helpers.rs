// SPDX-License-Identifier: MPL-2.0

//! Async helper functions for the Chromatic application.
//! These functions handle ChromaDB API interactions.

use crate::api::{ChromaClient, Collection, Document, ServerInfo};

/// Helper to create a client with auto-detected API version
pub async fn create_client(url: &str, token: &str, auth_header_type: &str) -> Result<ChromaClient, String> {
    let api_version = ChromaClient::detect_api_version(url, token, auth_header_type)
        .await
        .map_err(|e| e.to_string())?;
    ChromaClient::new(url, token, auth_header_type, api_version).map_err(|e| e.to_string())
}

/// Test connection to ChromaDB server
pub async fn test_connection(url: &str, token: &str, auth_header_type: &str) -> Result<(), String> {
    // Just detect API version - if it succeeds, connection works
    let _api_version = ChromaClient::detect_api_version(url, token, auth_header_type)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetch server information
pub async fn fetch_server_info(url: &str, token: &str, auth_header_type: &str) -> Result<ServerInfo, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.get_server_info().await.map_err(|e| e.to_string())
}

/// Validate tenant and database, returning (tenant_exists, database_exists) on failure
pub async fn validate_tenant_database(
    url: &str,
    token: &str,
    auth_header_type: &str,
    tenant: &str,
    database: &str,
) -> Result<(), (bool, bool)> {
    let client = create_client(url, token, auth_header_type).await.map_err(|_| (false, false))?;
    let (tenant_exists, database_exists) = client.check_tenant_database_status(tenant, database).await;
    if tenant_exists && database_exists {
        Ok(())
    } else {
        Err((tenant_exists, database_exists))
    }
}

/// Create missing tenant and/or database
pub async fn create_missing_resources(
    url: &str,
    token: &str,
    auth_header_type: &str,
    tenant: &str,
    database: &str,
    tenant_exists: bool,
    database_exists: bool,
) -> Result<(), String> {
    let client = create_client(url, token, auth_header_type).await?;
    
    // Create tenant if needed
    if !tenant_exists {
        client.create_tenant(tenant).await.map_err(|e| e.to_string())?;
    }
    
    // Create database if needed
    if !database_exists {
        client.create_database(tenant, database).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

/// Fetch available databases for a tenant
pub async fn fetch_databases(
    url: &str,
    token: &str,
    auth_header_type: &str,
    tenant: &str,
) -> Result<Vec<String>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    let databases = client.list_databases(tenant).await.map_err(|e| e.to_string())?;
    Ok(databases.into_iter().map(|db| db.name).collect())
}

/// Fetch available tenants
pub async fn fetch_tenants(
    url: &str,
    token: &str,
    auth_header_type: &str,
) -> Result<Vec<String>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    let tenants = client.list_tenants().await.map_err(|e| e.to_string())?;
    Ok(tenants.into_iter().map(|t| t.name).collect())
}

/// Fetch collections from the server
pub async fn fetch_collections(
    url: &str,
    token: &str,
    auth_header_type: &str,
    tenant: &str,
    database: &str,
) -> Result<Vec<Collection>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.list_collections(tenant, database).await.map_err(|e| e.to_string())
}

/// Fetch documents from a collection
pub async fn fetch_documents(
    url: &str,
    token: &str,
    auth_header_type: &str,
    collection_id: &str,
    tenant: &str,
    database: &str,
) -> Result<Vec<Document>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.get_documents(collection_id, Some(100), None, tenant, database).await.map_err(|e| e.to_string())
}
