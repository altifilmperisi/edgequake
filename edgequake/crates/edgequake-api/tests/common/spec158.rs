//! SPEC-158 e2e harness: one wiremock IdP + in-memory `AppState`, driven through the real router.
//!
//! The token endpoint serves whatever id_token the test last armed (`arm_id_token`), so a single
//! mounted mock supports many logins (DRY — every EC-158 e2e uses this).

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use base64::Engine;
use edgequake_api::services::federation::policy::FederationPolicy;
use edgequake_api::services::oidc_flow::OidcFlowService;
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::OidcConfig;
use edgequake_core::Tenant;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

pub const CLIENT_ID: &str = "spec158-client";
pub const CLIENT_SECRET: &str = "spec158-client-secret-0123456789abcdef";
pub const SUCCESS_URL: &str = "http://localhost:3000/auth/callback";

/// Token endpoint responder returning the currently armed id_token.
#[derive(Clone, Default)]
pub struct ArmedToken(pub Arc<Mutex<String>>);

impl Respond for ArmedToken {
    fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "idp-access",
            "token_type": "Bearer",
            "id_token": self.0.lock().unwrap().clone(),
        }))
    }
}

/// Mutable JWKS document (for rotation tests).
#[derive(Clone)]
pub struct ArmedJwks(pub Arc<Mutex<Value>>);

impl Respond for ArmedJwks {
    fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(self.0.lock().unwrap().clone())
    }
}

pub struct Sso {
    pub mock: MockServer,
    pub state: AppState,
    pub token: ArmedToken,
    pub jwks: ArmedJwks,
}

pub fn policy() -> FederationPolicy {
    FederationPolicy {
        slug: "idp".into(),
        ..FederationPolicy::default()
    }
}

fn server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".into(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

impl Sso {
    /// HS256 IdP (id_token signed with the client secret).
    pub async fn start(policy: FederationPolicy, success_redirect: Option<&str>) -> Self {
        Self::start_with_alg(policy, success_redirect, "HS256", json!({ "keys": [] })).await
    }

    pub async fn start_with_alg(
        policy: FederationPolicy,
        success_redirect: Option<&str>,
        alg: &str,
        jwks: Value,
    ) -> Self {
        let mock = MockServer::start().await;
        let token = ArmedToken::default();
        let jwks = ArmedJwks(Arc::new(Mutex::new(jwks)));
        mount_idp(&mock, alg, token.clone(), jwks.clone()).await;

        let mut state = AppState::test_state();
        let cfg = OidcConfig {
            enabled: true,
            issuer_url: mock.uri(),
            client_id: CLIENT_ID.into(),
            client_secret: Some(CLIENT_SECRET.into()),
            redirect_uri: "http://localhost/api/v1/auth/oidc/callback".into(),
            success_redirect_url: success_redirect.map(str::to_string),
        };
        state.auth.oidc_config = cfg.clone();
        state.auth.oidc_service = Some(Arc::new(OidcFlowService::with_policy(cfg, policy)));
        Self {
            mock,
            state,
            token,
            jwks,
        }
    }

    /// Turn the API into an authenticated deployment (auth enforced).
    pub fn enforce_auth(&mut self) {
        self.state.auth.config.auth_enabled = true;
        self.state.auth.config.dev_mode = false;
        self.state.security.strict_tenant_bind = true;
    }

    pub fn app(&self) -> axum::Router {
        Server::new(server_config(), self.state.clone()).build_router()
    }

    pub fn router(state: AppState) -> axum::Router {
        Server::new(server_config(), state).build_router()
    }

    pub async fn get(&self, uri: &str) -> Response {
        self.app()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    pub async fn post_json(&self, uri: &str, body: Value, bearer: Option<&str>) -> Response {
        let mut req = Request::post(uri).header(header::CONTENT_TYPE, "application/json");
        if let Some(t) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        self.app()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap()
    }

    pub async fn request(&self, method: &str, uri: &str, bearer: &str) -> Response {
        self.app()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// `/auth/oidc/login?<query>` → (state, nonce) from the authorize redirect.
    pub async fn begin(&self, query: &str) -> (String, String) {
        let res = self.get(&format!("/api/v1/auth/oidc/login?{query}")).await;
        assert_eq!(
            res.status(),
            StatusCode::SEE_OTHER,
            "login must redirect to the IdP"
        );
        let loc = location(&res);
        let url = url::Url::parse(&loc).expect("authorize url");
        let q = |k: &str| {
            url.query_pairs()
                .find(|(a, _)| a == k)
                .map(|(_, v)| v.to_string())
        };
        (q("state").expect("state"), q("nonce").expect("nonce"))
    }

    pub fn id_token(&self, nonce: &str, subject: &str, extra: Value) -> String {
        sign_hs256(&self.mock.uri(), nonce, subject, extra)
    }

    pub fn arm(&self, id_token: String) {
        *self.token.0.lock().unwrap() = id_token;
    }

    pub async fn callback(&self, state_param: &str) -> Response {
        self.get(&format!(
            "/api/v1/auth/oidc/callback?code=c&state={}",
            urlencoding::encode(state_param)
        ))
        .await
    }

    /// Full browser flow with an HS256 id_token. `login_query` e.g. `org=acme`.
    pub async fn login(&self, login_query: &str, subject: &str, extra: Value) -> Response {
        let (st, nonce) = self.begin(login_query).await;
        self.arm(self.id_token(&nonce, subject, extra));
        self.callback(&st).await
    }

    /// Point the configured issuer at a host that will not answer discovery (EC-158-16).
    pub fn with_dead_issuer(&mut self) {
        let Some(service) = self.state.auth.oidc_service.as_ref() else {
            panic!("OIDC service");
        };
        let mut cfg = service.config().clone();
        cfg.issuer_url = "http://127.0.0.1:1".into();
        let policy = service.policy().clone();
        self.state.auth.oidc_config = cfg.clone();
        self.state.auth.oidc_service = Some(Arc::new(OidcFlowService::with_policy(cfg, policy)));
    }

    /// Local (password) user via the public registration route; returns its `user_id`.
    pub async fn create_local_user(&self, username: &str, email: &str, password: &str) -> String {
        let res = self
            .post_json(
                "/api/v1/users",
                json!({ "username": username, "email": email, "password": password }),
                None,
            )
            .await;
        assert_eq!(res.status(), StatusCode::CREATED, "local user create");
        json_body(res).await["user"]["user_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    /// Platform-admin access token minted with the server's JWT service (no user row needed).
    pub fn admin_token(&self) -> String {
        self.state
            .auth
            .jwt
            .generate_token(Uuid::new_v4(), edgequake_auth::Role::Admin)
            .expect("admin token")
    }

    pub async fn password_login(&self, username: &str, password: &str) -> Response {
        self.post_json(
            "/api/v1/auth/login",
            json!({ "username": username, "password": password }),
            None,
        )
        .await
    }

    pub async fn create_tenant(&self, name: &str, slug: &str) -> Tenant {
        let t = Tenant::new(name, slug);
        self.state
            .workspace_service
            .create_tenant(t)
            .await
            .expect("create tenant")
    }
}

async fn mount_idp(mock: &MockServer, alg: &str, token: ArmedToken, jwks: ArmedJwks) {
    let issuer = mock.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}/authorize"),
            "token_endpoint": format!("{issuer}/token"),
            "jwks_uri": format!("{issuer}/jwks"),
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": [alg],
            "scopes_supported": ["openid", "email", "profile"],
            "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
        })))
        .mount(mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(jwks)
        .mount(mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(token)
        .mount(mock)
        .await;
}

pub fn base_claims(issuer: &str, nonce: &str, subject: &str, extra: Value) -> Value {
    let now = chrono::Utc::now().timestamp();
    let mut claims = json!({
        "iss": issuer, "sub": subject, "aud": CLIENT_ID, "nonce": nonce,
        "exp": now + 3600, "iat": now,
    });
    if let (Some(base), Some(extra)) = (claims.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    claims
}

pub fn sign_hs256(issuer: &str, nonce: &str, subject: &str, extra: Value) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &base_claims(issuer, nonce, subject, extra),
        &EncodingKey::from_secret(CLIENT_SECRET.as_bytes()),
    )
    .expect("sign")
}

pub fn sign_rs256(
    kid: &str,
    pem: &str,
    issuer: &str,
    nonce: &str,
    subject: &str,
    extra: Value,
) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());
    encode(
        &header,
        &base_claims(issuer, nonce, subject, extra),
        &EncodingKey::from_rsa_pem(pem.as_bytes()).expect("rsa pem"),
    )
    .expect("sign rs256")
}

pub fn location(res: &Response) -> String {
    res.headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .expect("Location header")
        .to_string()
}

pub async fn json_body(res: Response) -> Value {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }))
}

/// Payload of a JWT without verification (tests only).
pub fn jwt_payload(token: &str) -> Value {
    let part = token.split('.').nth(1).expect("jwt payload");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .expect("b64");
    serde_json::from_slice(&bytes).expect("json")
}

/// Query param `k` of a redirect Location.
pub fn query_param(location: &str, k: &str) -> Option<String> {
    url::Url::parse(location)
        .ok()?
        .query_pairs()
        .find(|(a, _)| a == k)
        .map(|(_, v)| v.to_string())
}

pub fn set_cookie(res: &Response) -> Option<String> {
    res.headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}
