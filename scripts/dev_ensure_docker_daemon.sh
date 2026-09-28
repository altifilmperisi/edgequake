#!/usr/bin/env bash
# Probe-only Docker daemon check for make db-start / make dev.
#
# First principles (cohabitation on a shared developer Mac):
#   1. NEVER open / quit / kill / restart OrbStack or Docker Desktop.
#      `open -ga OrbStack` triggers a VM handoff (vmgr) that drops the Docker
#      socket and kills running containers — that is what broke make dev.
#   2. Reuse an already-running daemon when present.
#   3. If the daemon is down, fail immediately and tell the human to open
#      OrbStack themselves. Make must not touch host container runtimes.
#
# Exit 0 when `docker info` succeeds; exit 1 when unavailable.
set -euo pipefail

_docker_ready() {
  command -v docker >/dev/null 2>&1 || return 1
  docker info >/dev/null 2>&1
}

if ! command -v docker >/dev/null 2>&1; then
  echo "✗ Docker CLI is not installed" >&2
  exit 1
fi

if _docker_ready; then
  echo "✓ Docker daemon already available" >&2
  exit 0
fi

echo "✗ Docker daemon unavailable — Make will NOT open/wake OrbStack or Docker Desktop" >&2
echo "  Open OrbStack.app (or Docker Desktop) yourself, wait until 'docker info' works," >&2
echo "  then rerun make db-start / make dev. Existing Postgres on 5432–5449 is reused if reachable." >&2
exit 1
