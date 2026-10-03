# 06 — Identity provider matrix

Parent: [README](README.md) · Architecture: [04](04-architecture.md) ·
Keycloak: [05](05-keycloak-integration.md)

Primary path for all hyperscalers: **Keycloak identity brokering**. EdgeQuake
speaks OIDC to Keycloak. Native adapters are optional (T2) except where noted.

## Summary

| Provider | Protocol to Keycloak | EdgeQuake sees | Stable subject | Email policy | Tenant hint |
|----------|----------------------|----------------|----------------|--------------|-------------|
| Google | Keycloak Google / OIDC | KC `sub` + org claim | Google `sub` (via broker link) | Honour `email_verified`; `trust_email` only for Workspace | `hd` domain → org domain |
| Microsoft Entra | Keycloak Microsoft / OIDC v2 | KC `sub` + org | Entra `oid`+`tid` | Email **mutable** — never AuthZ | `tid` map optional |
| AWS Cognito | Keycloak OIDC | KC `sub` + org | Cognito `sub` | Pool policy | app client → org |
| AWS IAM Identity Center | Keycloak SAML or OIDC | KC `sub` + org | NameID / sub | Org-controlled | org alias |
| GitHub | Keycloak GitHub | KC `sub` + org | GitHub user id | **Trust Email OFF** | none (invite/org) |
| Generic OIDC | Keycloak OIDC IdP | KC `sub` + org | upstream `sub` | Per IdP flag | org domain |
| Generic SAML | Keycloak SAML IdP | KC `sub` + org | NameID | Per IdP flag | org domain |

Official refs:

- [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect)
- [Entra ID token claims](https://learn.microsoft.com/en-us/entra/identity-platform/id-token-claims-reference)
- [Cognito federation endpoints](https://docs.aws.amazon.com/cognito/latest/developerguide/federation-endpoints.html)
- [GitHub authorizing OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)
- [Keycloak identity brokering](https://www.keycloak.org/docs/latest/server_admin/#_identity_broker)
- [Keycloak Organizations](https://docs.redhat.com/en/documentation/red_hat_build_of_keycloak/26.4/html/server_administration_guide/managing_organizations)

---

## Google

**Setup (Keycloak):** Identity Provider → Google; client from Google Cloud
Console; scopes `openid email profile`. Link IdP to Organization with domain
`example.com` and "Redirect when email domain matches" ON when desired.

**Claims EdgeQuake must honour (via KC token / brokered attributes):**

| Claim | Rule |
|-------|------|
| `sub` | Primary federation key (never email) |
| `email_verified` | Required true for email link / display trust |
| `hd` | If tenant restriction configured, reject missing/mismatch (Google warns: absence means consumer account) |

**Edge cases:** Consumer Gmail vs Workspace; email change keeps `sub`.

---

## Microsoft Entra ID

**Setup:** App registration (v2 endpoint); Keycloak Microsoft or generic OIDC
issuer `https://login.microsoftonline.com/{tenant}/v2.0`.

**Rules from Microsoft docs:**

- Identify users with `oid` (+ `tid` for tenant routing), not email/UPN.
- `email` is mutable and not guaranteed for managed users.
- Guest (`idp` differs): treat as new user in the resource tenant.
- Groups overage: do not require full `groups` in token for EdgeQuake AuthZ in
  v1; map Keycloak roles/groups instead.

**Multi-tenant Entra apps:** Restrict which `tid` values map to which EdgeQuake
Organization (allow-list).

---

## AWS Cognito / IAM Identity Center

**Cognito user pools as OIDC issuer:**

- Discovery: `https://cognito-idp.{region}.amazonaws.com/{poolId}/.well-known/openid-configuration`
- JWKS: `.../.well-known/jwks.json`
- Note **original vs updated issuer** hostnames (AWS docs); pin issuer in
  Keycloak IdP config.

**IAM Identity Center:** Prefer SAML 2.0 or OIDC app assignment into Keycloak;
IAM *roles* alone are not an end-user login for WebUI.

**Not in scope as user IdP:** raw AWS IAM access keys for humans.

---

## GitHub

**Protocol:** OAuth 2.0 authorization code (+ PKCE S256 strongly recommended /
required when challenge sent). **No OIDC id_token.**

**Primary:** Keycloak built-in GitHub IdP. Scopes: `read:user user:email`.

**Security policy (LAW-158-3 / LAW-158-12):**

- Keycloak **Trust Email = OFF**
- First broker login: confirm link / re-auth — **no auto-link**
- Prefer primary **verified** email from GitHub API when creating profiles
- Private email / `noreply` addresses: allow JIT with KC sub; do not merge to
  corporate accounts by string match

**T2 native adapter:** `GithubOAuthProvider` in EdgeQuake only if operator
runs without Keycloak; same trust rules.

---

## Role / group mapping (all providers)

Keycloak realm roles or organization groups → token claims (e.g. `realm_access`
or custom `edgequake_role`) → EdgeQuake mapper:

| IdP / KC role | MembershipRole cap |
|---------------|-------------------|
| `edgequake-owner` | Owner |
| `edgequake-admin` | Admin |
| `edgequake-member` | Member (default) |
| `edgequake-readonly` | Readonly |

Mapper never elevates above IdP config max. Re-applied every login (LAW-158-8).

---

## CI simulation strategy

| Upstream | Simulation |
|----------|------------|
| Google / Entra / Cognito | Second Keycloak realm acting as OIDC IdP, or wiremock discovery+JWKS+token |
| GitHub | wiremock authorize/token/user/emails |
| Organizations | Real Keycloak from `edgequake-keycloak` image |

No live hyperscaler credentials required in CI.
