#!/usr/bin/env bash
# Isolated SPEC-158 live stack: Keycloak :18081, working-tree API :18080, WebUI :13010.
# Never binds :8080/:8081 and never kills OrbStack.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE=(docker compose -f "$ROOT/docker-compose.spec158-proof.yml")
API_PORT="${SPEC158_API_PORT:-18080}"
WEB_PORT="${SPEC158_WEB_PORT:-13010}"
KC_PORT="${KC_PORT:-18081}"
ADMIN_PASSWORD="${EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD:-Admin-dev-only-change-me-1}"
DB_URL="${SPEC158_DATABASE_URL:-postgres://edgequake:edgequake_secret@127.0.0.1:5432/edgequake_sso_proof}"
API_PID_FILE=/tmp/edgequake-spec158-api.pid
WEB_PID_FILE=/tmp/edgequake-spec158-web.pid
API_LOG=/tmp/edgequake-spec158-api.log
WEB_LOG=/tmp/edgequake-spec158-web.log
IMAGE="${EDGEQUAKE_KEYCLOAK_IMAGE:-edgequake-keycloak:spec158}"

eq_health() {
  python3 - "$1" <<'PY'
import json, sys, urllib.request
url = sys.argv[1].rstrip("/") + "/health"
try:
    with urllib.request.urlopen(url, timeout=3) as r:
        body = r.read()
    data = json.loads(body)
    sys.exit(0 if isinstance(data, dict) and "storage_mode" in data else 1)
except Exception:
    sys.exit(1)
PY
}

stop_pidfile() {
  local f="$1"
  if [ -f "$f" ]; then
    kill "$(cat "$f")" 2>/dev/null || true
    rm -f "$f"
  fi
}

cmd="${1:-up}"

case "$cmd" in
  down)
    stop_pidfile "$API_PID_FILE"
    stop_pidfile "$WEB_PID_FILE"
    bash "$ROOT/scripts/dev_safe_kill_listen_port.sh" "$API_PORT" "$WEB_PORT"
    "${COMPOSE[@]}" down --remove-orphans
    echo "spec158 proof stack stopped (existing :8080/:8081 containers left alone)"
    ;;
  up)
    if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
      echo "missing image $IMAGE — build with: docker build -f deploy/keycloak/Containerfile -t $IMAGE deploy/keycloak" >&2
      exit 1
    fi
    EDGEQUAKE_KEYCLOAK_IMAGE="$IMAGE" \
      EQ_API_PUBLIC_URL="http://localhost:${WEB_PORT}" \
      EQ_API_INTERNAL_URL="http://host.docker.internal:${API_PORT}" \
      KC_PUBLIC_URL="http://localhost:${KC_PORT}" \
      "${COMPOSE[@]}" up -d
    echo "waiting for Keycloak on :${KC_PORT}"
    for _ in $(seq 1 60); do
      if curl -sf "http://127.0.0.1:${KC_PORT}/realms/edgequake/.well-known/openid-configuration" >/dev/null; then
        break
      fi
      sleep 2
    done
    curl -sf "http://127.0.0.1:${KC_PORT}/realms/edgequake/.well-known/openid-configuration" >/dev/null
    KC_CTR="$("${COMPOSE[@]}" ps -q keycloak)"
    docker exec "$KC_CTR" /opt/keycloak/bin/kcadm.sh config credentials \
      --server http://127.0.0.1:8081 --realm master --user admin --password "${KC_ADMIN_PASSWORD:-admin_dev_only}" >/dev/null
    CID="$(docker exec "$KC_CTR" /opt/keycloak/bin/kcadm.sh get clients -r edgequake -q clientId=edgequake-web --fields id --format csv --noquotes | tail -1 | tr -d '\r')"
    docker exec "$KC_CTR" /opt/keycloak/bin/kcadm.sh get "clients/${CID}" -r edgequake > /tmp/eq158-client.json
    python3 - "$WEB_PORT" "$API_PORT" <<'PY'
import json, sys
web, api = sys.argv[1], sys.argv[2]
c = json.load(open("/tmp/eq158-client.json"))
c["redirectUris"] = sorted(set(c.get("redirectUris") or []) | {
    f"http://localhost:{web}/api/v1/auth/oidc/callback",
    f"http://localhost:{api}/api/v1/auth/oidc/callback",
})
c.setdefault("attributes", {})["backchannel.logout.url"] = f"http://host.docker.internal:{api}/api/v1/auth/oidc/backchannel-logout"
json.dump(c, open("/tmp/eq158-client.out.json", "w"))
PY
    docker cp /tmp/eq158-client.out.json "$KC_CTR":/tmp/client.out.json
    docker exec "$KC_CTR" /opt/keycloak/bin/kcadm.sh update "clients/${CID}" -r edgequake -f /tmp/client.out.json

    if docker ps --format '{{.Names}}' | grep -qx edgequake-postgres; then
      docker exec edgequake-postgres psql -U edgequake -d postgres -tc \
        "SELECT 1 FROM pg_database WHERE datname='edgequake_sso_proof'" | grep -q 1 \
        || docker exec edgequake-postgres createdb -U edgequake edgequake_sso_proof
    fi

    echo "migrate $DB_URL"
    (
      cd "$ROOT/edgequake"
      EDGEQUAKE_MIGRATE_CLI=1 DATABASE_URL="$DB_URL" \
        EDGEQUAKE_LLM_PROVIDER=mock EDGEQUAKE_EMBEDDING_PROVIDER=mock EDGEQUAKE_ALLOW_MOCK_PROVIDER=1 \
        cargo run --quiet -- migrate
    )

    echo "build API"
    (cd "$ROOT/edgequake" && cargo build --quiet --bin edgequake)
    _TDIR="${CARGO_TARGET_DIR:-$(cd "$ROOT/edgequake" && cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')}"
    BIN="${_TDIR}/debug/edgequake"
    if [ ! -x "$BIN" ]; then
      echo "missing binary $BIN" >&2
      exit 1
    fi
    if eq_health "http://127.0.0.1:${API_PORT}"; then
      echo "API already healthy on :${API_PORT}"
    else
      bash "$ROOT/scripts/dev_safe_kill_listen_port.sh" "$API_PORT"
      export DATABASE_URL="$DB_URL"
      export EDGEQUAKE_HOST=0.0.0.0
      export EDGEQUAKE_PORT="$API_PORT"
      export PORT="$API_PORT"
      export EDGEQUAKE_LLM_PROVIDER=mock
      export EDGEQUAKE_EMBEDDING_PROVIDER=mock
      export EDGEQUAKE_ALLOW_MOCK_PROVIDER=1
      export EDGEQUAKE_SCHEMA_GATE=wait
      export EDGEQUAKE_AUTH_ENABLED=true
      export AUTH_ENABLED=true
      export EDGEQUAKE_STRICT_TENANT_BIND=true
      export JWT_SECRET="${JWT_SECRET:-spec158-proof-jwt-secret-32bytes-min}"
      export EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
      export EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD="$ADMIN_PASSWORD"
      export EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@localhost
      export EDGEQUAKE_OIDC_ENABLED=true
      export EDGEQUAKE_OIDC_KIND=keycloak
      export EDGEQUAKE_OIDC_SLUG=keycloak
      export EDGEQUAKE_OIDC_DISPLAY_NAME="Sign in with Keycloak"
      export EDGEQUAKE_OIDC_ISSUER_URL="http://localhost:${KC_PORT}/realms/edgequake"
      export EDGEQUAKE_OIDC_CLIENT_ID=edgequake-web
      export EDGEQUAKE_OIDC_CLIENT_SECRET="${EQ_KC_CLIENT_SECRET:-edgequake_dev_only_secret_change_me}"
      export EDGEQUAKE_OIDC_REDIRECT_URI="http://localhost:${WEB_PORT}/api/v1/auth/oidc/callback"
      export EDGEQUAKE_OIDC_SUCCESS_REDIRECT_URL="http://localhost:${WEB_PORT}/auth/callback"
      export EDGEQUAKE_OIDC_ROLE_MAP='{"eq-admin":"admin","eq-user":"member"}'
      export RUST_LOG="${RUST_LOG:-edgequake_api=info,edgequake=info}"
      python3 - "$BIN" "$API_LOG" "$API_PID_FILE" <<'PY'
import os, sys
binary, log, pidf = sys.argv[1], sys.argv[2], sys.argv[3]
if os.fork():
    sys.exit(0)
os.setsid()
if os.fork():
    sys.exit(0)
fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
os.dup2(fd, 1)
os.dup2(fd, 2)
os.close(fd)
dn = os.open("/dev/null", os.O_RDONLY)
os.dup2(dn, 0)
os.close(dn)
with open(pidf, "w", encoding="utf-8") as f:
    f.write(str(os.getpid()))
os.execv(binary, [binary])
PY
      for _ in $(seq 1 60); do
        eq_health "http://127.0.0.1:${API_PORT}" && break
        sleep 1
      done
      eq_health "http://127.0.0.1:${API_PORT}"
    fi

    if curl -sf "http://127.0.0.1:${WEB_PORT}/" >/dev/null; then
      echo "WebUI already on :${WEB_PORT}"
    else
      # Next 16 allows one dev server per app dir; free the default :3010 lock.
      bash "$ROOT/scripts/dev_safe_kill_listen_port.sh" "$WEB_PORT" 3010
      (
        cd "$ROOT/edgequake_webui"
        export PORT="$WEB_PORT"
        export EDGEQUAKE_API_URL="http://127.0.0.1:${API_PORT}"
        export NEXT_PUBLIC_API_URL="http://127.0.0.1:${API_PORT}"
        export NEXT_PUBLIC_AUTH_ENABLED=true
        nohup bun x next dev --port "$WEB_PORT" >"$WEB_LOG" 2>&1 < /dev/null &
        web_pid=$!
        disown "$web_pid" 2>/dev/null || true
        echo "$web_pid" >"$WEB_PID_FILE"
      )
      for _ in $(seq 1 60); do
        curl -sf "http://127.0.0.1:${WEB_PORT}/" >/dev/null && break
        sleep 1
      done
    fi
    echo "API  http://127.0.0.1:${API_PORT}"
    echo "Web  http://127.0.0.1:${WEB_PORT}"
    echo "KC   http://localhost:${KC_PORT}"
    ;;
  *)
    echo "usage: $0 up|down" >&2
    exit 2
    ;;
esac
