# AWS Cognito (and IAM Identity Center)

Create a user pool + app client (confidential, authorization-code grant, scopes `openid email profile`).

```bash
EDGEQUAKE_OIDC_ENABLED=true
EDGEQUAKE_OIDC_KIND=cognito
EDGEQUAKE_OIDC_SLUG=aws
EDGEQUAKE_OIDC_ISSUER_URL=https://cognito-idp.<region>.amazonaws.com/<pool-id>
EDGEQUAKE_OIDC_CLIENT_ID=...
EDGEQUAKE_OIDC_CLIENT_SECRET=...
EDGEQUAKE_OIDC_ROLE_CLAIM=cognito:groups
EDGEQUAKE_OIDC_ROLE_MAP={"eq-admin":"admin"}
EDGEQUAKE_OIDC_TENANT_SLUG=acme
```

Cognito groups arrive in `cognito:groups`. IAM Identity Center is best brokered through Keycloak
(SAML/OIDC identity provider) so organizations still map to tenants.
