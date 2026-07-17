#!/bin/sh
set -eu

APP_NAME="Orbit"
BIN="orbit"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP="$TARGET_DIR/$APP_NAME.app"
CONTENTS="$APP/Contents"
# Ad-hoc by default; set CODESIGN_IDENTITY to a Developer ID for distribution.
IDENTITY="${CODESIGN_IDENTITY:--}"
# Cargo.toml is the single source of truth for the version; the values in
# Info.plist are placeholders stamped at package time.
VERSION="$(cargo pkgid | sed 's/.*[#@]//')"
ZIP="$TARGET_DIR/$APP_NAME-$VERSION.zip"

echo "==> cargo build --release"
cargo build --release

echo "==> assembling $APP_NAME.app ($VERSION)"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"
cp "$HERE/Info.plist" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy \
    -c "Set :CFBundleVersion $VERSION" \
    -c "Set :CFBundleShortVersionString $VERSION" \
    "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"
cp "$TARGET_DIR/release/$BIN" "$CONTENTS/MacOS/$BIN"
if [ -f "$HERE/$BIN.icns" ]; then
    cp "$HERE/$BIN.icns" "$CONTENTS/Resources/$BIN.icns"
else
    echo "    (no $BIN.icns next to package.sh — provide one for an app icon)" >&2
fi

echo "==> codesign (identity: $IDENTITY)"
codesign --force --sign "$IDENTITY" "$APP"
codesign --verify "$APP"

echo "==> zipping $ZIP"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

echo
echo "Built $APP"
echo "  Install:  cp -R \"$APP\" /Applications/"
echo "  Run:      open \"$APP\""
