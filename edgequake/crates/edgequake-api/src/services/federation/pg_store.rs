//! PostgreSQL [`FederationStore`] (migration 164). Multi-replica safe: every single-use
//! artefact is consumed with `DELETE … RETURNING` so two replicas can never both redeem it.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::ApiError;

use super::store::{
    FederatedIdentityRecord, FederatedSession, FederationStore, HandoffRecord, LinkOutcome,
    LoginAttempt, SessionSelector, StoredProvider,
};

pub struct PgFederationStore {
    pool: PgPool,
}

impl PgFederationStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db(context: &'static str) -> impl Fn(sqlx::Error) -> ApiError {
    move |e| ApiError::Internal(format!("federation store ({context}): {e}"))
}

const IDENTITY_COLS: &str = "federated_id, user_id, provider_slug, issuer, subject, \
     email_at_link, email_verified_at_link, linked_at, last_login_at";

fn identity_from_row(row: &sqlx::postgres::PgRow) -> Result<FederatedIdentityRecord, sqlx::Error> {
    Ok(FederatedIdentityRecord {
        federated_id: row.try_get("federated_id")?,
        user_id: row.try_get("user_id")?,
        provider_slug: row.try_get("provider_slug")?,
        issuer: row.try_get("issuer")?,
        subject: row.try_get("subject")?,
        email_at_link: row.try_get("email_at_link")?,
        email_verified_at_link: row.try_get("email_verified_at_link")?,
        linked_at: row.try_get("linked_at")?,
        last_login_at: row.try_get("last_login_at")?,
    })
}

const SESSION_COLS: &str = "family_id, user_id, provider_slug, issuer, subject, idp_sid, \
     tenant_id, workspace_id, created_at, revoked_at";

fn session_from_row(row: &sqlx::postgres::PgRow) -> Result<FederatedSession, sqlx::Error> {
    Ok(FederatedSession {
        family_id: row.try_get("family_id")?,
        user_id: row.try_get("user_id")?,
        provider_slug: row.try_get("provider_slug")?,
        issuer: row.try_get("issuer")?,
        subject: row.try_get("subject")?,
        idp_sid: row.try_get("idp_sid")?,
        tenant_id: row.try_get("tenant_id")?,
        workspace_id: row.try_get("workspace_id")?,
        created_at: row.try_get("created_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

fn provider_from_row(row: &sqlx::postgres::PgRow) -> Result<StoredProvider, sqlx::Error> {
    let role_map: Json<serde_json::Value> = row.try_get("role_map")?;
    let metadata: Json<serde_json::Value> = row.try_get("metadata")?;
    Ok(StoredProvider {
        slug: row.try_get("slug")?,
        kind: row.try_get("kind")?,
        display_name: row.try_get("display_name")?,
        issuer: row.try_get("issuer")?,
        client_id: row.try_get("client_id")?,
        client_secret_ref: row.try_get("client_secret_ref")?,
        redirect_uri: row.try_get("redirect_uri")?,
        scopes: row.try_get("scopes")?,
        trust_email: row.try_get("trust_email")?,
        link_policy: row.try_get("link_policy")?,
        jit_enabled: row.try_get("jit_enabled")?,
        role_claim: row.try_get("role_claim")?,
        role_map: role_map.0,
        max_role: row.try_get("max_role")?,
        enabled: row.try_get("enabled")?,
        metadata: metadata.0,
    })
}

#[async_trait]
impl FederationStore for PgFederationStore {
    async fn find_identity(
        &self,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<FederatedIdentityRecord>, ApiError> {
        let sql = format!(
            "SELECT {IDENTITY_COLS} FROM federated_identities WHERE issuer = $1 AND subject = $2"
        );
        let row = sqlx::query(&sql)
            .bind(issuer)
            .bind(subject)
            .fetch_optional(&self.pool)
            .await
            .map_err(db("find_identity"))?;
        row.as_ref()
            .map(identity_from_row)
            .transpose()
            .map_err(db("find_identity"))
    }

    async fn link_identity(&self, r: &FederatedIdentityRecord) -> Result<LinkOutcome, ApiError> {
        let inserted = sqlx::query(
            r#"INSERT INTO federated_identities
               (federated_id, user_id, provider_slug, issuer, subject,
                email_at_link, email_verified_at_link, linked_at, last_login_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
               ON CONFLICT (issuer, subject) DO NOTHING"#,
        )
        .bind(r.federated_id)
        .bind(r.user_id)
        .bind(&r.provider_slug)
        .bind(&r.issuer)
        .bind(&r.subject)
        .bind(&r.email_at_link)
        .bind(r.email_verified_at_link)
        .bind(r.linked_at)
        .bind(r.last_login_at)
        .execute(&self.pool)
        .await
        .map_err(db("link_identity"))?
        .rows_affected();
        if inserted == 1 {
            return Ok(LinkOutcome::Created);
        }
        match self.find_identity(&r.issuer, &r.subject).await? {
            Some(existing) => Ok(LinkOutcome::AlreadyLinked(Box::new(existing))),
            None => Err(ApiError::Internal(
                "federation store: link race lost row".into(),
            )),
        }
    }

    async fn touch_identity(&self, issuer: &str, subject: &str) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE federated_identities SET last_login_at = NOW() WHERE issuer = $1 AND subject = $2",
        )
        .bind(issuer)
        .bind(subject)
        .execute(&self.pool)
        .await
        .map_err(db("touch_identity"))?;
        Ok(())
    }

    async fn put_login_attempt(&self, a: &LoginAttempt) -> Result<(), ApiError> {
        sqlx::query(
            r#"INSERT INTO oidc_login_attempts
               (state, provider_slug, pkce_verifier, nonce, organization_hint, redirect_after, expires_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
        )
        .bind(&a.state)
        .bind(&a.provider_slug)
        .bind(&a.pkce_verifier)
        .bind(&a.nonce)
        .bind(&a.organization_hint)
        .bind(&a.redirect_after)
        .bind(a.expires_at)
        .execute(&self.pool)
        .await
        .map_err(db("put_login_attempt"))?;
        Ok(())
    }

    async fn take_login_attempt(&self, state: &str) -> Result<Option<LoginAttempt>, ApiError> {
        let row = sqlx::query(
            r#"DELETE FROM oidc_login_attempts WHERE state = $1
               RETURNING state, provider_slug, pkce_verifier, nonce, organization_hint,
                         redirect_after, expires_at"#,
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(db("take_login_attempt"))?;
        let Some(row) = row else { return Ok(None) };
        let attempt = LoginAttempt {
            state: row.try_get("state").map_err(db("take_login_attempt"))?,
            provider_slug: row
                .try_get("provider_slug")
                .map_err(db("take_login_attempt"))?,
            pkce_verifier: row
                .try_get("pkce_verifier")
                .map_err(db("take_login_attempt"))?,
            nonce: row.try_get("nonce").map_err(db("take_login_attempt"))?,
            organization_hint: row
                .try_get("organization_hint")
                .map_err(db("take_login_attempt"))?,
            redirect_after: row
                .try_get("redirect_after")
                .map_err(db("take_login_attempt"))?,
            expires_at: row
                .try_get("expires_at")
                .map_err(db("take_login_attempt"))?,
        };
        Ok((attempt.expires_at > Utc::now()).then_some(attempt))
    }

    async fn put_handoff(&self, h: &HandoffRecord) -> Result<(), ApiError> {
        sqlx::query(
            r#"INSERT INTO auth_handoff_codes
               (code_hash, user_id, family_id, provider_slug, tenant_id, workspace_id,
                redirect_after, expires_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"#,
        )
        .bind(&h.code_hash)
        .bind(h.user_id)
        .bind(h.family_id)
        .bind(&h.provider_slug)
        .bind(h.tenant_id)
        .bind(h.workspace_id)
        .bind(&h.redirect_after)
        .bind(h.expires_at)
        .execute(&self.pool)
        .await
        .map_err(db("put_handoff"))?;
        Ok(())
    }

    async fn take_handoff(&self, code_hash: &str) -> Result<Option<HandoffRecord>, ApiError> {
        let row = sqlx::query(
            r#"DELETE FROM auth_handoff_codes WHERE code_hash = $1
               RETURNING code_hash, user_id, family_id, provider_slug, tenant_id,
                         workspace_id, redirect_after, expires_at"#,
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(db("take_handoff"))?;
        let Some(row) = row else { return Ok(None) };
        let e = db("take_handoff");
        let record = HandoffRecord {
            code_hash: row.try_get("code_hash").map_err(&e)?,
            user_id: row.try_get("user_id").map_err(&e)?,
            family_id: row.try_get("family_id").map_err(&e)?,
            provider_slug: row.try_get("provider_slug").map_err(&e)?,
            tenant_id: row.try_get("tenant_id").map_err(&e)?,
            workspace_id: row.try_get("workspace_id").map_err(&e)?,
            redirect_after: row.try_get("redirect_after").map_err(&e)?,
            expires_at: row.try_get("expires_at").map_err(&e)?,
        };
        Ok((record.expires_at > Utc::now()).then_some(record))
    }

    async fn put_session(&self, s: &FederatedSession) -> Result<(), ApiError> {
        sqlx::query(
            r#"INSERT INTO federated_sessions
               (family_id, user_id, provider_slug, issuer, subject, idp_sid,
                tenant_id, workspace_id, created_at, revoked_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               ON CONFLICT (family_id) DO UPDATE SET
                 idp_sid = EXCLUDED.idp_sid, tenant_id = EXCLUDED.tenant_id,
                 workspace_id = EXCLUDED.workspace_id"#,
        )
        .bind(s.family_id)
        .bind(s.user_id)
        .bind(&s.provider_slug)
        .bind(&s.issuer)
        .bind(&s.subject)
        .bind(&s.idp_sid)
        .bind(s.tenant_id)
        .bind(s.workspace_id)
        .bind(s.created_at)
        .bind(s.revoked_at)
        .execute(&self.pool)
        .await
        .map_err(db("put_session"))?;
        Ok(())
    }

    async fn get_session(&self, family_id: Uuid) -> Result<Option<FederatedSession>, ApiError> {
        let sql = format!("SELECT {SESSION_COLS} FROM federated_sessions WHERE family_id = $1");
        let row = sqlx::query(&sql)
            .bind(family_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db("get_session"))?;
        row.as_ref()
            .map(session_from_row)
            .transpose()
            .map_err(db("get_session"))
    }

    async fn sessions_for(
        &self,
        selector: &SessionSelector,
    ) -> Result<Vec<FederatedSession>, ApiError> {
        let (sql, a, b) = match selector {
            SessionSelector::Sid { issuer, sid } => (
                format!(
                    "SELECT {SESSION_COLS} FROM federated_sessions \
                     WHERE issuer = $1 AND idp_sid = $2 AND revoked_at IS NULL"
                ),
                issuer,
                sid,
            ),
            SessionSelector::Subject { issuer, subject } => (
                format!(
                    "SELECT {SESSION_COLS} FROM federated_sessions \
                     WHERE issuer = $1 AND subject = $2 AND revoked_at IS NULL"
                ),
                issuer,
                subject,
            ),
        };
        let rows = sqlx::query(&sql)
            .bind(a)
            .bind(b)
            .fetch_all(&self.pool)
            .await
            .map_err(db("sessions_for"))?;
        rows.iter()
            .map(session_from_row)
            .collect::<Result<_, _>>()
            .map_err(db("sessions_for"))
    }

    async fn mark_session_revoked(&self, family_id: Uuid) -> Result<(), ApiError> {
        sqlx::query("UPDATE federated_sessions SET revoked_at = NOW() WHERE family_id = $1")
            .bind(family_id)
            .execute(&self.pool)
            .await
            .map_err(db("mark_session_revoked"))?;
        Ok(())
    }

    async fn remember_access_jti(
        &self,
        family_id: Uuid,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"INSERT INTO federated_access_jti (jti, family_id, expires_at)
               VALUES ($1, $2, $3)
               ON CONFLICT (jti) DO UPDATE
                 SET family_id = EXCLUDED.family_id, expires_at = EXCLUDED.expires_at"#,
        )
        .bind(jti)
        .bind(family_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(db("remember_access_jti"))?;
        Ok(())
    }

    async fn access_jtis(&self, family_id: Uuid) -> Result<Vec<(String, DateTime<Utc>)>, ApiError> {
        let rows = sqlx::query(
            "SELECT jti, expires_at FROM federated_access_jti
             WHERE family_id = $1 AND expires_at > NOW()",
        )
        .bind(family_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db("access_jtis"))?;
        rows.iter()
            .map(|row| {
                Ok((
                    row.try_get("jti").map_err(db("access_jtis"))?,
                    row.try_get("expires_at").map_err(db("access_jtis"))?,
                ))
            })
            .collect()
    }

    async fn record_logout_jti(
        &self,
        issuer: &str,
        jti: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, ApiError> {
        sqlx::query(
            "DELETE FROM oidc_logout_jti WHERE issuer = $1 AND jti = $2 AND expires_at <= NOW()",
        )
        .bind(issuer)
        .bind(jti)
        .execute(&self.pool)
        .await
        .map_err(db("record_logout_jti"))?;
        let inserted = sqlx::query(
            "INSERT INTO oidc_logout_jti (issuer, jti, expires_at) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(issuer)
        .bind(jti)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(db("record_logout_jti"))?
        .rows_affected();
        Ok(inserted == 1)
    }

    async fn list_providers(&self) -> Result<Vec<StoredProvider>, ApiError> {
        let rows = sqlx::query(
            r#"SELECT slug, kind, display_name, issuer, client_id, client_secret_ref, redirect_uri,
                      scopes, trust_email, link_policy, jit_enabled, role_claim, role_map,
                      max_role, enabled, metadata
               FROM identity_providers ORDER BY slug"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db("list_providers"))?;
        rows.iter()
            .map(provider_from_row)
            .collect::<Result<_, _>>()
            .map_err(db("list_providers"))
    }

    async fn upsert_provider(&self, p: &StoredProvider) -> Result<(), ApiError> {
        sqlx::query(
            r#"INSERT INTO identity_providers
               (slug, kind, display_name, issuer, client_id, client_secret_ref, redirect_uri,
                scopes, trust_email, link_policy, jit_enabled, role_claim, role_map, max_role,
                enabled, metadata)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
               ON CONFLICT (slug) DO UPDATE SET
                 kind=EXCLUDED.kind, display_name=EXCLUDED.display_name, issuer=EXCLUDED.issuer,
                 client_id=EXCLUDED.client_id, client_secret_ref=EXCLUDED.client_secret_ref,
                 redirect_uri=EXCLUDED.redirect_uri, scopes=EXCLUDED.scopes,
                 trust_email=EXCLUDED.trust_email, link_policy=EXCLUDED.link_policy,
                 jit_enabled=EXCLUDED.jit_enabled, role_claim=EXCLUDED.role_claim,
                 role_map=EXCLUDED.role_map, max_role=EXCLUDED.max_role,
                 enabled=EXCLUDED.enabled, metadata=EXCLUDED.metadata, updated_at=NOW()"#,
        )
        .bind(&p.slug)
        .bind(&p.kind)
        .bind(&p.display_name)
        .bind(&p.issuer)
        .bind(&p.client_id)
        .bind(&p.client_secret_ref)
        .bind(&p.redirect_uri)
        .bind(&p.scopes)
        .bind(p.trust_email)
        .bind(&p.link_policy)
        .bind(p.jit_enabled)
        .bind(&p.role_claim)
        .bind(Json(&p.role_map))
        .bind(&p.max_role)
        .bind(p.enabled)
        .bind(Json(&p.metadata))
        .execute(&self.pool)
        .await
        .map_err(db("upsert_provider"))?;
        Ok(())
    }

    async fn delete_provider(&self, slug: &str) -> Result<bool, ApiError> {
        let n = sqlx::query("DELETE FROM identity_providers WHERE slug = $1")
            .bind(slug)
            .execute(&self.pool)
            .await
            .map_err(db("delete_provider"))?
            .rows_affected();
        Ok(n > 0)
    }

    async fn purge_expired(&self) -> Result<u64, ApiError> {
        let mut removed = 0;
        for table in [
            "oidc_login_attempts",
            "auth_handoff_codes",
            "oidc_logout_jti",
            "federated_access_jti",
        ] {
            let sql = format!("DELETE FROM {table} WHERE expires_at <= NOW()");
            removed += sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(db("purge_expired"))?
                .rows_affected();
        }
        Ok(removed)
    }
}
