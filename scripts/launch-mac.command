#!/bin/bash
# Double-click in Finder to launch natus on macOS (builds first if needed).
cd "$(dirname "$0")/.." || exit 1
BIN="$(pwd)/target/release/natus"

if [ ! -x "$BIN" ]; then
  echo
  echo "==> No release build found; building first..."
  echo
  if ! cargo build --release; then
    echo
    echo "BUILD FAILED."
    echo "Press Return to close..."
    read -r _
    exit 1
  fi
fi

echo
echo "==> Launching natus..."
echo
"$BIN"
status=$?
if [ "$status" -ne 0 ]; then
  echo
  echo "natus exited with code $status."
  echo "Press Return to close..."
  read -r _
fi
exit "$status"
