#!/usr/bin/env bash
# Safely free TCP listen PIDs on given ports without touching host container runtimes.
#
# First principles: OrbStack / Docker Desktop / docker-proxy are shared infrastructure.
# Killing them (or `open -ga OrbStack`) destabilizes every stack on the Mac.
#
# Usage:
#   ./scripts/dev_safe_kill_listen_port.sh 8090 3010
# Env:
#   EDGEQUAKE_SAFE_KILL_DRY_RUN=1  print what would be killed, do not kill
set -euo pipefail

DRY="${EDGEQUAKE_SAFE_KILL_DRY_RUN:-0}"

_is_host_runtime() {
  local cmd="$1"
  case "$cmd" in
    *[Oo]rb[Ss]tack*|*/OrbStack/*|*/.orbstack/*) return 0 ;;
    */Docker.app*|com.docker*|docker-proxy*|*/docker-desktop*) return 0 ;;
    *vpnkit*|*com.docker.backend*) return 0 ;;
  esac
  return 1
}

_kill_pid() {
  local pid="$1" port="$2" cmd="$3"
  if _is_host_runtime "$cmd"; then
    echo "  Skipping PID $pid on :$port (host container runtime — never kill): ${cmd:0:80}" >&2
    return 0
  fi
  if [ "$DRY" = "1" ]; then
    echo "  DRY-RUN would kill PID $pid on :$port: ${cmd:0:80}" >&2
    return 0
  fi
  echo "  Freeing :$port PID $pid" >&2
  kill -9 "$pid" 2>/dev/null || true
}

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <port> [port...]" >&2
  exit 2
fi

for port in "$@"; do
  [ -z "$port" ] && continue
  for pid in $(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null || true); do
    cmd=$(ps -p "$pid" -o command= 2>/dev/null || true)
    _kill_pid "$pid" "$port" "$cmd"
  done
done
