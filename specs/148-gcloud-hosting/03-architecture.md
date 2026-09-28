# 03 — Architecture

> **Cross-refs:** [Laws](01-first-principles.md) · [SSOT](../../deploy/gcp/)

## Shape (Option A)

```text
Clients
   │
   ├─ HTTP :80  ──301/308──▶  HTTPS :443
   └─ HTTPS :443 ──────────▶  Caddy (tls internal | Let's Encrypt)
                                   │
                    ┌──────────────┼──────────────┐
                    ▼              ▼              ▼
              frontend:3000    api:8080      (no public bind)
                    │              │
                    └──────┬───────┘
                           ▼
                    postgres:5432
                    GHCR edgequake-postgres
                    AGE + pgvector
                           │
                    /mnt/elitizon-db-data/pgdata  (pd-balanced + snapshots)
```

## Compose mapping (quickstart → GCP overlay)

| Quickstart service | GCP overlay | Difference |
|--------------------|-------------|------------|
| `postgres` | `postgres` | Bind-mount data disk; **no** host `ports` |
| `api` | `api` | No host `ports`; prod auth; secrets from `.env`; `EDGEQUAKE_CORS_ORIGINS=$PUBLIC_ORIGIN` (v0.26.5 refuses open CORS on non-local DB) |
| `frontend` | `frontend` | No host `ports`; `EDGEQUAKE_API_URL=https://$PUBLIC_ORIGIN` |
| — | `caddy` | Only published ports `80`/`443` |
| `EDGEQUAKE_DEV_MODE=true` | `false` | LAW-148-9 |
| `latest` | `0.26.5` | LAW-148-4 |

Images (LAW-148-4):

| Role | Image |
|------|--------|
| API | `ghcr.io/raphaelmansuy/edgequake:0.26.5` |
| Web | `ghcr.io/raphaelmansuy/edgequake-frontend:0.26.5` |
| DB | `ghcr.io/raphaelmansuy/edgequake-postgres:0.26.5` (default PG18) |
| Edge | `caddy:2.10-alpine` |

API image is **distroless** (`ENTRYPOINT ["/usr/local/bin/edgequake"]`). Migrate is `docker compose run --rm --no-deps api migrate` — same binary as Helm `command: ["edgequake", "migrate"]`.

## Caddy routes (HTTPS)

| Path | Upstream |
|------|----------|
| `/api/*`, `/health`, `/live`, `/ready`, `/metrics`, `/version`, `/swagger-ui*`, `/api-docs*`, `/ws*` | `api:8080` |
| everything else (incl. WebUI `/api-explorer`) | `frontend:3000` |

Use `/api/*` (not `/api*`): Caddy `/api*` is a string prefix and steals `/api-explorer` from the frontend.

`:80` contains **only** `redir https://{host}{uri} permanent`. No `reverse_proxy` on HTTP.

HTTPS (`:443`) uses a **file** cert with IP SAN (`generate-tls.sh`) until DNS. `tls internal` fails on Docker-published :443 because clients send **empty SNI** for IP URLs and Caddy sees the container IP ([caddy#6344](https://github.com/caddyserver/caddy/issues/6344)). After `hostname` tfvar, switch to [Caddyfile.hostname](../../deploy/gcp/compose/Caddyfile.hostname) (Let's Encrypt).

SSE: do **not** gzip at Caddy (Next.js already `compress: false` for event-stream).

## GCP resources

| Resource | Name | Notes |
|----------|------|--------|
| VPC / subnet | `edgequake-host-vpc` / `edgequake-host-subnet` (`10.48.0.0/24`) | Not `default`; leftover Option B keeps `edgequake-vpc` / `edgequake-subnet` |
| VM | `elitizon-db` | Shared AGE+pgvector host; `e2-medium`, `us-central1-a`, Shielded VM, OS Login |
| Static IP | `edgequake-ip` | EXTERNAL, region `us-central1` |
| Data disk | `elitizon-db-data` | 50 GiB `pd-balanced`, `auto_delete=false` |
| Snapshot policy | `edgequake-daily-snap` | 14-day retention |
| SA (VM) | `edgequake-gce@` | secrets + GCS read + logging |
| SA (GitHub) | `edgequake-github@` | IAP tunnel + OS Login admin |
| Secrets | `edgequake-postgres-password`, `edgequake-jwt`, `edgequake-bootstrap-admin-password`, `edgequake-master-api-key`, `edgequake-openai-api-key`, `edgequake-mistral-api-key` | |
| Bucket | `saas-app-001-edgequake-artifacts` | Compose + scripts |
| WIF | pool `edgequake-github` | GitHub OIDC, repo-scoped |

## LLM

POC: external provider APIs (OpenAI/etc.) via Secret Manager. **No** GCE GPU. Ollama is optional later and is **not** in the first overlay (4 GiB is tight for app+DB+model).

`render-env.sh` uses OpenAI when `edgequake-openai-api-key` is a real secret version. While the Terraform placeholder is `UNSET`, it sets `EDGEQUAKE_LLM_PROVIDER=mock` **and** `EDGEQUAKE_ALLOW_MOCK_PROVIDER=1` so v0.26.5 can boot (EC8). Production ingest needs a real key: add a Secret Manager version and re-run `deploy.sh` (the allow-mock hatch is then cleared). Do **not** copy the leftover Cloud Run `OPENAI_API_KEY`.
