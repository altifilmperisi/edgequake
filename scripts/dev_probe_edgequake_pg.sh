#!/usr/bin/env bash
# SPEC / make_dev cohabitation: find a reachable EdgeQuake Postgres without Docker.
#
# WHY: When OrbStack/Docker is briefly unavailable (VPN flap, sleep wake), the
# edgequake-postgres *port forward* may still be up, or a sibling stack may already
# expose credentials on 5432–5449. Reuse beats recreate — never kill host daemons.
#
# Usage:
#   URL=$(./scripts/dev_probe_edgequake_pg.sh [base_database_url])
#   # exit 0 + URL on stdout when auth succeeds; exit 1 when nothing matches.
#
# Probe order:
#   1. /tmp/edgequake-db-url (last successful make db-start)
#   2. base_database_url (default DATABASE_URL / Makefile default)
#   3. localhost ports 5432–5449 with the same user/pass/db as the base URL
set -euo pipefail

BASE_URL="${1:-${DATABASE_URL:-postgresql://edgequake:edgequake_secret@localhost:5432/edgequake}}"
CACHE_FILE="${EDGEQUAKE_DB_URL_FILE:-/tmp/edgequake-db-url}"

_parse() {
  local url="$1"
  _DB_USER=$(printf '%s' "$url" | sed -E 's|^[^:]+://([^:]+):.*|\1|')
  _DB_PASS=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|')
  _DB_HOST=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|')
  _DB_PORT=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|')
  _DB_PORT="${_DB_PORT:-5432}"
  _DB_NAME=$(printf '%s' "$url" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|')
  _DB_QS=$(printf '%s' "$url" | sed -nE 's|^[^?]+\?(.*)$|\1|p')
}

_auth_ok() {
  local host="$1" port="$2" user="$3" pass="$4" name="$5"
  command -v pg_isready >/dev/null 2>&1 || return 1
  command -v psql >/dev/null 2>&1 || return 1
  pg_isready -h "$host" -p "$port" >/dev/null 2>&1 || return 1
  PGPASSWORD="$pass" psql -h "$host" -p "$port" -U "$user" -d "$name" -c '\q' >/dev/null 2>&1
}

_emit() {
  local url="$1" why="$2"
  printf '%s\n' "$url"
  printf '✓ Reusing PostgreSQL (%s)\n' "$why" >&2
  exit 0
}

_try_url() {
  local url="$1" why="$2"
  [ -n "$url" ] || return 1
  _parse "$url"
  if _auth_ok "$_DB_HOST" "$_DB_PORT" "$_DB_USER" "$_DB_PASS" "$_DB_NAME"; then
    _emit "$url" "$why"
  fi
  return 1
}

# 1) Cached effective URL from a prior successful db-start
if [ -f "$CACHE_FILE" ]; then
  _cached=$(tr -d '\n' <"$CACHE_FILE" 2>/dev/null || true)
  _try_url "$_cached" "cached $CACHE_FILE" || true
fi

# 2) Caller / env base URL
_try_url "$BASE_URL" "DATABASE_URL credentials on default host/port" || true

# 3) Same credentials across cohabitation port band (other stacks / remaps)
_parse "$BASE_URL"
for _TRY in 5432 5433 5434 5435 5436 5437 5438 5439 5440 5441 5442 5443 5444 5445 5446 5447 5448 5449; do
  if _auth_ok localhost "$_TRY" "$_DB_USER" "$_DB_PASS" "$_DB_NAME"; then
    _qs=""
    [ -n "${_DB_QS:-}" ] && _qs="?${_DB_QS}"
    _emit "postgresql://${_DB_USER}:${_DB_PASS}@localhost:${_TRY}/${_DB_NAME}${_qs}" \
      "edgequake credentials on localhost:${_TRY}"
  fi
done

exit 1
