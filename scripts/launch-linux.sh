#!/bin/sh
# Double-click in your file manager (or run from a terminal) to launch natus on Linux.
# Builds first if no release binary exists yet.
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
