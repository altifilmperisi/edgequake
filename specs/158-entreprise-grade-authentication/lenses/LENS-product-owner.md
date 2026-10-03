# Lens — Product Owner

Parent: [README](../README.md) · Stories: [03](../03-product-spec.md)

## Job story

Enterprises will not put Graph-RAG corpora behind a second password database.
SSO via Keycloak + Organizations is the **product** answer; oauth2-proxy is an
escape hatch, not the brochure path.

## Priorities

| P | Item | Wave |
|---|------|------|
| P0 | No takeover / no tokens in URL / tenant AuthZ | W1 |
| P0 | Org → tenant binding | W3 |
| P1 | Shippable Keycloak image + compose | W4 |
| P1 | WebUI SSO | W5 |
| P1 | Back-channel logout | W6 |
| P2 | Native GitHub without Keycloak | W7 |
| P2 | Docs excellence | W7 |

## Commercial talking points

- Hyperscaler IdPs: Google, Microsoft, AWS, GitHub via one control plane
- Multi-tenant Organizations aligned to EdgeQuake tenants
- MCP/API keys unchanged (SPEC-154)
- CNCF Keycloak — no proprietary IdP lock-in for EdgeQuake itself

## Compatibility promises

- Password login remains
- Quickstart auth-off / DEV_MODE remains for demos
- Legacy `EDGEQUAKE_OIDC_*` keeps working (handoff URL change is the one
  breaking change — call out in release notes)

## Non-goals to defend in reviews

- Do not embed Keycloak in the Rust binary
- Do not delay W1 safety for perfect admin UI
- Do not accept email-as-identity "for convenience"

## Acceptance for GA

US-158-01…10 met; EC matrix green; docs published; SPEC-154 green.
