#!/usr/bin/env bash
# Gate: EdgeQuake Make/scripts must never open, wake, quit, or kill OrbStack /
# Docker Desktop. Regression: `open -ga OrbStack` triggered VM handoff (vmgr)
# and dropped the Docker socket during make_dev.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

# Executable / recipe surfaces only (not this gate file, not docs).
SCAN_PATHS=(
  "$ROOT/Makefile"
)
# All scripts except this gate (its patterns match themselves).
while IFS= read -r -d '' f; do
  SCAN_PATHS+=("$f")
done < <(find "$ROOT/scripts" -type f \( -name '*.sh' -o -name '*.py' \) \
  ! -name 'check_no_orbstack_kill.sh' -print0 2>/dev/null)

# Ban patterns that actively touch host container runtimes.
# Allow comments that document the ban (lines starting with # or containing NEVER /
# "will NOT" / "Do not" / "do not" / "never").
_is_doc_or_ban_comment() {
  local line="$1"
  case "$line" in
    *"NEVER"*|*"never"*|*"will NOT"*|*"Do not"*|*"do not"*|*"must not"*|*"Must not"*) return 0 ;;
  esac
  # Makefile recipe comment or shell comment
  if printf '%s' "$line" | grep -Eq '^[[:space:]]*#|^[[:space:]]*@?#'; then
    return 0
  fi
  return 1
}

check_pattern() {
  local label="$1"
  local pattern="$2"
  local hits
  hits=$(rg -n --no-heading -e "$pattern" "${SCAN_PATHS[@]}" 2>/dev/null || true)
  if [ -z "$hits" ]; then
    echo "✓ no matches: $label"
    return 0
  fi
  local bad=0
  while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    local file_line="${hit%%:*}"
    local rest="${hit#*:}"
    local lineno="${rest%%:*}"
    local text="${rest#*:}"
    if _is_doc_or_ban_comment "$text"; then
      continue
    fi
    echo "✗ BANNED ($label): $file_line:$lineno: $text" >&2
    bad=1
  done <<< "$hits"
  if [ "$bad" = "1" ]; then
    FAIL=1
  else
    echo "✓ only documented bans: $label"
  fi
}

echo "→ Checking Make/scripts for OrbStack/Docker Desktop kill/wake/quit..."

check_pattern 'open -ga OrbStack/Docker' 'open[[:space:]]+-g?a[[:space:]]+(OrbStack|Docker)\b'
check_pattern 'killall OrbStack/Docker' 'killall[[:space:]].*[Oo]rb[Ss]tack|killall[[:space:]].*Docker'
check_pattern 'pkill OrbStack' 'pkill[[:space:]].*[Oo]rb[Ss]tack'
check_pattern 'orb stop/restart/down' '\borb[[:space:]]+(stop|restart|down|quit)\b'
check_pattern 'osascript quit OrbStack' 'osascript.*[Oo]rb[Ss]tack'

# Blind port kills without the safe helper are a footgun (can hit docker-proxy).
# Flag raw `lsof -ti:PORT | xargs kill` in Makefile/scripts.
blind=$(rg -n --no-heading -e 'lsof[[:space:]]+-ti:[0-9]+[[:space:]]*\|[[:space:]]*xargs[[:space:]]+kill' \
  "$ROOT/Makefile" "$ROOT/scripts" 2>/dev/null || true)
if [ -n "$blind" ]; then
  echo "✗ BANNED blind lsof|xargs kill (use scripts/dev_safe_kill_listen_port.sh):" >&2
  echo "$blind" >&2
  FAIL=1
else
  echo "✓ no blind lsof|xargs kill in Makefile/scripts"
fi

# Ensure soft-wake path is gone from ensure script.
if rg -n --no-heading -e 'open[[:space:]]+-g?a' "$ROOT/scripts/dev_ensure_docker_daemon.sh" 2>/dev/null | grep -v '#' >/dev/null; then
  echo "✗ scripts/dev_ensure_docker_daemon.sh still contains active open -ga" >&2
  FAIL=1
else
  echo "✓ dev_ensure_docker_daemon.sh has no active open -ga"
fi

if [ "$FAIL" = "1" ]; then
  echo "✗ OrbStack cohabitation gate FAILED" >&2
  exit 1
fi
echo "✓ OrbStack cohabitation gate PASSED (Make never opens/kills/restarts OrbStack)"
exit 0
