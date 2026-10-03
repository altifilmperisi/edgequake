---
title: 'Security Best Practices'
---

# Security Best Practices

> **Product: v0.30.0** · See also: [Runtime auth hardening](/docs/operations/runtime-auth-hardening/)

> **Securing Your EdgeQuake Deployment**

This guide covers security considerations for production EdgeQuake deployments.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ NETWORK LAYER                                            │   │
│  │ • TLS termination (reverse proxy)                        │   │
│  │ • IP allowlisting                                        │   │
│  │ • DDoS protection                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ APPLICATION LAYER                                        │   │
│  │ • API key authentication                                 │   │
│  │ • JWT token validation                                   │   │
│  │ • Rate limiting                                          │   │
│  │ • Request validation                                     │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            ↓                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DATA LAYER                                               │   │
│  │ • Tenant isolation                                       │   │
│  │ • Workspace boundaries                                   │   │
│  │ • Database encryption                                    │   │
│  │ • Secret management                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Network Security

### TLS Configuration

**Always use HTTPS in production.** EdgeQuake doesn't handle TLS directly; use a reverse proxy.

**Caddy (Recommended)**:

```caddyfile
edgequake.example.com {
    reverse_proxy localhost:8080
    # Automatic TLS via Let's Encrypt
}
```

**nginx**:

```nginx
server {
    listen 443 ssl http2;
    server_name edgequake.example.com;

    ssl_certificate /etc/letsencrypt/live/edgequake.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/edgequake.example.com/privkey.pem;

    # Modern TLS settings
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options nosniff;
    add_header X-Frame-Options DENY;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### IP Allowlisting

Restrict access to trusted networks:

```nginx
# nginx: Allow only trusted IPs
location / {
    allow 10.0.0.0/8;      # Internal network
    allow 192.168.0.0/16;  # VPN range
    deny all;

    proxy_pass http://127.0.0.1:8080;
}
```

### Firewall Rules

```bash
# iptables: Only allow HTTP/HTTPS from load balancer
iptables -A INPUT -p tcp --dport 8080 -s 10.0.1.10 -j ACCEPT  # LB IP
iptables -A INPUT -p tcp --dport 8080 -j DROP

# PostgreSQL: Only from app servers
iptables -A INPUT -p tcp --dport 5432 -s 10.0.0.0/24 -j ACCEPT
iptables -A INPUT -p tcp --dport 5432 -j DROP
```

---

## Authentication

EdgeQuake uses a **fail-closed** auth model when enabled. Local dev (`make dev`, Docker quickstart) may set `EDGEQUAKE_DEV_MODE=true` for an open API — **never use that in production**.

| Mode | When | What callers need |
| ---- | ---- | ----------------- |
| Dev / demo | `EDGEQUAKE_DEV_MODE=true` | No credentials (local only) |
| Production | `EDGEQUAKE_AUTH_ENABLED=true` | Valid **JWT** (WebUI login) or **API key** |
| Bootstrap | First admin, no users yet | `EDGEQUAKE_MASTER_API_KEY` or bootstrap env vars |

**SPEC-154 (security hardening)**:

- Access JWT default TTL is **900s (15 minutes)**.
- Web refresh tokens use HttpOnly cookie `eq_refresh` (Path=`/api/v1/auth`); SPA login/refresh omits `refresh_token` from JSON when `Sec-Fetch-*` or `X-Edgequake-Client: webui` is present.
- `EDGEQUAKE_MASTER_API_KEY` is break-glass (full MCP scopes + membership bind bypass, compliance-audited). `EDGEQUAKE_API_KEYS` are **read+query only** — not write break-glass.
- WebSocket: pass JWT via `Authorization: Bearer` or `Sec-WebSocket-Protocol: edgequake.bearer, <jwt>`. Query `?token=` is **rejected**.
- Auth disabled + non-local `DATABASE_URL` without `EDGEQUAKE_DEV_MODE` is a **Fatal** startup error.

Full setup: [Runtime auth hardening](/docs/operations/runtime-auth-hardening/). Spec pack: [`specs/154-sec-hardening/`](../../specs/154-sec-hardening/).

### JWT (interactive users)

WebUI sessions use short-lived JWTs after login. SDKs and scripts should use the access token as a Bearer credential:

```bash
curl -H "Authorization: Bearer eyJ..." \
     -H "X-Tenant-ID: tenant-uuid" \
     -H "X-User-ID: user-uuid" \
     -H "X-Workspace-ID: workspace-uuid" \
     http://localhost:8080/api/v1/documents
```

Refresh tokens rotate via `/api/v1/auth/refresh` (cookie or body). Keep access tokens in memory — not in `localStorage`, URLs, or logs. Residual: Next middleware may mirror access token in a non-HttpOnly cookie for SSR; prefer not to expose JWT to `document.cookie` long-term.

### API key authentication

Programmatic access uses API keys (created via `/api/v1/api-keys` or bootstrap master key):

```bash
# Via X-API-Key header
curl -H "X-API-Key: your-secret-key" \
     -H "X-Workspace-ID: workspace-uuid" \
     http://localhost:8080/api/v1/documents

# Via Authorization Bearer (same key material)
curl -H "Authorization: Bearer your-secret-key" \
     http://localhost:8080/api/v1/documents
```

**API Key Best Practices**:

| Practice     | Recommendation                         |
| ------------ | -------------------------------------- |
| Key length   | Minimum 32 characters                  |
| Key rotation | Every 90 days                          |
| Scope        | Per-tenant or per-workspace            |
| Storage      | Environment variable or secret manager |
| Logging      | Never log full keys                    |

### Workspace and tenant headers

Most `/api/v1/*` routes require explicit tenancy context:

- `X-Tenant-ID` — organization boundary
- `X-User-ID` — acting user (JWT flows)
- `X-Workspace-ID` — data isolation scope (documents, graph, embeddings)

Missing or invalid workspace context returns **403/404**, not silent cross-tenant reads. Configure headers once on your SDK client.

### PostgreSQL row-level security (RLS)

Data isolation is enforced at two layers:

1. **Application layer** — Axum handlers validate tenant/workspace headers and filter queries.
2. **Database layer** — PostgreSQL RLS policies filter rows by session variables (`tenant_id`, `workspace_id`) set per connection checkout.

Do not bypass RLS with a superuser connection for application traffic. Use the `DATABASE_URL` role EdgeQuake expects. Pool checkout sets RLS context on each connection (SPEC-027 SEC-014); sharing connections across tenants without the guard is unsafe.

### LLM provider identity (Vertex AI / SPEC-043)

**Gemini Developer API** uses a static API key (`GOOGLE_API_KEY`).

**Google Vertex AI** (enterprise `vertexai` provider) uses **OAuth2 identity** — short-lived bearer tokens from GCP Application Default Credentials or a service account — **not** a static API key. Leave `api_key_env` empty in `models.toml` for Vertex profiles.

```bash
export GOOGLE_CLOUD_PROJECT="your-project"
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/sa.json"  # or use gcloud ADC
```

The Settings → Provider Status Hub shows **Identity (ADC)** for Vertex. Treat service-account JSON like any other secret (Vault, K8s Secret, not Git).

### Enterprise SSO (Keycloak / OIDC)

Preferred: the built-in OIDC SSO with the shipped Keycloak image (`make dev-sso`). Tenants come from
Keycloak Organizations, roles from realm roles, and tokens never appear in URLs. Start at
[Authentication & SSO](/docs/security/authentication/) and follow the
[production hardening checklist](/docs/security/authentication/production-hardening/).

### External authentication proxy (alternative)

If you cannot use the built-in SSO, put an authentication proxy in front of the WebUI:

**OAuth2 Proxy (for SSO)**:

```yaml
# docker-compose.yml
oauth2-proxy:
  image: quay.io/oauth2-proxy/oauth2-proxy
  environment:
    OAUTH2_PROXY_PROVIDER: oidc
    OAUTH2_PROXY_OIDC_ISSUER_URL: https://auth.example.com
    OAUTH2_PROXY_CLIENT_ID: edgequake
    OAUTH2_PROXY_CLIENT_SECRET: ${OAUTH_SECRET}
    OAUTH2_PROXY_COOKIE_SECRET: ${COOKIE_SECRET}
    OAUTH2_PROXY_UPSTREAMS: http://edgequake:8080
  ports:
    - "4180:4180"
```

---

## Authorization

### Multi-Tenant Isolation

EdgeQuake enforces strict tenant boundaries:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TENANT ISOLATION                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────┐       ┌───────────────────┐              │
│  │    Tenant A       │       │    Tenant B       │              │
│  │ ┌───────────────┐ │       │ ┌───────────────┐ │              │
│  │ │ Workspace 1   │ │       │ │ Workspace 3   │ │              │
│  │ │ - Documents   │ │       │ │ - Documents   │ │              │
│  │ │ - Entities    │ │       │ │ - Entities    │ │              │
│  │ │ - Embeddings  │ │       │ │ - Embeddings  │ │              │
│  │ └───────────────┘ │       │ └───────────────┘ │              │
│  │ ┌───────────────┐ │       │ ┌───────────────┐ │              │
│  │ │ Workspace 2   │ │       │ │ Workspace 4   │ │              │
│  │ │ - Documents   │ │       │ │ - Documents   │ │              │
│  │ │ - Entities    │ │       │ │ - Entities    │ │              │
│  │ │ - Embeddings  │ │       │ │ - Embeddings  │ │              │
│  │ └───────────────┘ │       │ └───────────────┘ │              │
│  └───────────────────┘       └───────────────────┘              │
│           ╲                           ╱                         │
│            ╲   NO DATA SHARING       ╱                          │
│             ╲─────────────────────────                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Enforcement**:

- All queries include `workspace_id` filter
- All data includes `tenant_id` column
- Cross-tenant access denied at database level

### Roles

User records carry a `role` field (`admin`, `editor`, `viewer`, etc.). Sensitive admin routes (user management, API keys, workspace creation) require elevated roles. Prefer least-privilege API keys scoped to a single workspace where possible.

---

## Data Security

### Data at Rest

**PostgreSQL Encryption**:

```sql
-- Enable TDE (Transparent Data Encryption)
-- Requires PostgreSQL Enterprise or managed service

-- For community PostgreSQL, use filesystem encryption:
-- Linux: LUKS, dm-crypt
-- AWS: Encrypted EBS volumes
-- GCP: Default encryption enabled
```

**Environment Variables**:

```bash
# Use encrypted secrets
export OPENAI_API_KEY="$(vault read -field=key secret/openai)"
export DATABASE_URL="$(vault read -field=url secret/database)"
```

### Data in Transit

| Connection             | Encryption        |
| ---------------------- | ----------------- |
| Client → EdgeQuake     | HTTPS (via proxy) |
| EdgeQuake → PostgreSQL | SSL/TLS           |
| EdgeQuake → OpenAI     | HTTPS             |
| EdgeQuake → Ollama     | HTTP (local only) |

**PostgreSQL SSL**:

```bash
# Connection string with SSL
DATABASE_URL="postgresql://user:pass@host:5432/db?sslmode=require"

# With certificate verification
DATABASE_URL="postgresql://user:pass@host:5432/db?sslmode=verify-full&sslrootcert=/path/to/ca.crt"
```

### Secret Management

**Never commit secrets to Git.**

| Secret           | Storage Recommendation     |
| ---------------- | -------------------------- |
| `OPENAI_API_KEY` | Vault, AWS Secrets Manager |
| `DATABASE_URL`   | Vault, Kubernetes Secret   |
| API keys         | Database (hashed)          |
| JWT signing key  | Vault, environment         |

**HashiCorp Vault Example**:

```bash
# Store secrets
vault kv put secret/edgequake \
  openai_key="sk-..." \
  database_url="postgresql://..."

# Retrieve in application
export OPENAI_API_KEY="$(vault kv get -field=openai_key secret/edgequake)"
```

**Kubernetes Secrets**:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edgequake-secrets
type: Opaque
stringData:
  OPENAI_API_KEY: sk-your-key-here
  DATABASE_URL: postgresql://...
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: edgequake
          envFrom:
            - secretRef:
                name: edgequake-secrets
```

---

## Input Validation

### Request Validation

EdgeQuake validates all inputs:

| Field          | Validation                              |
| -------------- | --------------------------------------- |
| `workspace_id` | UUID format, exists                     |
| `document_id`  | UUID format, exists, owned by workspace |
| `query`        | Non-empty, max 10,000 chars             |
| `file`         | Size limit, MIME type check             |

### File Upload Security

```rust
// Implemented in EdgeQuake
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;  // 50 MB
const ALLOWED_TYPES: &[&str] = &["application/pdf", "text/plain", "text/markdown"];
```

**Additional Protections**:

- Content-type sniffing (actual vs declared)
- Filename sanitization
- Path traversal prevention
- Virus scanning (integrate ClamAV)

---

## Rate Limiting

EdgeQuake includes built-in rate limiting:

```
┌─────────────────────────────────────────────────────────────────┐
│                    RATE LIMITING                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Request → [Per-IP Limiter] → [Per-Key Limiter] → Handler       │
│                  │                    │                         │
│              429 if                429 if                       │
│              exceeded              exceeded                     │
│                                                                 │
│  Default Limits:                                                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Endpoint Category  │ Requests │ Window │ Burst          │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ Document upload    │ 10       │ 1 min  │ 3              │    │
│  │ Query              │ 60       │ 1 min  │ 10             │    │
│  │ Graph traversal    │ 100      │ 1 min  │ 20             │    │
│  │ Health checks      │ No limit │ -      │ -              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Custom Limits** (nginx):

```nginx
# Additional rate limiting at proxy
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $http_x_api_key zone=apikey:10m rate=100r/s;

location /api/ {
    limit_req zone=api burst=20 nodelay;
    limit_req zone=apikey burst=50 nodelay;
    proxy_pass http://127.0.0.1:8080;
}
```

---

## Logging & Auditing

### Security Logging

EdgeQuake logs security events:

| Event         | Log Level | Example                              |
| ------------- | --------- | ------------------------------------ |
| Auth success  | INFO      | `user=X authenticated`               |
| Auth failure  | WARN      | `invalid_api_key from IP`            |
| Rate limited  | WARN      | `rate_limit_exceeded user=X`         |
| Access denied | WARN      | `access_denied tenant=X workspace=Y` |
| Admin action  | INFO      | `workspace_deleted by user=X`        |

### Log Aggregation

```yaml
# Ship logs to centralized system
docker-compose.yml:
  edgequake:
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "5"
```

**Recommended Stack**:

- Loki + Grafana (lightweight)
- ELK Stack (feature-rich)
- Datadog/Splunk (managed)

---

## LLM Security

### API Key Protection

```bash
# Don't pass keys in URLs
# BAD: curl "http://api.openai.com?api_key=sk-..."
# GOOD: curl -H "Authorization: Bearer sk-..." http://api.openai.com

# Rotate keys if compromised
# 1. Generate new key in OpenAI dashboard
# 2. Update environment variable
# 3. Revoke old key
```

### Prompt Injection Prevention

EdgeQuake mitigates prompt injection:

| Mitigation        | Implementation                      |
| ----------------- | ----------------------------------- |
| System prompt     | Separate from user input            |
| Context isolation | Retrieved docs in structured format |
| Output validation | Response format checking            |

**Example System Prompt**:

```
You are a helpful assistant answering questions about the provided documents.
Answer based ONLY on the context provided. If the answer is not in the context,
say "I don't have enough information to answer that question."

<context>
{retrieved_documents}
</context>

User question: {user_query}
```

### Data Leakage Prevention

```
┌─────────────────────────────────────────────────────────────────┐
│                 DATA FLOW TO LLM                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document Upload → Chunking → [PII Detection] → LLM             │
│                                      │                          │
│                               Redact if                         │
│                               configured                        │
│                                                                 │
│  Sensitive Data Handling:                                       │
│  • Never send passwords to LLM                                  │
│  • Optionally redact PII before processing                      │
│  • Use local LLM (Ollama) for sensitive data                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Multi-replica operations (SPEC-057)

When `EDGEQUAKE_REPLICAS>1`, task delivery must be `bridged` or `notify_only` — boot **fails** with `local` delivery.

| Risk | Mitigation |
| ---- | ---------- |
| Duplicate task processing | Correctness is always `claim_next` + lease — never process from a channel payload without claim |
| Stale cancel/progress UI | Use `track_id` SSOT; poll or WebSocket `/ws/progress/{track_id}` |
| Cross-replica auth drift | Same `DATABASE_URL`, same auth env on every replica |
| RLS context leaks | One connection per request scope; do not share pooled connections across tenants |

See [Deployment § Multi-replica](/docs/operations/deployment/#multi-replica-task-delivery) and [Ingestion cancel & fairness](/docs/ingestion-cancel-and-fairness.md).

---

## Storage requirements

`DATABASE_URL` is **required** for all server modes. In-memory storage has been removed — running without PostgreSQL exits with code 1.

Supported PostgreSQL images: **16, 17, 18** (`ghcr.io/raphaelmansuy/edgequake-postgres:0.23.0-pg16|pg17|pg18`). Use TLS for remote databases (`sslmode=require` or stronger).

---

## Container image

The GHCR API image (`ghcr.io/raphaelmansuy/edgequake`) is **distroless** (`gcr.io/distroless/cc-debian12:nonroot`). It has no shell and no `curl`/`wget`.

| Probe | How |
| ----- | --- |
| Docker / Compose HEALTHCHECK | `edgequake healthcheck` (GET `/live` on `127.0.0.1:${EDGEQUAKE_PORT:-8080}`) |
| Kubernetes liveness | HTTP GET `/live` |
| Kubernetes readiness | HTTP GET `/ready` |
| Kubernetes preStop | `edgequake pre-stop <seconds>` (no `sh`) |

Do **not** `docker exec … /bin/sh` into the API container — there is no shell. Debug with pod logs, a sidecar, or the distroless `:debug-nonroot` variant if you rebuild with that base. Host-side `curl http://localhost:8080/live` is fine.

Trivy HIGH/CRITICAL on the API image is a release gate. The only documented ignore is `CVE-2026-14456` (OpenSSL QUIC server, 3.5+ only; the image ships OpenSSL 3.0.x).

---

## Production Hardening Checklist

### Pre-Deployment

- [ ] TLS enabled (HTTPS)
- [ ] Reverse proxy configured (nginx/Caddy)
- [ ] `EDGEQUAKE_AUTH_ENABLED=true`, `EDGEQUAKE_DEV_MODE` unset
- [ ] API keys rotated from defaults; master key not in compose files
- [ ] `DATABASE_URL` set; PostgreSQL 16–18 with pgvector + AGE
- [ ] Vertex/service-account secrets in secret manager (not env files in Git)
- [ ] If `EDGEQUAKE_REPLICAS>1`: `EDGEQUAKE_TASK_DELIVERY=bridged` or `notify_only`
- [ ] Rate limiting configured
- [ ] Firewall rules applied
- [ ] Logging to centralized system

### Runtime

- [ ] Container probes use `/live` (liveness) and `/ready` (readiness), not deep `/health`
- [ ] Health checks monitored
- [ ] Error rates alerting configured
- [ ] Rate limit violations tracked
- [ ] Auth failure monitoring
- [ ] Database backups verified
- [ ] Log retention policy set

### Periodic

- [ ] API key rotation (90 days)
- [ ] Dependency updates (monthly)
- [ ] Security audit (quarterly)
- [ ] Penetration testing (annually)
- [ ] Incident response plan tested

---

## Security Incidents

### Response Procedure

1. **Detect**: Monitor for anomalies
2. **Contain**: Disable compromised credentials
3. **Investigate**: Review logs
4. **Remediate**: Patch vulnerabilities
5. **Communicate**: Notify affected parties
6. **Document**: Post-incident review

### Contact

For security vulnerabilities, contact: security@edgequake.dev

---

## See Also

- [Runtime auth hardening](/docs/operations/runtime-auth-hardening/) — JWT, API keys, bootstrap
- [Deployment Guide](/docs/operations/deployment/) — Production setup, GHCR images, multi-replica
- [Configuration Reference](/docs/operations/configuration/) — Vertex OAuth2, `EDGEQUAKE_REPLICAS`
- [Monitoring Guide](/docs/operations/monitoring/) — Observability
