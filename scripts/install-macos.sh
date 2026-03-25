#!/usr/bin/env bash
set -euo pipefail

# Install Task Hub from GitHub Releases (macOS only)
# Usage: bash scripts/install-macos.sh [version]
#   version: e.g. "0.1.0" (default: latest)

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This installer is for macOS only." >&2
  exit 1
fi

REPO="rysk-tanaka/task-hub-desktop"
APP_NAME="Task Hub"
VERSION="${1:-}"

case "$(uname -m)" in
  arm64) PATTERN="*aarch64.dmg" ;;
  x86_64) PATTERN="*x64.dmg" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

if [ -n "$VERSION" ]; then
  echo "Downloading Task Hub v${VERSION}..."
  gh release download "v${VERSION}" --repo "$REPO" --pattern "$PATTERN" --dir "$WORK_DIR"
else
  echo "Downloading Task Hub (latest)..."
  gh release download --repo "$REPO" --pattern "$PATTERN" --dir "$WORK_DIR"
fi

DMG="$(find "$WORK_DIR" -name '*.dmg' | head -1)"
if [ -z "$DMG" ]; then
  echo "Error: DMG not found" >&2
  exit 1
fi

MOUNT_POINT="$(hdiutil attach "$DMG" -nobrowse \
  | awk -F$'\t' 'NF>=3 && $3 ~ /^\// {print $3; exit}')"
if [ -z "$MOUNT_POINT" ]; then
  echo "Error: Failed to mount DMG" >&2
  exit 1
fi
trap 'hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null; rm -rf "$WORK_DIR"' EXIT

DEST="/Applications/${APP_NAME}.app"
STAGING="${WORK_DIR}/${APP_NAME}.app"

echo "Installing to ${DEST}..."
ditto "${MOUNT_POINT}/${APP_NAME}.app" "$STAGING"

# Safe replace: back up old version, install new, roll back on failure
BACKUP="${DEST}.backup.$$"
if [ -d "$DEST" ]; then
  mv "$DEST" "$BACKUP"
fi
if mv "$STAGING" "$DEST"; then
  rm -rf "$BACKUP" 2>/dev/null || true
else
  if [ -d "$BACKUP" ]; then
    mv "$BACKUP" "$DEST"
  fi
  echo "Error: Failed to install ${APP_NAME}" >&2
  exit 1
fi

echo "Done."
