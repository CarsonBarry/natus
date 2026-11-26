# Natus

**Natus** (shortening of *Cincinnatus*) — a pure-Rust, cross-platform (macOS/Windows/Linux) dictation and feedback-capture tool.

## Purpose

Record feedback about applications you are developing as you test them. Natus pairs screenshots with spoken comments, takes screenshots automatically, and produces cleaned transcripts and bullet summaries. It strips filler words ("um", "uh", etc.) like Plaud and other dictation assistants.

## Features

- Audio recording (cpal) + transcription via local hangar (Parakeet) or any OpenAI-compatible STT endpoint.
- Automatic screenshot capture (xcap) paired with comments.
- Capture sequences: invoke once, then successive "capture" commands (voice or button).
- Filler-word removal and LLM summarization into bullets.
- Review UI: step through screenshots with their related text.
- Custom voice commands and script/macro triggers.
- Click-sequence recording and playback.
- Click-menu (GUI) for all voice commands.
- Wired to hangar for voice + LLM providers, with direct local/API provider fallback.

## Build

```sh
cargo build --release
cargo run
```

## Installer

Natus is packaged with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager)
(NSIS on Windows). Install the tooling once:

```sh
cargo install cargo-packager --locked
```

Windows also needs [NSIS](https://nsis.sourceforge.io/) (`makensis`) on `PATH`.

Build an installer:

```sh
# Windows
cargo packager --release --formats nsis
# macOS
cargo packager --release --formats dmg
# Linux
cargo packager --release --formats deb
```

The Windows installer lands at `target/release/natus_<version>_x64-setup.exe`
and installs Natus into `Program Files` with a Start Menu entry and an
uninstaller. `scripts/build-win.bat` runs both the build and the packaging step.

## Double-click scripts

`scripts/` contains a build and a launch script for each OS. Each one changes to the
repository root before running, so they work from wherever you double-click them.

| OS | Build | Launch |
| --- | --- | --- |
| Windows | `scripts/build-win.bat` | `scripts/launch-win.bat` |
| macOS | `scripts/build-mac.command` | `scripts/launch-mac.command` |
| Linux | `scripts/build-linux.sh` | `scripts/launch-linux.sh` |

Both launch scripts build a release binary first if one does not exist yet, then run
`target/release/natus` (`natus.exe` on Windows).

- **macOS**: `.command` files open in Terminal when double-clicked. If Finder doesn't
  open them, run `chmod +x scripts/*.command` once.
- **Linux**: make the `.sh` files executable with `chmod +x scripts/*.sh`. Some file
  managers (e.g. GNOME Files) need you to right-click → *Allow Launching* the first time.

## Configuration

Config lives in `~/.config/natus/config.json` (or platform equivalent). Defaults point at the local hangar (`http://127.0.0.1:8500`), parakeet STT, and the flagship chat model.

## Architecture

Pure Rust. GUI via egui/eframe. No web frontend, no Node/Python runtime.
EOF