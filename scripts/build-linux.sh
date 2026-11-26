#!/bin/sh
# Double-click in your file manager (or run from a terminal) to build natus on Linux.
cd "$(dirname "$0")/.." || exit 1

echo
echo "==> Building natus (release)..."
echo
cargo build --release
status=$?

echo
if [ "$status" -eq 0 ]; then
  echo "BUILD OK: $(pwd)/target/release/natus"
else
  echo "BUILD FAILED."
fi
echo
echo "Press Return to close..."
read -r _
exit "$status"
