# API keys and MCP

SSO governs interactive sessions only. Programmatic access uses **API keys** and the MCP OAuth
flow (SPEC-154). An SSO user creates them through the normal endpoints with the session token, so
the SPEC-154 scoping rules apply unchanged (tenant binding, scopes, `aud`).

- Back-channel logout revokes the SSO session (refresh family + federated session); it does **not**
  revoke API keys already issued. Revoke keys explicitly, or deactivate the user.
- Removing a membership or suspending a tenant blocks session refresh immediately
  (`membership_revoked` / `tenant_suspended`).
- MCP OAuth discovery endpoints stay public.
