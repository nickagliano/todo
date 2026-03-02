#!/usr/bin/env bash
set -euo pipefail

# simple_todo/run.sh — entry point for this EPS harness
#
# Edit this file to wire up your ports.
# See CUSTOMIZE.md for a description of each extension point.

cargo run --quiet -- "$@"
