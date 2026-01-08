#!/usr/bin/env bash
set -e

# =========================
# 多 target 配置
# =========================

TARGETS=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
)

ARCHIVE_EXT="tar.gz"

BINS=(
  "proxyd"
  "portald"
  "portal_hub_grpc"
  "portal_hub_rest"
)

# =========================
# 主循环
# =========================

for TARGET in "${TARGETS[@]}"; do
  echo "=============================="
  echo "Packaging target: $TARGET"
  echo "=============================="

  STAGING="remote_rpc_rs-${TARGET}"
  RELEASE_DIR="target/${TARGET}/release"

  rm -rf "$STAGING"
  mkdir -p "$STAGING"

  for bin in "${BINS[@]}"; do
    if [ -f "$RELEASE_DIR/$bin.exe" ]; then
      cp "$RELEASE_DIR/$bin.exe" "$STAGING/"
    elif [ -f "$RELEASE_DIR/$bin" ]; then
      cp "$RELEASE_DIR/$bin" "$STAGING/"
    else
      echo "⚠️  Skip $bin (not found for $TARGET)"
    fi
  done

  if [ "$ARCHIVE_EXT" = "zip" ]; then
    7z a "${STAGING}.zip" "$STAGING"
    echo "Output: ${STAGING}.zip"
  else
    tar -czvf "${STAGING}.tar.gz" "$STAGING"
    echo "Output: ${STAGING}.tar.gz"
  fi

  echo
done
