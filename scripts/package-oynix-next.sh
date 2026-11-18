#!/bin/sh
# Builds stonedgem-browser and packs it as the oynix-next preview:
#   dist/oynix-next-<version>-linux-x86_64.tar.gz
# with bin/oynix-next, share/applications/oynix-next.desktop, LICENSE and
# a README. Run as oynix-next it keeps its own profile (~/.config/oynix-next,
# ~/.local/share/oynix-next), so it never touches OyNIx's.
#
#   scripts/package-oynix-next.sh            # release build
#   PROFILE=dev scripts/package-oynix-next.sh # quicker, for testing
set -eu
cd "$(dirname "$0")/.."
PROFILE=${PROFILE:-release}
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --profile "$PROFILE" -p stonedgem-chrome
DIR=target/$( [ "$PROFILE" = dev ] && echo debug || echo "$PROFILE" )
NAME=oynix-next-$VERSION-linux-$(uname -m)
STAGE=dist/$NAME
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/share/applications"
install -m755 "$DIR/stonedgem-browser" "$STAGE/bin/oynix-next"
install -m644 crates/stonedgem-chrome/packaging/oynix-next.desktop "$STAGE/share/applications/"
install -m644 LICENSE "$STAGE/"
cat > "$STAGE/README.txt" <<TXT
OyNIx Next $VERSION (preview)

The next OyNIx, built on StonedGem: Servo renders pages, Iced draws the
window, the StonedGem blocker checks every request. It runs alongside
OyNIx with its own profile; nothing of OyNIx's is changed.

Run:      bin/oynix-next
Install:  copy bin/oynix-next to ~/.local/bin and
          share/applications/oynix-next.desktop to ~/.local/share/applications
Needs:    Mesa's EGL (libegl-mesa0 / mesa), fontconfig.
TXT
tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
echo "dist/$NAME.tar.gz"
