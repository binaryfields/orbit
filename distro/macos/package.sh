#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP="$TARGET_DIR/Orbit.app"
CONTENTS="$APP/Contents"

# Cargo.toml is the single source of truth for the version; the values in
# Info.plist are placeholders stamped at package time.
VERSION="$(cargo pkgid | sed 's/.*[#@]//')"

echo "==> cargo build --release"
cargo build --release

echo "==> assembling Orbit.app ($VERSION)"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"
cp "$HERE/Info.plist" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy \
    -c "Set :CFBundleVersion $VERSION" \
    -c "Set :CFBundleShortVersionString $VERSION" \
    "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"
cp "$TARGET_DIR/release/orbit" "$CONTENTS/MacOS/orbit"
if [ -f "$HERE/orbit.icns" ]; then
    cp "$HERE/orbit.icns" "$CONTENTS/Resources/orbit.icns"
else
    echo "    (no orbit.icns next to package.sh — provide one for an app icon)" >&2
fi

# Ad-hoc by default; set CODESIGN_IDENTITY to a Developer ID for distribution.
IDENTITY="${CODESIGN_IDENTITY:--}"
echo "==> codesign (identity: $IDENTITY)"
codesign --force --sign "$IDENTITY" "$APP"
codesign --verify "$APP"

echo
echo "Built $APP"
echo "  Install:  cp -R \"$APP\" /Applications/"
echo "  Run:      open \"$APP\""
