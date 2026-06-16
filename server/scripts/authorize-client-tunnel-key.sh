#!/bin/sh
set -eu

AUTHORIZED_KEYS=${ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS:-/etc/antminer-fleet/client-tunnel/authorized_keys}
OWNER_USER=${ANTMINER_FLEET_CLIENT_TUNNEL_USER:-antminer-fleet-client-tunnel}
OWNER_GROUP=${ANTMINER_FLEET_CLIENT_TUNNEL_GROUP:-antminer-fleet-client-tunnel}
PERMIT_OPEN=${ANTMINER_FLEET_CLIENT_TUNNEL_PERMIT_OPEN:-127.0.0.1:8443}

usage() {
  cat >&2 <<'USAGE'
Usage:
  authorize-client-tunnel-key.sh --label LABEL --public-key 'ssh-ed25519 AAAA... comment'
  authorize-client-tunnel-key.sh --label LABEL --public-key-file /path/to/id_ed25519.pub

Adds or replaces a restricted client SSH public key for the Antminer Fleet
local-forward tunnel account. The key is written to the configured authorized_keys
file with options that allow only TCP forwarding to the Fleet endpoint.

Environment overrides:
  ANTMINER_FLEET_CLIENT_TUNNEL_AUTHORIZED_KEYS  default /etc/antminer-fleet/client-tunnel/authorized_keys
  ANTMINER_FLEET_CLIENT_TUNNEL_USER             default antminer-fleet-client-tunnel
  ANTMINER_FLEET_CLIENT_TUNNEL_GROUP            default antminer-fleet-client-tunnel
  ANTMINER_FLEET_CLIENT_TUNNEL_PERMIT_OPEN      default 127.0.0.1:8443
USAGE
}

LABEL=
PUBLIC_KEY=
PUBLIC_KEY_FILE=
TMP_KEY=
TMP_KEYS=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --label)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      LABEL=$2
      shift 2
      ;;
    --public-key)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PUBLIC_KEY=$2
      shift 2
      ;;
    --public-key-file)
      [ "$#" -ge 2 ] || { usage; exit 2; }
      PUBLIC_KEY_FILE=$2
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

if [ -n "$PUBLIC_KEY" ] && [ -n "$PUBLIC_KEY_FILE" ]; then
  echo "Use either --public-key or --public-key-file, not both" >&2
  exit 2
fi

if [ -n "$PUBLIC_KEY_FILE" ]; then
  if [ ! -r "$PUBLIC_KEY_FILE" ]; then
    echo "Public key file is not readable: $PUBLIC_KEY_FILE" >&2
    exit 2
  fi
  PUBLIC_KEY=$(sed -n '1p' "$PUBLIC_KEY_FILE")
fi

if [ -z "$PUBLIC_KEY" ]; then
  echo "A public key is required" >&2
  usage
  exit 2
fi

case "$LABEL" in
  *[!A-Za-z0-9._@+-]*|'')
    echo "Label may contain only letters, numbers, dot, underscore, at, plus, and dash" >&2
    exit 2
    ;;
esac

KEY_TYPE=$(printf '%s\n' "$PUBLIC_KEY" | awk '{print $1}')
KEY_BODY=$(printf '%s\n' "$PUBLIC_KEY" | awk '{print $2}')
if [ -z "$KEY_TYPE" ] || [ -z "$KEY_BODY" ]; then
  echo "Public key must be in OpenSSH format" >&2
  exit 2
fi
case "$KEY_TYPE" in
  ssh-ed25519|ecdsa-sha2-nistp256|ecdsa-sha2-nistp384|ecdsa-sha2-nistp521|rsa-sha2-256|rsa-sha2-512|ssh-rsa)
    ;;
  *)
    echo "Unsupported public key type: $KEY_TYPE" >&2
    exit 2
    ;;
esac

if command -v ssh-keygen >/dev/null 2>&1; then
  TMP_KEY=$(mktemp)
  trap 'rm -f "$TMP_KEY" "$TMP_KEYS"' EXIT HUP INT TERM
  printf '%s\n' "$PUBLIC_KEY" >"$TMP_KEY"
  ssh-keygen -l -f "$TMP_KEY" >/dev/null 2>&1 || {
    echo "ssh-keygen rejected the public key" >&2
    exit 2
  }
else
  TMP_KEYS=
  trap 'rm -f "$TMP_KEYS"' EXIT HUP INT TERM
fi

AUTHORIZED_DIR=$(dirname -- "$AUTHORIZED_KEYS")
install -d -m 0750 -o "$OWNER_USER" -g "$OWNER_GROUP" "$AUTHORIZED_DIR"
touch "$AUTHORIZED_KEYS"
chown "$OWNER_USER:$OWNER_GROUP" "$AUTHORIZED_KEYS"
chmod 0640 "$AUTHORIZED_KEYS"

OPTIONS="restrict,port-forwarding,permitopen=\"$PERMIT_OPEN\",no-agent-forwarding,no-X11-forwarding,no-pty"
MARKER="antminer-fleet-client:$LABEL"
ENTRY="$OPTIONS $KEY_TYPE $KEY_BODY $MARKER"

TMP_KEYS=$(mktemp)
if [ -s "$AUTHORIZED_KEYS" ]; then
  grep -v " $MARKER$" "$AUTHORIZED_KEYS" >"$TMP_KEYS" || true
fi
printf '%s\n' "$ENTRY" >>"$TMP_KEYS"
install -m 0640 -o "$OWNER_USER" -g "$OWNER_GROUP" "$TMP_KEYS" "$AUTHORIZED_KEYS"

printf 'Authorized client tunnel key: %s\n' "$LABEL"
printf 'Authorized keys file: %s\n' "$AUTHORIZED_KEYS"
printf 'PermitOpen target: %s\n' "$PERMIT_OPEN"
