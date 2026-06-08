#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
VERSION=0.3.0
STAGE="$ROOT/server/package/antminer-fleet-server_${VERSION}_amd64"
OUTPUT="$ROOT/server/package/antminer-fleet-server_${VERSION}_amd64.deb"

cargo build --release -p antminer-fleet-server --manifest-path "$ROOT/Cargo.toml"
rm -rf "$STAGE"
install -d "$STAGE/DEBIAN" "$STAGE/usr/bin" "$STAGE/lib/systemd/system" "$STAGE/usr/share/doc/antminer-fleet-server"
install -m 0755 "$ROOT/target/release/antminer-fleet-server" "$STAGE/usr/bin/antminer-fleet-server"
install -m 0644 "$ROOT/server/packaging/antminer-fleet-server.service" "$STAGE/lib/systemd/system/antminer-fleet-server.service"
install -m 0644 "$ROOT/server/config/server.example.toml" "$STAGE/usr/share/doc/antminer-fleet-server/server.example.toml"
install -m 0644 "$ROOT/server/packaging/debian/control" "$STAGE/DEBIAN/control"
install -m 0755 "$ROOT/server/packaging/debian/postinst" "$STAGE/DEBIAN/postinst"
install -m 0755 "$ROOT/server/packaging/debian/prerm" "$STAGE/DEBIAN/prerm"
dpkg-deb --build --root-owner-group "$STAGE" "$OUTPUT"
echo "Built $OUTPUT"
