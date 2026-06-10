#!/bin/sh
set -eu

CONFIG=${ANTMINER_FLEET_TUNNEL_CONFIG:-/etc/antminer-fleet/tunnel.conf}

if [ ! -r "$CONFIG" ]; then
  echo "Tunnel configuration is not readable: $CONFIG" >&2
  exit 1
fi

# The configuration file is root-managed and contains shell assignments only.
# shellcheck disable=SC1090
. "$CONFIG"

: "${SSH_HOST:?SSH_HOST is required}"
: "${SSH_USER:?SSH_USER is required}"
: "${SSH_IDENTITY:?SSH_IDENTITY is required}"
: "${SSH_KNOWN_HOSTS:?SSH_KNOWN_HOSTS is required}"

REMOTE_BIND=${REMOTE_BIND:-127.0.0.1}
REMOTE_PORT=${REMOTE_PORT:-8443}
LOCAL_HOST=${LOCAL_HOST:-127.0.0.1}
LOCAL_PORT=${LOCAL_PORT:-8443}
RETRY_SECONDS=${RETRY_SECONDS:-5}

while :; do
  if curl --silent --show-error --fail --insecure --max-time 2 \
    "https://${LOCAL_HOST}:${LOCAL_PORT}/health" >/dev/null 2>&1; then
    ssh \
      -N \
      -T \
      -i "$SSH_IDENTITY" \
      -o BatchMode=yes \
      -o ExitOnForwardFailure=yes \
      -o ServerAliveInterval=30 \
      -o ServerAliveCountMax=3 \
      -o StrictHostKeyChecking=yes \
      -o UserKnownHostsFile="$SSH_KNOWN_HOSTS" \
      -R "${REMOTE_BIND}:${REMOTE_PORT}:${LOCAL_HOST}:${LOCAL_PORT}" \
      "${SSH_USER}@${SSH_HOST}" || true
  fi

  sleep "$RETRY_SECONDS"
done
