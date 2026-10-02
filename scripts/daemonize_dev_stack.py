#!/usr/bin/env python3
"""Double-fork EdgeQuake backend+frontend so agent shells cannot SIGTERM them."""
from __future__ import annotations

import os
import pathlib
import sys
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
BACKEND_SCRIPT = pathlib.Path("/tmp/edgequake-launch-backend.sh")
FRONTEND_SCRIPT = pathlib.Path("/tmp/edgequake-launch-frontend.sh")


def double_fork(script: pathlib.Path, logfile: pathlib.Path) -> None:
    if os.fork() > 0:
        return
    os.setsid()
    if os.fork() > 0:
        os._exit(0)
    with logfile.open("a") as f:
        os.dup2(f.fileno(), 1)
        os.dup2(f.fileno(), 2)
    dn = os.open("/dev/null", os.O_RDONLY)
    os.dup2(dn, 0)
    os.close(dn)
    os.execv("/bin/bash", ["bash", str(script)])


def wait_url(url: str, timeout: float = 90.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=2) as r:
                body = r.read().decode("utf-8", "replace")
                if "healthy" in body or r.status == 200:
                    return True
        except Exception:
            pass
        time.sleep(1)
    return False


def main() -> int:
    if not BACKEND_SCRIPT.exists() or not FRONTEND_SCRIPT.exists():
        print("missing /tmp/edgequake-launch-*.sh — run make sync-dev-ports first", file=sys.stderr)
        return 1
    pathlib.Path("/tmp/edgequake-backend.log").write_text("")
    pathlib.Path("/tmp/edgequake-frontend.log").write_text("")
    double_fork(BACKEND_SCRIPT, pathlib.Path("/tmp/edgequake-backend.log"))
    double_fork(FRONTEND_SCRIPT, pathlib.Path("/tmp/edgequake-frontend.log"))
    ok_api = wait_url("http://127.0.0.1:8091/health")
    ok_ui = wait_url("http://127.0.0.1:3010/health")
    print(f"api={'ok' if ok_api else 'FAIL'} ui={'ok' if ok_ui else 'FAIL'}")
    return 0 if ok_api and ok_ui else 1


if __name__ == "__main__":
    raise SystemExit(main())
