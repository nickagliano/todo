#!/usr/bin/env bash
set -euo pipefail

# simple_todo/serve.sh — EPC service entry point
#
# EPC calls this script to start simple_todo as a persistent HTTP service.
# The server binds to the port declared in eps.toml [service].port.
#
# To test manually:
#   ./serve.sh
#   curl http://localhost:8765/tasks

cd "$(dirname "$0")"

cargo build --release --quiet
exec ./target/release/simple_todo serve --port "${PORT:-8765}"
