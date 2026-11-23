#!/bin/sh
# Local CI for natus. POSIX sh: macOS/Linux directly, Windows via Git Bash.
#
# One-time setup in each clone (the hook is committed at .githooks/pre-push):
#   git config core.hooksPath .githooks
set -eu

cd "$(git rev-parse --show-toplevel)"

echo "==> format"
cargo fmt --all -- --check

echo "==> clippy"
cargo clippy --workspace --all-targets

echo "==> tests"
cargo test --workspace

echo "==> local CI passed"
