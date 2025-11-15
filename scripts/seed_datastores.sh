#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$ROOT_DIR/docker-compose.yml}"
WAIT_ATTEMPTS="${WAIT_ATTEMPTS:-40}"

if [[ ! -f "$COMPOSE_FILE" ]]; then
  echo "Cannot find docker-compose file at $COMPOSE_FILE" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required to seed the sample data." >&2
  exit 1
fi

PYTHON_BIN="${PYTHON_BIN:-python3}"
if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  echo "Python 3 (set via PYTHON_BIN) is required to seed Memcached." >&2
  exit 1
fi

MEMCACHED_HOST="${MEMCACHED_HOST:-127.0.0.1}"
MEMCACHED_PORT="${MEMCACHED_PORT:-11211}"

if [[ -n "${COMPOSE_BIN:-}" ]]; then
  # shellcheck disable=SC2206
  COMPOSE_CMD=($COMPOSE_BIN)
elif docker compose version >/dev/null 2>&1; then
  COMPOSE_CMD=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD=(docker-compose)
else
  echo "Neither 'docker compose' nor 'docker-compose' is available." >&2
  exit 1
fi

compose() {
  "${COMPOSE_CMD[@]}" -f "$COMPOSE_FILE" "$@"
}

log() {
  printf '[seed] %s\n' "$*"
}

wait_for_service() {
  local service="$1"
  shift
  local attempts=1
  while (( attempts <= WAIT_ATTEMPTS )); do
    if compose exec -T "$service" "$@" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
    attempts=$((attempts + 1))
  done
  echo "Timed out waiting for $service to become ready." >&2
  exit 1
}

memcached_send_file() {
  local payload_file="$1"
  "$PYTHON_BIN" - "$MEMCACHED_HOST" "$MEMCACHED_PORT" "$payload_file" <<'PY'
import socket
import sys
from pathlib import Path

host = sys.argv[1]
port = int(sys.argv[2])
payload_path = Path(sys.argv[3])
data = payload_path.read_bytes()

with socket.create_connection((host, port), timeout=5) as sock:
    sock.sendall(data)
PY
}

wait_for_memcached() {
  local attempts=1
  while (( attempts <= WAIT_ATTEMPTS )); do
    if "$PYTHON_BIN" - "$MEMCACHED_HOST" "$MEMCACHED_PORT" <<'PY' >/dev/null 2>&1; then
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])

with socket.create_connection((host, port), timeout=2) as sock:
    sock.sendall(b"version\r\n")
PY
      return 0
    fi
    sleep 2
    attempts=$((attempts + 1))
  done
  echo "Timed out waiting for memcached at ${MEMCACHED_HOST}:${MEMCACHED_PORT}." >&2
  exit 1
}

seed_redis() {
  log "Populating Redis with assorted data"
  compose exec -T redis redis-cli FLUSHALL >/dev/null

  compose exec -T redis redis-cli <<'EOF' >/dev/null
MULTI
SET app:config:version "1.3.0"
HSET app:config:flags readonly true telemetry_enabled true theme "nord"
LPUSH queue:emails "welcome:alice" "receipt:bob" "digest:charlie"
SADD feature:flags dark_mode live_reload beta_banner
ZADD leaderboard 980 alice 1020 bob 870 charlie 1200 dana
XADD streams:events * type signup user_id 1010 plan pro
XADD streams:events * type plan_change user_id 1001 plan enterprise
EXEC
EOF

  compose exec -T redis sh <<'EOF'
set -e
for i in $(seq 1 200); do
  tier="free"
  if [ $((i % 3)) -eq 0 ]; then
    tier="enterprise"
  elif [ $((i % 2)) -eq 0 ]; then
    tier="pro"
  fi
  redis-cli SET "session:$i" "token-$i"
  redis-cli HSET "user:$i" name "User $i" email "user$i@example.com" tier "$tier"
done
for bucket in $(seq 0 4); do
  for event in $(seq 1 20); do
    redis-cli LPUSH "events:$bucket" "$(date -u +%s)-event-$bucket-$event"
  done
done
EOF
}

seed_memcached() {
  log "Populating Memcached with assorted data"
  local tmpfile value bytes segment state remainder
  tmpfile="$(mktemp)"
  trap 'rm -f "$tmpfile"' RETURN
  {
    printf "flush_all\r\n"
    for i in $(seq 1 120); do
      if [ $((i % 2)) -eq 0 ]; then
        segment="consumer"
      else
        segment="enterprise"
      fi
      value=$(printf '{"id":%d,"name":"User %03d","segment":"%s"}' "$i" "$i" "$segment")
      bytes=${#value}
      printf "set users:%03d 0 1800 %d\r\n%s\r\n" "$i" "$bytes" "$value"
    done
    for i in $(seq 1 40); do
      remainder=$((i % 3))
      case $remainder in
        0) state="scheduled" ;;
        1) state="running" ;;
        *) state="finished" ;;
      esac
      value=$(printf "job-%02d|state=%s|duration=%ds" "$i" "$state" "$((20 + i))")
      bytes=${#value}
      printf "set jobs:%02d 0 600 %d\r\n%s\r\n" "$i" "$bytes" "$value"
    done
    printf "incr metrics:ingest 0\r\n"
    printf "incr metrics:alerts 0\r\n"
    printf "set app:config 0 3600 26\r\nregion=us-west-2|env=dev\r\n"
    printf "quit\r\n"
  } > "$tmpfile"

  memcached_send_file "$tmpfile"
  rm -f "$tmpfile"
  trap - RETURN
}

log "Starting backing services (if needed)"
compose up -d redis memcached >/dev/null

log "Waiting for Redis..."
wait_for_service redis redis-cli ping
log "Waiting for Memcached..."
wait_for_memcached

seed_redis
seed_memcached

log "Done! Sample data is now loaded."

