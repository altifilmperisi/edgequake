#!/usr/bin/env python3
"""SPEC-158 Keycloak smoke (stdlib only).

Part 1 (always): headless Authorization-Code+PKCE login against the imported `edgequake` realm and
assertions on the ID token EdgeQuake consumes (organization claim, realm roles, sid, email_verified).
Part 2 (--api URL): the same login driven through EdgeQuake (`/auth/oidc/login` → Keycloak →
`/auth/oidc/callback` → SPA redirect with an opaque `code` → `POST /auth/handoff`).

  python3 scripts/keycloak_smoke.py [--kc http://keycloak:8081] [--resolve 127.0.0.1]
                                    [--api http://localhost:8080] [--admin-password ...] [--deep] [--password demo-password-change-me]

`--resolve` pins the Keycloak hostname to an IP (equivalent to the /etc/hosts line in the docs).
"""
import argparse, base64, hashlib, hmac, json, os, re, secrets, socket, subprocess, sys, threading, time
import urllib.error, urllib.parse, urllib.request
from http.cookiejar import CookieJar, DefaultCookiePolicy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REALM = "edgequake"
CLIENT_ID = "edgequake-web"


def pin_host(host: str, ip: str) -> None:
    """Resolve `host` to `ip` for this process only."""
    real = socket.getaddrinfo

    def patched(h, *a, **k):
        return real(ip if h == host else h, *a, **k)

    socket.getaddrinfo = patched


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):  # noqa: D401
        return None


class _LocalPolicy(DefaultCookiePolicy):
    """Browsers treat http://localhost as a secure context and keep `Secure` cookies; mirror that."""

    def return_ok_secure(self, cookie, request):
        return True


def new_jar() -> CookieJar:
    return CookieJar(_LocalPolicy())


def opener(jar: CookieJar):
    return urllib.request.build_opener(NoRedirect, urllib.request.HTTPCookieProcessor(jar))


def _lower(headers) -> dict:
    return {k.lower(): v for k, v in headers.items()}


def fetch(op, url, data=None, headers=None):
    req = urllib.request.Request(url, data=data, headers=headers or {})
    try:
        r = op.open(req, timeout=30)
        return r.status, _lower(r.headers), r.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, _lower(e.headers), e.read().decode()


def login_form(html: str):
    m = re.search(r'<form[^>]+id="kc-form-login"[^>]+action="([^"]+)"', html) or re.search(
        r'<form[^>]+action="([^"]+)"[^>]*method="post"', html, re.I
    )
    if not m:
        raise SystemExit("keycloak login form not found")
    return m.group(1).replace("&amp;", "&")


def kc_login(op, location: str, username: str, password: str, stop_prefix: str) -> str:
    """Follow redirects from `location`, answering Keycloak's (one- or two-step) login form,
    and return the first URL that starts with `stop_prefix`."""
    url, sent_password = location, False
    status, headers, body = 302, {"location": location}, ""
    for _ in range(16):
        if status in (301, 302, 303, 307):
            url = urllib.parse.urljoin(url, headers["location"])
            if url.startswith(stop_prefix):
                return url
            status, headers, body = fetch(op, url)
        elif status == 200 and "kc-form-login" in body:
            broker = None
            if username == "unused":
                broker = re.search(
                    r'href="([^"]*(?:kc_idp_hint=stub-oidc|/broker/stub-oidc/)[^"]*)"',
                    body,
                )
            if broker:
                url = urllib.parse.urljoin(url, broker.group(1).replace("&amp;", "&"))
                status, headers, body = fetch(op, url)
                continue
            if sent_password:  # form re-rendered after the password step → rejected (never retry: brute-force lock)
                raise SystemExit(f"login failed for {username}: credentials rejected")
            fields = {k: v for k, v in (("username", username), ("password", password)) if f'name="{k}"' in body}
            if not fields:
                raise SystemExit(f"login form for {username} has no username/password field")
            sent_password = "password" in fields
            url = urllib.parse.urljoin(url, login_form(body))
            status, headers, body = fetch(
                op, url, urllib.parse.urlencode(fields).encode(), {"Content-Type": "application/x-www-form-urlencoded"}
            )
        else:
            snippet = body[:400].replace("\n", " ")
            raise SystemExit(f"unexpected HTTP {status} at {url}: {snippet}")
    raise SystemExit("too many redirects")


def pkce():
    verifier = secrets.token_urlsafe(48)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    return verifier, challenge


def jwt_payload(token: str) -> dict:
    part = token.split(".")[1]
    return json.loads(base64.urlsafe_b64decode(part + "=" * (-len(part) % 4)))


def part1(kc: str, password: str, secret: str, redirect_uri: str) -> None:
    base = f"{kc}/realms/{REALM}"
    op = opener(new_jar())
    status, _, body = fetch(op, f"{base}/.well-known/openid-configuration")
    assert status == 200, f"discovery HTTP {status}"
    disc = json.loads(body)
    assert disc["issuer"] == base, f"issuer {disc['issuer']} != {base}"
    assert "S256" in disc.get("code_challenge_methods_supported", []), "PKCE S256 unsupported"
    assert disc.get("backchannel_logout_supported"), "back-channel logout unsupported"
    print("ok  discovery: issuer + PKCE + back-channel logout")

    for user, scope, want_orgs, want_roles in [
        ("alice", "openid email profile organization:acme", {"acme"}, {"eq-admin"}),
        ("carol", "openid email profile organization:*", {"acme", "globex"}, {"eq-user"}),
    ]:
        verifier, challenge = pkce()
        q = urllib.parse.urlencode({
            "client_id": CLIENT_ID, "redirect_uri": redirect_uri, "response_type": "code", "scope": scope,
            "state": secrets.token_hex(8), "nonce": secrets.token_hex(8),
            "code_challenge": challenge, "code_challenge_method": "S256",
        })
        final = kc_login(opener(new_jar()), f"{disc['authorization_endpoint']}?{q}", user, password, redirect_uri)
        code = urllib.parse.parse_qs(urllib.parse.urlparse(final).query)["code"][0]
        data = urllib.parse.urlencode({
            "grant_type": "authorization_code", "code": code, "redirect_uri": redirect_uri,
            "client_id": CLIENT_ID, "client_secret": secret, "code_verifier": verifier,
        }).encode()
        status, _, body = fetch(op, disc["token_endpoint"], data, {"Content-Type": "application/x-www-form-urlencoded"})
        assert status == 200, f"token HTTP {status}: {body[:200]}"
        claims = jwt_payload(json.loads(body)["id_token"])
        org = claims.get("organization")
        aliases = set(org.keys()) if isinstance(org, dict) else set(org or [])
        assert want_orgs <= aliases, f"{user}: organization {org!r} missing {want_orgs}"
        roles = set(claims.get("realm_access", {}).get("roles", []))
        assert want_roles <= roles, f"{user}: roles {roles} missing {want_roles}"
        assert claims.get("email_verified") is True and claims.get("sid"), f"{user}: email_verified/sid missing"
        print(f"ok  {user}: organization={sorted(aliases)} roles⊇{sorted(want_roles)} sid+email_verified")

    # Wrong password must not authenticate (the login form is re-rendered, never a code redirect).
    _, challenge = pkce()
    probe = f"{disc['authorization_endpoint']}?" + urllib.parse.urlencode({
        "client_id": CLIENT_ID, "redirect_uri": redirect_uri, "response_type": "code", "scope": "openid",
        "code_challenge": challenge, "code_challenge_method": "S256",
    })
    try:
        kc_login(opener(new_jar()), probe, "bob", "wrong-password", redirect_uri)
    except SystemExit as e:
        assert "rejected" in str(e), f"unexpected failure mode: {e}"
        print("ok  wrong password rejected")
    else:
        raise AssertionError("login with a wrong password succeeded")


def api_json(method: str, url: str, body=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data, headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            raw = r.read()
            return r.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as e:
        return e.code, None


def ensure_tenants(api: str, admin_user: str, admin_password: str, slugs) -> None:
    """Organizations map to tenants.slug and are never auto-created (LAW-158-2): seed them first."""
    status, login = api_json("POST", f"{api}/api/v1/auth/login", {"username": admin_user, "password": admin_password})
    assert status == 200, f"admin login HTTP {status} (set EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD)"
    token = login["access_token"]
    _, listing = api_json("GET", f"{api}/api/v1/tenants", token=token)
    items = listing.get("items", listing) if isinstance(listing, dict) else (listing or [])
    have = {t.get("slug") for t in items}
    for slug in slugs:
        if slug in have:
            continue
        status, _ = api_json("POST", f"{api}/api/v1/tenants", {"name": slug.title(), "slug": slug}, token)
        assert status in (200, 201), f"create tenant {slug}: HTTP {status}"
    print(f"ok  tenants seeded: {sorted(slugs)}")


def part2(session_origin: str, web_callback: str, password: str, org: str, user: str) -> None:
    """Browser origin must equal OIDC redirect_uri (cookie + proxy)."""
    jar = new_jar()
    op = opener(jar)
    status, headers, _ = fetch(
        op,
        f"{session_origin}/api/v1/auth/oidc/login?" + urllib.parse.urlencode({"provider": "keycloak", "org": org}),
    )
    assert status in (302, 303, 307), f"login start HTTP {status}"
    final = kc_login(op, urllib.parse.urljoin(session_origin, headers["location"]), user, password, web_callback)
    qs = urllib.parse.parse_qs(urllib.parse.urlparse(final).query)
    assert "error" not in qs, f"SSO error redirect: {qs['error']}"
    assert not any(k in final for k in ("access_token", "refresh_token", "id_token")), "token leaked into URL"
    code = qs["code"][0]
    req = urllib.request.Request(
        f"{session_origin}/api/v1/auth/handoff",
        json.dumps({"code": code}).encode(),
        {"Content-Type": "application/json"},
    )
    body = json.loads(urllib.request.urlopen(req, timeout=30).read())
    claims = jwt_payload(body["access_token"])
    assert claims.get("tenant_id") and body["token_type"] == "Bearer"
    print(f"ok  api handoff: {user}@{org} → tenant {claims['tenant_id']} (user {body['user']['username']})")
    replay = urllib.request.Request(
        f"{session_origin}/api/v1/auth/handoff",
        json.dumps({"code": code}).encode(),
        {"Content-Type": "application/json"},
    )
    try:
        urllib.request.urlopen(replay, timeout=30)
        raise AssertionError("handoff code was reusable")
    except urllib.error.HTTPError as e:
        assert e.code == 401
        print("ok  api handoff code is single-use")


def origin_of(url: str) -> str:
    p = urllib.parse.urlparse(url)
    return f"{p.scheme}://{p.netloc}"


def kc_container(kc: str) -> str:
    """Keycloak container publishing this host port (in-container admin, no kcadm sslRequired edit)."""
    port = str(urllib.parse.urlparse(kc).port or 8081)
    out = subprocess.check_output(["docker", "ps", "--format", "{{.ID}}\t{{.Ports}}"], text=True)
    for line in out.splitlines():
        cid, _, ports = line.partition("\t")
        if f":{port}->" in ports:
            return cid
    raise SystemExit(f"no docker container publishing :{port}")


def kcadm(ctr: str, *args: str) -> str:
    return subprocess.check_output(
        ["docker", "exec", ctr, "/opt/keycloak/bin/kcadm.sh", *args],
        text=True,
    )


def kcadm_login(ctr: str, password: str) -> None:
    kcadm(
        ctr, "config", "credentials",
        "--server", "http://127.0.0.1:8081",
        "--realm", "master",
        "--user", "admin",
        "--password", password,
    )


def assert_edgequake_api(api: str) -> None:
    status, _, body = fetch(opener(new_jar()), f"{api.rstrip('/')}/health")
    try:
        data = json.loads(body)
    except json.JSONDecodeError as e:
        raise SystemExit(f"{api}/health is not EdgeQuake (HTTP {status}): {body[:120]!r}") from e
    if status != 200 or not isinstance(data, dict) or "storage_mode" not in data:
        raise SystemExit(f"{api}/health is not EdgeQuake: HTTP {status} {body[:120]!r}")


def part3_backchannel(kc: str, api: str, session_origin: str, web_callback: str, password: str, kc_admin_password: str) -> None:
    """Keycloak admin logout -> signed back-channel logout token -> EdgeQuake revokes the session.

    Needs Keycloak to reach `backchannel.logout.url` (compose overlay: http://api:8080;
    isolated proof: http://host.docker.internal:18080). Cookie routes use the browser origin.
    """
    jar = new_jar()
    op = opener(jar)
    _, headers, _ = fetch(
        op,
        f"{session_origin}/api/v1/auth/oidc/login?" + urllib.parse.urlencode({"provider": "keycloak", "org": "acme"}),
    )
    final = kc_login(op, urllib.parse.urljoin(session_origin, headers["location"]), "alice", password, web_callback)
    code = urllib.parse.parse_qs(urllib.parse.urlparse(final).query)["code"][0]
    session = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    access_token = None

    def post(path: str, body: dict):
        nonlocal access_token
        req = urllib.request.Request(
            f"{session_origin}{path}", json.dumps(body).encode(), {"Content-Type": "application/json"}
        )
        try:
            r = session.open(req, timeout=30)
            raw = r.read()
            if path.endswith("/handoff") and raw:
                access_token = json.loads(raw).get("access_token")
            return r.status
        except urllib.error.HTTPError as e:
            return e.code

    assert post("/api/v1/auth/handoff", {"code": code}) == 200
    assert access_token, "handoff must return an access token"
    assert post("/api/v1/auth/refresh", {}) == 200, "refresh should work before logout"

    ctr = kc_container(kc)
    kcadm_login(ctr, kc_admin_password)
    listed = json.loads(kcadm(ctr, "get", "users", "-r", REALM, "-q", "username=alice"))
    alice = next(u for u in listed if u.get("username") == "alice")
    kcadm(ctr, "create", f"users/{alice['id']}/logout", "-r", REALM)
    deadline = time.time() + 10
    while time.time() < deadline and post("/api/v1/auth/refresh", {}) == 200:
        time.sleep(0.5)
    assert post("/api/v1/auth/refresh", {}) == 401, "back-channel logout did not revoke the EdgeQuake session"
    me = urllib.request.Request(f"{api}/api/v1/auth/me", headers={"Authorization": f"Bearer {access_token}"})
    try:
        urllib.request.urlopen(me, timeout=30)
        raise AssertionError("access token still valid after back-channel logout")
    except urllib.error.HTTPError as e:
        assert e.code == 401, f"access token HTTP {e.code}"
    print("ok  back-channel logout revokes refresh and access (401)")


def _b64url(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode()


def _hs256(claims: dict, secret: str) -> str:
    header = _b64url(json.dumps({"alg": "HS256", "typ": "JWT"}, separators=(",", ":")).encode())
    payload = _b64url(json.dumps(claims, separators=(",", ":")).encode())
    sig = hmac.new(secret.encode(), f"{header}.{payload}".encode(), hashlib.sha256).digest()
    return f"{header}.{payload}.{_b64url(sig)}"


def start_oidc_stub(email: str, secret: str, issuer: str, subject: str, port: int = 0) -> ThreadingHTTPServer:
    """Minimal authorization-code IdP Keycloak can broker (EC-158-25)."""
    armed = {"code": "stub-code", "nonce": ""}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):  # noqa: ARG002
            return

        def _send(self, status, body, ctype="application/json"):
            data = body if isinstance(body, bytes) else body.encode()
            self.send_response(status)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            parsed = urllib.parse.urlparse(self.path)
            if parsed.path.endswith("/.well-known/openid-configuration"):
                self._send(200, json.dumps({
                    "issuer": issuer,
                    "authorization_endpoint": f"{issuer}/authorize",
                    "token_endpoint": f"{issuer}/token",
                    "jwks_uri": f"{issuer}/jwks",
                    "response_types_supported": ["code"],
                    "subject_types_supported": ["public"],
                    "id_token_signing_alg_values_supported": ["HS256"],
                }))
            elif parsed.path.endswith("/jwks"):
                self._send(200, json.dumps({"keys": []}))
            elif parsed.path.endswith("/authorize"):
                q = urllib.parse.parse_qs(parsed.query)
                armed["nonce"] = q.get("nonce", [""])[0]
                loc = q.get("redirect_uri", [""])[0]
                sep = "&" if "?" in loc else "?"
                loc = f"{loc}{sep}code={armed['code']}&state={q.get('state', [''])[0]}"
                self.send_response(302)
                self.send_header("Location", loc)
                self.end_headers()
            else:
                self._send(404, "{}")

        def do_POST(self):
            if not urllib.parse.urlparse(self.path).path.endswith("/token"):
                self._send(404, "{}")
                return
            now = int(time.time())
            token = _hs256({
                "iss": issuer, "sub": subject, "aud": "stub-client",
                "email": email, "email_verified": False, "preferred_username": "stub-user",
                "nonce": armed["nonce"], "iat": now, "exp": now + 300,
            }, secret)
            self._send(200, json.dumps({
                "access_token": "stub-at", "token_type": "Bearer", "id_token": token,
            }))

    httpd = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd


def part4_broker(kc: str, api: str, session_origin: str, web_callback: str, admin_user: str, admin_password: str, kc_admin_password: str) -> None:
    """EC-158-25: Keycloak OIDC IdP, trustEmail off, existing local email does not auto-link."""
    ctr = kc_container(kc)
    kcadm_login(ctr, kc_admin_password)
    skip_providers = {
        "idp-review-profile",
        "idp-add-organization-member",
        "idp-confirm-link",
    }
    executions = json.loads(
        kcadm(ctr, "get", "authentication/flows/first%20broker%20login/executions", "-r", REALM)
    )
    for exe in executions:
        if exe.get("providerId") in skip_providers and exe.get("requirement") != "DISABLED":
            kcadm(
                ctr, "update", "authentication/flows/first%20broker%20login/executions", "-r", REALM,
                "-b", json.dumps({"id": exe["id"], "requirement": "DISABLED"}),
            )
    for alias in ("VERIFY_PROFILE", "VERIFY_EMAIL"):
        kcadm(
            ctr, "update", f"authentication/required-actions/{alias}", "-r", REALM,
            "-s", "enabled=false", "-s", "defaultAction=false",
        )
    email = f"broker-{secrets.token_hex(4)}@corp.test"
    status, _ = api_json("POST", f"{api}/api/v1/users", {
        "username": f"local-{email.split('@')[0]}", "email": email, "password": "SecurePass123!",
    }, token=None)
    if status in (401, 403):
        st, login = api_json("POST", f"{api}/api/v1/auth/login", {"username": admin_user, "password": admin_password})
        assert st == 200, f"bootstrap admin login HTTP {st}"
        status, _ = api_json("POST", f"{api}/api/v1/users", {
            "username": f"local-{email.split('@')[0]}", "email": email, "password": "SecurePass123!",
        }, token=login["access_token"])
    assert status in (200, 201), f"local user create HTTP {status}"

    probe = ThreadingHTTPServer(("0.0.0.0", 0), BaseHTTPRequestHandler)
    port = probe.server_address[1]
    probe.server_close()
    pin_host("host.docker.internal", "127.0.0.1")
    issuer = f"http://host.docker.internal:{port}"
    httpd = start_oidc_stub(email, "stub-secret", issuer, f"stub-{secrets.token_hex(8)}", port=port)

    idp = {
        "alias": "stub-oidc",
        "providerId": "oidc",
        "enabled": True,
        "trustEmail": False,
        "config": {
            "clientId": "stub-client",
            "clientSecret": "stub-secret",
            "authorizationUrl": f"{issuer}/authorize",
            "tokenUrl": f"{issuer}/token",
            "jwksUrl": f"{issuer}/jwks",
            "issuer": issuer,
            "defaultScope": "openid email",
            "validateSignature": "false",
            "useJwksUrl": "true",
            "disableUserInfo": "true",
            "syncMode": "IMPORT",
            "hideOnLoginPage": "false",
            "guiOrder": "1",
            "updateProfileFirstLoginMode": "off",
        },
    }
    subprocess.run(
        ["docker", "exec", ctr, "/opt/keycloak/bin/kcadm.sh", "delete", "identity-provider/instances/stub-oidc", "-r", REALM],
        check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    idp_path = "/tmp/eq158-idp.json"
    with open(idp_path, "w", encoding="utf-8") as f:
        json.dump(idp, f)
    subprocess.check_call(["docker", "cp", idp_path, f"{ctr}:/tmp/idp.json"])
    kcadm(ctr, "create", "identity-provider/instances", "-r", REALM, "-f", "/tmp/idp.json")

    jar = new_jar()
    op = opener(jar)
    _, headers, _ = fetch(op, f"{session_origin}/api/v1/auth/oidc/login?" + urllib.parse.urlencode({"provider": "keycloak"}))
    loc = urllib.parse.urljoin(session_origin, headers["location"])
    if "kc_idp_hint" not in loc:
        loc += ("&" if "?" in loc else "?") + "kc_idp_hint=stub-oidc"
    final = kc_login(op, loc, "unused", "unused", web_callback)
    qs = urllib.parse.parse_qs(urllib.parse.urlparse(final).query)
    err = (qs.get("error") or [""])[0]
    httpd.shutdown()
    assert "account_exists_unlinked" in err, f"expected account_exists_unlinked, got {qs}"
    print("ok  keycloak oidc broker does not auto-link existing local email")


def part5_realm_restart(image: str) -> None:
    """EC-158-28: realm import is create-only; admin-created users survive restart."""
    net, pg, name, vol, port = "eq158-rr", "eq158-rr-pg", "eq158-realm-restart", "eq158-rr-pgdata", "18082"
    for cmd in (
        ["docker", "rm", "-f", name, pg],
        ["docker", "network", "rm", net],
        ["docker", "volume", "rm", "-f", vol],
    ):
        subprocess.run(cmd, check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.check_call(["docker", "network", "create", net], stdout=subprocess.DEVNULL)
    subprocess.check_call([
        "docker", "run", "-d", "--name", pg, "--network", net, "-v", f"{vol}:/var/lib/postgresql/data",
        "-e", "POSTGRES_DB=keycloak", "-e", "POSTGRES_USER=keycloak", "-e", "POSTGRES_PASSWORD=keycloak",
        "postgres:16-alpine",
    ], stdout=subprocess.DEVNULL)
    deadline = time.time() + 40
    while time.time() < deadline:
        if subprocess.run(["docker", "exec", pg, "pg_isready", "-U", "keycloak"], stdout=subprocess.DEVNULL).returncode == 0:
            break
        time.sleep(1)
    else:
        raise AssertionError("restart-proof postgres did not become ready")
    env = [
        "-e", "KC_DB=postgres", "-e", "KC_DB_URL=jdbc:postgresql://eq158-rr-pg:5432/keycloak",
        "-e", "KC_DB_USERNAME=keycloak", "-e", "KC_DB_PASSWORD=keycloak",
        "-e", "KC_BOOTSTRAP_ADMIN_USERNAME=admin", "-e", "KC_BOOTSTRAP_ADMIN_PASSWORD=admin_dev_only",
        "-e", "KC_HTTP_PORT=8081", "-e", "KC_HTTP_ENABLED=true", "-e", "KC_HOSTNAME_STRICT=false",
        "-e", "KC_HOSTNAME=http://localhost:18082",
        "-e", "EQ_KC_SSL_REQUIRED=none", "-e", "EQ_KC_CLIENT_SECRET=edgequake_dev_only_secret_change_me",
        "-e", "EQ_KC_DEMO_PASSWORD=demo-password-change-me",
        "-e", "EQ_API_PUBLIC_URL=http://localhost:8080", "-e", "EQ_API_INTERNAL_URL=http://api:8080",
    ]
    subprocess.check_call([
        "docker", "run", "-d", "--name", name, "--network", net, "-p", f"{port}:8081",
        *env, image, "start", "--optimized", "--import-realm",
    ], stdout=subprocess.DEVNULL)
    kc = f"http://localhost:{port}"

    def wait_ready(msg: str) -> None:
        until = time.time() + 180
        while time.time() < until:
            try:
                status, _, _ = fetch(opener(new_jar()), f"{kc}/realms/{REALM}/.well-known/openid-configuration")
                if status == 200:
                    return
            except Exception:
                pass
            time.sleep(2)
        raise AssertionError(msg)

    def kcadm(*args: str) -> str:
        cmd = [
            "docker", "exec", name, "/opt/keycloak/bin/kcadm.sh", *args,
        ]
        return subprocess.check_output(cmd, text=True)

    wait_ready("keycloak restart-proof container did not become ready")
    kcadm(
        "config", "credentials",
        "--server", "http://127.0.0.1:8081",
        "--realm", "master",
        "--user", "admin",
        "--password", "admin_dev_only",
    )
    kcadm("create", "users", "-r", REALM, "-s", "username=restart-survivor", "-s", "enabled=true", "-s", "email=survivor@eq.test")
    subprocess.check_call(["docker", "restart", name], stdout=subprocess.DEVNULL)
    wait_ready("keycloak did not come back after restart")
    kcadm(
        "config", "credentials",
        "--server", "http://127.0.0.1:8081",
        "--realm", "master",
        "--user", "admin",
        "--password", "admin_dev_only",
    )
    listed = kcadm("get", "users", "-r", REALM, "-q", "username=restart-survivor")
    assert "restart-survivor" in listed, listed
    ghosts = kcadm("get", "users", "-r", REALM, "-q", "username=json-v2-only")
    assert "json-v2-only" not in ghosts, ghosts
    subprocess.run(["docker", "rm", "-f", name, pg], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run(["docker", "network", "rm", net], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run(["docker", "volume", "rm", "-f", vol], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("ok  realm import skipped on restart (survivor kept, json-v2-only absent)")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--kc", default=os.environ.get("KC_PUBLIC_URL", "http://keycloak:8081"))
    ap.add_argument("--resolve", help="IP to pin the Keycloak hostname to (e.g. 127.0.0.1)")
    ap.add_argument("--api", help="EdgeQuake API base URL to also smoke the SSO handoff")
    ap.add_argument("--web-callback", default=os.environ.get("EQ_WEB_PUBLIC_URL", "http://localhost:3000") + "/auth/callback")
    ap.add_argument("--admin-user", default=os.environ.get("EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME", "admin"))
    ap.add_argument("--admin-password", default=os.environ.get("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD"))
    ap.add_argument("--deep", action="store_true", help="also verify back-channel logout (needs --api and KC->API reachability)")
    ap.add_argument("--broker", action="store_true", help="EC-158-25 Keycloak OIDC broker vs local stub")
    ap.add_argument("--realm-restart", action="store_true", help="EC-158-28 realm import skip on restart")
    ap.add_argument("--keycloak-image", default=os.environ.get("EDGEQUAKE_KEYCLOAK_IMAGE", "edgequake-keycloak:spec158"))
    ap.add_argument("--kc-admin-password", default=os.environ.get("KC_ADMIN_PASSWORD", "admin_dev_only"))
    ap.add_argument("--password", default=os.environ.get("EQ_KC_DEMO_PASSWORD", "demo-password-change-me"))
    ap.add_argument("--secret", default=os.environ.get("EQ_KC_CLIENT_SECRET", "edgequake_dev_only_secret_change_me"))
    ap.add_argument("--redirect-uri", default=os.environ.get("EQ_API_PUBLIC_URL", "http://localhost:8080") + "/api/v1/auth/oidc/callback")
    a = ap.parse_args()
    if a.resolve:
        pin_host(urllib.parse.urlparse(a.kc).hostname, a.resolve)
    part1(a.kc, a.password, a.secret, a.redirect_uri)
    if a.api:
        assert_edgequake_api(a.api)
        session_origin = origin_of(a.web_callback)
        if a.admin_password:
            ensure_tenants(a.api, a.admin_user, a.admin_password, ["acme", "globex"])
        part2(session_origin, a.web_callback, a.password, "acme", "alice")
        if a.deep:
            part3_backchannel(a.kc, a.api, session_origin, a.web_callback, a.password, a.kc_admin_password)
        if a.broker:
            part4_broker(a.kc, a.api, session_origin, a.web_callback, a.admin_user, a.admin_password, a.kc_admin_password)
    if a.realm_restart:
        part5_realm_restart(a.keycloak_image)
    print("KEYCLOAK SMOKE PASSED")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print(f"FAIL {e}", file=sys.stderr)
        sys.exit(1)
