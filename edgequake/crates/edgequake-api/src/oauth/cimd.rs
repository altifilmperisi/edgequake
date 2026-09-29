//! OAuth Client ID Metadata Documents (CIMD) fetch with SSRF limits.

use std::net::IpAddr;
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use url::Url;

use crate::error::ApiError;

use super::store;
use super::types::OAuthClientRegistration;
use crate::state::StorageRuntime;

const MAX_BODY_BYTES: usize = 64 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct CimdDocument {
    #[serde(default)]
    client_name: Option<String>,
    redirect_uris: Vec<String>,
    #[serde(default)]
    grant_types: Option<Vec<String>>,
    #[serde(default)]
    response_types: Option<Vec<String>>,
    #[serde(default)]
    token_endpoint_auth_method: Option<String>,
}

/// Resolve a client: DCR registry lookup, else CIMD HTTPS fetch when client_id is a URL.
pub async fn resolve_client(
    storage: &StorageRuntime,
    client_id: &str,
) -> Result<OAuthClientRegistration, ApiError> {
    if let Some(existing) = store::get_client(storage, client_id).await? {
        return Ok(existing);
    }
    if client_id.starts_with("https://") {
        let client = fetch_cimd(client_id).await?;
        store::store_client(storage, client.clone()).await?;
        return Ok(client);
    }
    Err(ApiError::BadRequest(format!(
        "unknown oauth client_id: {client_id}"
    )))
}

async fn fetch_cimd(client_id_url: &str) -> Result<OAuthClientRegistration, ApiError> {
    let url = Url::parse(client_id_url)
        .map_err(|e| ApiError::BadRequest(format!("invalid CIMD client_id URL: {e}")))?;
    if url.scheme() != "https" {
        return Err(ApiError::BadRequest(
            "CIMD client_id must be an https:// URL".into(),
        ));
    }
    if url.username() != "" || url.password().is_some() {
        return Err(ApiError::BadRequest(
            "CIMD client_id must not include credentials".into(),
        ));
    }
    reject_private_host(url.host_str().unwrap_or(""))?;

    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ApiError::Internal(format!("CIMD http client: {e}")))?;

    let response = client
        .get(url.as_str())
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| ApiError::BadRequest(format!("CIMD fetch failed: {e}")))?;

    if !response.status().is_success() {
        return Err(ApiError::BadRequest(format!(
            "CIMD fetch HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| ApiError::BadRequest(format!("CIMD body read failed: {e}")))?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err(ApiError::BadRequest("CIMD document too large".into()));
    }

    let doc: CimdDocument = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::BadRequest(format!("invalid CIMD JSON: {e}")))?;

    if doc.redirect_uris.is_empty() {
        return Err(ApiError::BadRequest(
            "CIMD document missing redirect_uris".into(),
        ));
    }
    for uri in &doc.redirect_uris {
        validate_redirect_uri(uri)?;
    }

    Ok(OAuthClientRegistration {
        client_id: client_id_url.to_string(),
        client_name: doc.client_name,
        redirect_uris: doc.redirect_uris,
        grant_types: doc
            .grant_types
            .unwrap_or_else(|| vec!["authorization_code".into(), "refresh_token".into()]),
        response_types: doc.response_types.unwrap_or_else(|| vec!["code".into()]),
        token_endpoint_auth_method: doc
            .token_endpoint_auth_method
            .or_else(|| Some("none".into())),
        is_cimd: true,
        created_at: Utc::now(),
    })
}

pub fn validate_redirect_uri(uri: &str) -> Result<(), ApiError> {
    let url =
        Url::parse(uri).map_err(|e| ApiError::BadRequest(format!("invalid redirect_uri: {e}")))?;
    match url.scheme() {
        "https" => Ok(()),
        "http" => {
            let host = url.host_str().unwrap_or("");
            if host == "127.0.0.1" || host == "localhost" || host == "[::1]" || host == "::1" {
                Ok(())
            } else {
                Err(ApiError::BadRequest(
                    "http redirect_uri is only allowed for loopback hosts".into(),
                ))
            }
        }
        other => Err(ApiError::BadRequest(format!(
            "unsupported redirect_uri scheme: {other}"
        ))),
    }
}

pub fn redirect_uri_allowed(client: &OAuthClientRegistration, redirect_uri: &str) -> bool {
    client.redirect_uris.iter().any(|u| u == redirect_uri)
}

fn reject_private_host(host: &str) -> Result<(), ApiError> {
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
        || host_lower == "metadata.google.internal"
    {
        return Err(ApiError::BadRequest(
            "CIMD host is not allowed (private/local)".into(),
        ));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(ip) {
            return Err(ApiError::BadRequest(
                "CIMD host resolves to a private IP".into(),
            ));
        }
    }
    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_http_non_loopback_redirect() {
        assert!(validate_redirect_uri("https://app.example.com/cb").is_ok());
        assert!(validate_redirect_uri("http://127.0.0.1:5555/cb").is_ok());
        assert!(validate_redirect_uri("http://localhost/cb").is_ok());
        assert!(validate_redirect_uri("http://evil.example/cb").is_err());
    }

    #[test]
    fn reject_private_cimd_hosts() {
        assert!(reject_private_host("claude.ai").is_ok());
        assert!(reject_private_host("localhost").is_err());
        assert!(reject_private_host("127.0.0.1").is_err());
        assert!(reject_private_host("10.0.0.1").is_err());
        assert!(reject_private_host("metadata.google.internal").is_err());
    }

    #[tokio::test]
    async fn resolve_client_rejects_http_cimd_url() {
        let storage = crate::state::AppState::test_state().storage;
        let err = resolve_client(&storage, "http://evil.example/client.json")
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown oauth client_id") || msg.contains("https"),
            "{msg}"
        );
    }
}
