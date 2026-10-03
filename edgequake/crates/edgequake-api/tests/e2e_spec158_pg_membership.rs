//! SPEC-158 — membership metadata must round-trip through PostgreSQL.
//!
//! SSO-managed memberships are identified by `metadata.source == "sso"` (role resync, last-owner
//! guard). The in-memory service always preserved it; the PG service used to drop it, so this
//! guards the real store. Skips unless `DATABASE_URL` points at a migrated database.
#![cfg(feature = "postgres")]

use edgequake_core::{Membership, MembershipRole, Tenant, WorkspaceService, WorkspaceServiceImpl};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())?;
    PgPool::connect(&url).await.ok()
}

#[tokio::test]
async fn pg_membership_metadata_roundtrips_and_survives_role_update() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let svc = WorkspaceServiceImpl::new(pool.clone());
    let slug = format!("spec158-{}", &Uuid::new_v4().to_string()[..8]);
    let tenant = svc
        .create_tenant(Tenant::new("SPEC-158 membership", &slug))
        .await
        .expect("create tenant");
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (user_id, tenant_id, username, role) VALUES ($1, $2, $3, 'user')",
    )
    .bind(user_id)
    .bind(tenant.tenant_id)
    .bind(format!("u-{}", &user_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("insert user");

    let mut membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
    membership.metadata.insert("source".into(), json!("sso"));
    membership
        .metadata
        .insert("provider".into(), json!("keycloak"));
    svc.add_membership(membership)
        .await
        .expect("add membership");

    let stored = svc.get_user_memberships(user_id).await.expect("read back");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].metadata.get("source"), Some(&json!("sso")));
    assert_eq!(stored[0].metadata.get("provider"), Some(&json!("keycloak")));

    let updated = svc
        .update_membership_role(stored[0].membership_id, MembershipRole::Admin)
        .await
        .expect("role update");
    assert_eq!(updated.role, MembershipRole::Admin);
    assert_eq!(
        updated.metadata.get("source"),
        Some(&json!("sso")),
        "role update keeps metadata"
    );

    let by_tenant = svc.get_tenant_memberships(tenant.tenant_id).await.unwrap();
    assert_eq!(by_tenant[0].metadata.get("source"), Some(&json!("sso")));

    let _ = sqlx::query("DELETE FROM users WHERE user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await;
    let _ = svc.delete_tenant(tenant.tenant_id).await;
}
