#!/usr/bin/env bash
# Read-only CoinBot status dashboard. Run from anywhere.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$ROOT/scripts/status.py" "$@"
