//! SPEC-154 Wave 4 / gap-close — web refresh rotation (EC-154-08 / EC-154-24).
//!
//! Memory path: `AppState::test_state()` (no pool).
//! Production path: PG pool + `PostgresSessionStore` — must still use family consume
//! (`take_web_refresh_pg`), not SessionStore approximate rotate.

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use edgequake_api::{AppState, Server, ServerConfig};

async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn build_app(state: AppState) -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: true,
        },
        state,
    )
    .build_router()
}

fn auth_memory_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.api_keys = vec!["master-refresh-test-key".to_string()];
    state.auth.config.master_api_key = Some("master-refresh-test-key".to_string());
    state
}

#[cfg(feature = "postgres")]
fn auth_pg_production_like(pool: sqlx::PgPool) -> AppState {
    use edgequake_api::services::postgres_session_store::PostgresSessionStore;
    use std::sync::Arc;

    let mut state = AppState::test_state_with_pg_pool(pool.clone());
    state.auth.config.auth_enabled = true;
    state.auth.config.api_keys = vec!["master-refresh-test-key".to_string()];
    state.auth.config.master_api_key = Some("master-refresh-test-key".to_string());
    state.security.kv_identity_mirror = false;
    // Mimic serve-time operational_stores: SessionStore port is set.
    state.operational_stores.sessions = Some(Arc::new(PostgresSessionStore::new(pool)));
    state
}

#[cfg(feature = "postgres")]
async fn connect_and_bootstrap() -> Option<sqlx::PgPool> {
    use edgequake_api::state::migration_bootstrap::run_postgres_migrations;
    use sqlx::postgres::PgPoolOptions;

    let database_url = common::spec013_postgres::try_database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect postgres");
    run_postgres_migrations(&pool)
        .await
        .expect("bootstrap migrations");
    Some(pool)
}

async fn bootstrap_user(app: &axum::Router, username: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-refresh-test-key")
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": "user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.status() == StatusCode::CREATED || response.status() == StatusCode::OK,
        "create user status {}",
        response.status()
    );
}

async fn login(app: &axum::Router, username: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": username,
                        "password": "SecurePass123!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    parse_json(response).await
}

async fn assert_rotate_and_reuse_revokes_family(app: &axum::Router, username: &str) {
    bootstrap_user(app, username).await;

    let login_body = login(app, username).await;
    let refresh1 = login_body["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_string();
    assert!(login_body["expires_in"].as_i64().unwrap_or(0) <= 900);

    let rot = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rot.status(), StatusCode::OK);
    let rot_body = parse_json(rot).await;
    let refresh2 = rot_body["refresh_token"]
        .as_str()
        .expect("rotated refresh")
        .to_string();
    assert_ne!(refresh1, refresh2);

    let reuse = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reuse.status(), StatusCode::UNAUTHORIZED);

    let after = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh2 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        after.status(),
        StatusCode::UNAUTHORIZED,
        "family must be revoked after reuse"
    );
}

async fn assert_concurrent_refresh_one_winner(app: &axum::Router, username: &str) {
    bootstrap_user(app, username).await;
    let login_body = login(app, username).await;
    let refresh = login_body["refresh_token"].as_str().unwrap().to_string();

    let req = || {
        Request::builder()
            .method("POST")
            .uri("/api/v1/auth/refresh")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "refresh_token": refresh }).to_string(),
            ))
            .unwrap()
    };

    let (a, b) = tokio::join!(app.clone().oneshot(req()), app.clone().oneshot(req()));
    let sa = a.unwrap().status();
    let sb = b.unwrap().status();
    let oks = [sa, sb]
        .into_iter()
        .filter(|s| *s == StatusCode::OK)
        .count();
    let fails = [sa, sb]
        .into_iter()
        .filter(|s| *s == StatusCode::UNAUTHORIZED)
        .count();
    assert_eq!(
        oks, 1,
        "exactly one concurrent refresh succeeds ({sa:?},{sb:?})"
    );
    assert_eq!(fails, 1, "loser treated as reuse ({sa:?},{sb:?})");
}

#[tokio::test]
async fn ec_154_08_refresh_rotates_and_reuse_revokes_family_memory() {
    let state = auth_memory_state();
    state.initialize_defaults().await.expect("defaults");
    let app = build_app(state);
    let username = format!("w4_rot_mem_{}", &Uuid::new_v4().to_string()[..8]);
    assert_rotate_and_reuse_revokes_family(&app, &username).await;
}

#[tokio::test]
async fn ec_154_24_concurrent_refresh_one_winner_memory() {
    let state = auth_memory_state();
    state.initialize_defaults().await.expect("defaults");
    let app = build_app(state);
    let username = format!("w4_race_mem_{}", &Uuid::new_v4().to_string()[..8]);
    assert_concurrent_refresh_one_winner(&app, &username).await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn ec_154_08_refresh_rotates_and_reuse_revokes_family_pg() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP ec_154_08_refresh_rotates_and_reuse_revokes_family_pg: DATABASE_URL not set"
            );
            return;
        }
    };

    let state = auth_pg_production_like(pool);
    state.initialize_defaults().await.expect("defaults");
    let app = build_app(state);
    let username = format!("w4_rot_pg_{}", &Uuid::new_v4().to_string()[..8]);
    assert_rotate_and_reuse_revokes_family(&app, &username).await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn ec_154_24_concurrent_refresh_one_winner_pg() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP ec_154_24_concurrent_refresh_one_winner_pg: DATABASE_URL not set");
            return;
        }
    };

    let state = auth_pg_production_like(pool);
    state.initialize_defaults().await.expect("defaults");
    let app = build_app(state);
    let username = format!("w4_race_pg_{}", &Uuid::new_v4().to_string()[..8]);
    assert_concurrent_refresh_one_winner(&app, &username).await;
}
