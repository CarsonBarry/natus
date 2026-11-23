# Natus — agent rules

build and run
- build: `cargo build` (dev), `cargo build --release` (release). Pure Rust; no Node or Python runtime.

local CI (do not use GitHub Actions)
- verify with `sh scripts/ci.sh`: cargo fmt --check, clippy, `cargo test --workspace`
- the committed `.githooks/pre-push` runs it on every push once the clone has run `git config core.hooksPath .githooks`; `git push --no-verify` bypasses one push
- extend `scripts/ci.sh` rather than adding GitHub Actions workflows for checks that should run on every change
- keep CI files POSIX sh so the same committed files work on macOS, Linux and Windows (Git Bash)
