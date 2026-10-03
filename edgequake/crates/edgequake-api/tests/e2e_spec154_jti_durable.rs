//! SPEC-154 Wave 4 — durable jti denylist across AppState replicas (EC-154-09).

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use edgequake_api::state::migration_bootstrap::run_postgres_migrations;
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

fn auth_pg_state(pool: sqlx::PgPool) -> AppState {
    let mut state = AppState::test_state_with_pg_pool(pool);
    state.auth.config.auth_enabled = true;
    state.auth.config.api_keys = vec!["master-jti-test-key".to_string()];
    state.auth.config.master_api_key = Some("master-jti-test-key".to_string());
    state.security.kv_identity_mirror = false;
    state
}

async fn connect_and_bootstrap() -> Option<sqlx::PgPool> {
    let database_url = common::spec013_postgres::try_database_url()?;
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .expect("connect postgres");
    run_postgres_migrations(&pool)
        .await
        .expect("bootstrap migrations");
    Some(pool)
}

#[tokio::test]
async fn ec_154_09_logout_jti_rejected_on_second_appstate() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP ec_154_09_logout_jti_rejected_on_second_appstate: DATABASE_URL not set"
            );
            return;
        }
    };

    let state_a = auth_pg_state(pool.clone());
    state_a.initialize_defaults().await.expect("defaults");
    let app_a = build_app(state_a);

    let username = format!("w4_jti_{}", &Uuid::new_v4().to_string()[..8]);
    let create = app_a
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-jti-test-key")
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
        create.status().is_success(),
        "create user {}",
        create.status()
    );

    let login = app_a
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
    assert_eq!(login.status(), StatusCode::OK);
    let login_body = parse_json(login).await;
    let access = login_body["access_token"].as_str().unwrap().to_string();
    let refresh = login_body["refresh_token"].as_str().unwrap().to_string();

    let logout = app_a
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {access}"))
                .body(Body::from(json!({ "refresh_token": refresh }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    // Replica B: fresh AppState sharing the same PG denylist.
    let state_b = auth_pg_state(pool);
    let app_b = build_app(state_b);
    let me = app_b
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/me")
                .header(header::AUTHORIZATION, format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        me.status(),
        StatusCode::UNAUTHORIZED,
        "durable jti must reject on second AppState"
    );
}
