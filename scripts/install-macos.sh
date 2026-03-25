#!/usr/bin/env bash
set -euo pipefail

# Install Task Hub from GitHub Releases (macOS only)
# Usage: bash scripts/install-macos.sh [version]
#   version: e.g. "0.1.0" (default: latest)

REPO="rysk-tanaka/task-hub-desktop"
APP_NAME="Task Hub"
VERSION="${1:-}"

case "$(uname -m)" in
  arm64) PATTERN="*aarch64.dmg" ;;
  x86_64) PATTERN="*x64.dmg" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if [ -n "$VERSION" ]; then
  echo "Downloading Task Hub v${VERSION}..."
  gh release download "v${VERSION}" --repo "$REPO" --pattern "$PATTERN" --dir "$TMPDIR"
else
  echo "Downloading Task Hub (latest)..."
  gh release download --repo "$REPO" --pattern "$PATTERN" --dir "$TMPDIR"
fi

DMG="$(find "$TMPDIR" -name '*.dmg' | head -1)"
if [ -z "$DMG" ]; then
  echo "Error: DMG not found" >&2
  exit 1
fi

MOUNT_POINT="$(hdiutil attach "$DMG" -nobrowse \
  | awk -F$'\t' 'NF>=3 && $3 ~ /^\// {print $3; exit}')"
trap 'hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null; rm -rf "$TMPDIR"' EXIT

DEST="/Applications/${APP_NAME}.app"
STAGING="${TMPDIR}/${APP_NAME}.app"

echo "Installing to ${DEST}..."
cp -R "${MOUNT_POINT}/${APP_NAME}.app" "$STAGING"

# Atomic replace: copy succeeds before removing the old version
if [ -d "$DEST" ]; then
  rm -rf "$DEST"
fi
mv "$STAGING" "$DEST"

echo "Done."
