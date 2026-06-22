#!/bin/sh
set -eu

AUTHORIZED_KEYS=${ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS:-/etc/antminer-fleet/client-tunnel/authorized_keys}
OWNER_USER=${ANTMINER_FLEET_CLIENT_TUNNEL_USER:-root}
OWNER_GROUP=${ANTMINER_FLEET_CLIENT_TUNNEL_GROUP:-antminer-fleet}

usage() {
  cat >&2 <<'USAGE'
Usage:
  revoke-client-tunnel-key.sh --label LABEL

Removes the restricted client SSH public key entry for LABEL from the configured
authorized_keys file. Keys are matched by the antminer-fleet-client:LABEL marker.

Exit codes:
  0  key removed
  2  label not found (already absent)
USAGE
}

LABEL=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --label)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      LABEL=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [ -z "$LABEL" ]; then
  echo "--label is required" >&2
  exit 2
fi

case "$LABEL" in
  *[!A-Za-z0-9._@+-]*|'')
    echo "Label may contain only letters, numbers, dot, underscore, at, plus, and dash" >&2
    exit 2
    ;;
esac

if [ ! -f "$AUTHORIZED_KEYS" ]; then
  echo "authorized_keys file does not exist: $AUTHORIZED_KEYS" >&2
  exit 2
fi

MARKER="antminer-fleet-client:$LABEL"
TMP_KEYS=$(mktemp)
trap 'rm -f "$TMP_KEYS"' EXIT HUP INT TERM

if ! grep -F "$MARKER" "$AUTHORIZED_KEYS" >/dev/null 2>&1; then
  echo "No authorized key found for label: $LABEL" >&2
  exit 2
fi

grep -Fv "$MARKER" "$AUTHORIZED_KEYS" >"$TMP_KEYS" || true
install -m 0660 -o "$OWNER_USER" -g "$OWNER_GROUP" "$TMP_KEYS" "$AUTHORIZED_KEYS"
echo "Revoked tunnel key for label: $LABEL"
