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

## Configuration

Config lives in `~/.config/natus/config.json` (or platform equivalent). Defaults point at the local hangar (`http://127.0.0.1:8500`), parakeet STT, and the flagship chat model.

## Architecture

Pure Rust. GUI via egui/eframe. No web frontend, no Node/Python runtime.
EOF