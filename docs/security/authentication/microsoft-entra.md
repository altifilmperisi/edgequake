# Microsoft Entra ID

1. App registrations -> New -> Web, redirect `https://<api>/api/v1/auth/oidc/callback`,
   supported account types per your policy; create a client secret.
2. Configure with the **v2.0 issuer of your directory**:

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=entra
EDGEQUAKE_OIDC_SLUG=microsoft
EDGEQUAKE_OIDC_ISSUER_URL=https://login.microsoftonline.com/<tenant-guid>/v2.0
EDGEQUAKE_OIDC_CLIENT_ID=<application-id>
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_ALLOWED_TID=<tenant-guid>      # allow-list of directories (tid claim)
EDGEQUAKE_OIDC_TENANT_SLUG=contoso
```

Notes: the stable identity is `tid` + `oid`/`sub`; do not use the multi-tenant `common` authority
without an `ALLOWED_TID` allow-list (`idp_tenant_not_allowed`). Guest users keep their home `tid`.
