# Lens — UX / UI Designer

Parent: [README](../README.md) · Spec: [09](../09-ux-ui-spec.md)

## Experience principles

1. **Corporate SSO feels native** — one click, familiar IdP, back to EdgeQuake.
2. **Org context is visible** — users always know which tenant they are in.
3. **Failures are calm** — no stack traces; no account enumeration copy.
4. **Password is not shameful** — still available for break-glass / lab.

## Journey maps

### Happy path

```text
Login → SSO → IdP MFA → brief "Signing you in…" → Dashboard (correct tenant)
```

### Multi-org

```text
Login → SSO → KC org picker OR EdgeQuake picker → Dashboard
```

### Failure

```text
Login → SSO → error alert → retry SSO or password
```

## Content / i18n

- Ship en, fr, zh in same PR as UI strings
- Avoid "IdP", "OIDC", "JWT" in primary UI — use "Single sign-on", "Organization"

## Visual

- Reuse SPEC-155 login Card layout
- Provider buttons: consistent height, icon + label, dark mode
- Do not invent a second design system

## Sign-off checklist

- [ ] Keyboard-only login + SSO
- [ ] Screen reader names on buttons
- [ ] Callback loading not blank white > 100ms without spinner
- [ ] No token visible in URL bar screenshot tests
