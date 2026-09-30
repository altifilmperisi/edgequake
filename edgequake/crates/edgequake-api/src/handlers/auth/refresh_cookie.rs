//! HttpOnly refresh-token cookie helpers (SPEC-154 Wave 5).

use axum::http::{header, HeaderMap, HeaderValue};

/// Cookie name for web refresh tokens.
pub(crate) const REFRESH_COOKIE_NAME: &str = "eq_refresh";

/// Path scoped to auth endpoints only.
pub(crate) const REFRESH_COOKIE_PATH: &str = "/api/v1/auth";

/// Build `Set-Cookie` for a refresh token.
pub(crate) fn set_refresh_cookie_header(token: &str, secure: bool) -> HeaderValue {
    let mut value = format!(
        "{REFRESH_COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path={REFRESH_COOKIE_PATH}; Max-Age=2592000"
    );
    if secure {
        value.push_str("; Secure");
    }
    HeaderValue::from_str(&value)
        .unwrap_or_else(|_| HeaderValue::from_static("eq_refresh=; Max-Age=0"))
}

/// Clear the refresh cookie.
pub(crate) fn clear_refresh_cookie_header(secure: bool) -> HeaderValue {
    let mut value = format!(
        "{REFRESH_COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path={REFRESH_COOKIE_PATH}; Max-Age=0"
    );
    if secure {
        value.push_str("; Secure");
    }
    HeaderValue::from_str(&value)
        .unwrap_or_else(|_| HeaderValue::from_static("eq_refresh=; Max-Age=0"))
}

/// Prefer body refresh token; fall back to `eq_refresh` cookie.
pub(crate) fn resolve_refresh_token(
    body_token: Option<&str>,
    headers: &HeaderMap,
) -> Option<String> {
    if let Some(t) = body_token.map(str::trim).filter(|t| !t.is_empty()) {
        return Some(t.to_string());
    }
    read_refresh_cookie(headers)
}

/// Browser / SPA clients: omit refresh_token from JSON (cookie is SSOT — SPEC-154).
pub(crate) fn omit_refresh_in_json_body(headers: &HeaderMap) -> bool {
    if headers.get("sec-fetch-mode").is_some() || headers.get("sec-fetch-dest").is_some() {
        return true;
    }
    if headers
        .get("x-edgequake-client")
        .and_then(|v| v.to_str().ok())
        == Some("webui")
    {
        return true;
    }
    false
}

/// Body field for refresh when not omitted for SPA.
pub(crate) fn refresh_token_for_json_body(headers: &HeaderMap, token: &str) -> Option<String> {
    if omit_refresh_in_json_body(headers) {
        None
    } else {
        Some(token.to_string())
    }
}

pub(crate) fn read_refresh_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&format!("{REFRESH_COOKIE_NAME}="))
            .map(|v| {
                urlencoding::decode(v)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| v.to_string())
            })
            .filter(|v| !v.is_empty())
    })
}

/// True when the request should use the Secure cookie flag.
pub(crate) fn cookie_secure_from_headers(headers: &HeaderMap) -> bool {
    if let Ok(v) = std::env::var("EDGEQUAKE_COOKIE_SECURE") {
        let t = v.trim().to_ascii_lowercase();
        if matches!(t.as_str(), "1" | "true" | "yes" | "on") {
            return true;
        }
        if matches!(t.as_str(), "0" | "false" | "no" | "off") {
            return false;
        }
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|p| p.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}
