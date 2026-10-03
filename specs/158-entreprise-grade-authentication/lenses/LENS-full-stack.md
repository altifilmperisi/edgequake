# Lens — Full Stack Developer

Parent: [README](../README.md) · Architecture: [04](../04-architecture.md) ·
Surfaces: [02](../02-surfaces.md)

## Design rules

1. **Extend, don't fork** — grow `oidc_flow` into providers; keep
   `CredentialDecision` as the only verifier.
2. **Pure logic in `edgequake-auth`** — claim parsing, role maps, org claim.
3. **Side effects in `edgequake-api` services** — HTTP, SQL, cookies.
4. **One token issuer** — password and SSO call the same function.
5. **Env is a shim** — not the long-term config SSOT (DB registry is).

## Touch list by wave

| Wave | Primary files |
|------|---------------|
| W1 | `handlers/auth/oidc.rs`, `tenants.rs`, `refresh_cookie.rs`, `security_config` / startup |
| W2 | new migration 164, `oidc_pending`, identity services |
| W3 | `provider_registry`, `tenant_resolver`, `federation_jit` |
| W4 | `deploy/keycloak/*`, compose |
| W5 | WebUI login/callback/stores |
| W6 | `backchannel_logout` |
| W7 | docs + optional GitHub provider |

## Anti-patterns

- Copy-pasting `issue_login_tokens` again
- Special-casing `if provider == "google"` in handlers
- Trusting `X-Tenant-ID` when SSO is on
- Returning refresh tokens in JSON to browsers
- In-memory maps for login state

## Local verify

```bash
cargo test -p edgequake-api --test spec027_oidc_e2e
cargo clippy -p edgequake-api -p edgequake-auth --all-targets
```
