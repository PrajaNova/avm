#!/usr/bin/env bash
# Rebuild avm from this checkout and open a bash shell in a clean Linux
# container (see docker-compose.yml). Starts OrbStack/Docker Desktop on macOS
# if the Docker daemon isn't running. Nothing persists after you exit.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

command -v docker >/dev/null || { echo "docker not found; install OrbStack or Docker Desktop" >&2; exit 1; }

if ! docker info >/dev/null 2>&1; then
  if [ "$(uname -s)" != "Darwin" ]; then
    echo "Docker daemon is not running; start it and retry" >&2
    exit 1
  fi
  # Try OrbStack first, then Docker Desktop.
  open -a OrbStack 2>/dev/null || open -a Docker 2>/dev/null || {
    echo "Could not start OrbStack or Docker Desktop; start one manually" >&2
    exit 1
  }
  echo "Waiting for Docker to start..."
  for _ in $(seq 1 60); do
    docker info >/dev/null 2>&1 && break
    sleep 2
  done
  docker info >/dev/null 2>&1 || { echo "Docker did not start within 2 minutes" >&2; exit 1; }
fi

cd "$ROOT_DIR"
exec docker compose run --rm --build avm "$@"
