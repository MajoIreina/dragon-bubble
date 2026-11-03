#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ "$(uname -s)" != Darwin ]]; then
    echo "Run this script on macOS." >&2
    exit 1
fi
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
if [[ "${1:-}" == --release ]]; then
    shift
    exec cargo run --locked --release -p pet-desktop -- "$@"
fi
exec cargo run --locked -p pet-desktop -- "$@"
