# 03 — Product specification

Parent: [README](README.md) · Prev: [02-surfaces](02-surfaces.md) ·
Next: [04-architecture](04-architecture.md)

## Personas

| Persona | Goal |
|---------|------|
| **Platform operator** | Deploy EdgeQuake with SSO in one compose/Helm path; rotate secrets; monitor health. |
| **Tenant admin** | Map their org (Google Workspace / Entra tenant) to an EdgeQuake tenant; invite users via IdP. |
| **Analyst** | Sign in with corporate IdP; land in the right workspace; query documents. |
| **Security reviewer** | Verify no email takeover, no tokens in URLs, deprovision works, tenant AuthZ. |
| **AI / MCP host** | Keep using API keys / MCP OAuth (SPEC-154); SSO users may mint scoped keys after login. |

## User stories

### US-158-01 — Operator brings up Keycloak-backed SSO

As a platform operator, I can start EdgeQuake with the Keycloak overlay and a
preloaded realm so `/health` reports OIDC/SSO mechanisms without hand-editing
Keycloak for the happy path.

**Acceptance:** `make dev-sso` (or compose overlay) healthy; realm imported once;
docs match env vars.

### US-158-02 — Analyst signs in with Google (via Keycloak)

As an analyst in Google Workspace, I click "Continue with Google" (or org-first
login), complete Google MFA, and land in my EdgeQuake tenant without a local
password.

**Acceptance:** Session cookie set; no tokens in URL; claims tenant = org alias.

### US-158-03 — Analyst signs in with Microsoft Entra

As an Entra user, I SSO via Keycloak Microsoft/OIDC broker; guest users in a
different Entra tenant are treated as distinct principals.

**Acceptance:** Key on oid+tid mapping through Keycloak `sub`; email not used
for AuthZ.

### US-158-04 — Analyst signs in with GitHub

As a developer, I use GitHub via Keycloak broker; unverified GitHub emails never
auto-link to existing EdgeQuake accounts.

**Acceptance:** Trust Email off; takeover e2e fails closed.

### US-158-05 — AWS workforce identity

As an AWS customer, I federate Cognito user pool or IAM Identity Center through
Keycloak OIDC/SAML and reach EdgeQuake.

**Acceptance:** Documented issuer formats; smoke e2e with simulated OIDC IdP.

### US-158-06 — Multi-org user picks tenant

As a user in two Organizations, I am prompted (Keycloak and/or EdgeQuake) to
select the active tenant; switching requires re-bind, not header spoofing.

**Acceptance:** EC-158-13 green.

### US-158-07 — Tenant admin sees only their tenants

As a tenant admin, `GET /tenants` returns my memberships; I cannot delete
another tenant.

**Acceptance:** EC-158-14 green after W1.

### US-158-08 — Deprovision revokes sessions

As a security admin, when I disable/remove a user in Keycloak, EdgeQuake
sessions end via back-channel logout within the access-token TTL window for
refresh.

**Acceptance:** EC-158-22 green (W6).

### US-158-09 — Break-glass local admin survives IdP outage

As an operator, a bootstrap/local admin password login still works when Keycloak
is down (with audit), so the cluster is not bricked.

**Acceptance:** EC-158-16 green; documented rate limits.

### US-158-10 — Docs replace tribal knowledge

As an implementer, I follow `docs/security/authentication/` end-to-end for
Google, Entra, Cognito, and GitHub without reading source.

**Acceptance:** [16-documentation-plan](16-documentation-plan.md) checklist.

## Non-goals

- Passwordless / passkeys native in EdgeQuake (Keycloak WebAuthn OK).
- Customer-managed encryption keys for tokens.
- Replacing MCP OAuth thin AS.
- Marketplace "Login with X" without Keycloak for SAML.

## Success metrics (post W7)

| Metric | Target |
|--------|--------|
| Time-to-SSO on empty laptop | < 30 minutes with docs |
| Takeover e2e | Fail closed (0 false links) |
| Token-in-URL occurrences | 0 |
| Default-tenant SSO sessions in multi-tenant mode | 0 |
| SPEC-154 suite | Still green |

## Compatibility

| Mode | Behavior |
|------|----------|
| Quickstart auth-off | Unchanged (`EDGEQUAKE_DEV_MODE`) |
| Password-only | Unchanged |
| Legacy `EDGEQUAKE_OIDC_*` | Shim seeds one provider; handoff replaces URL tokens |
| oauth2-proxy | Still documented alternative |
