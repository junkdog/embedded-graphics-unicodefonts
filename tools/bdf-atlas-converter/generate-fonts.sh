#!/bin/bash

# generate-fonts.sh - Generate font files from BDF fonts using bdf-atlas-converter

set -e

# Change to the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Path to the bdf-atlas-converter binary
BDF_ATLAS_BIN="../../target/release/bdf-atlas-converter"

# Check if binary exists, if not build it
if [ ! -f "$BDF_ATLAS_BIN" ]; then
    echo "Binary not found at $BDF_ATLAS_BIN, building with cargo build --release..."
    cargo build --release
fi

# Clone misc-misc repository if it doesn't exist, or update if it does
if [ ! -d "misc-misc" ]; then
    echo "Cloning misc-misc repository..."
    git clone https://gitlab.freedesktop.org/xorg/font/misc-misc.git
else
    cd misc-misc && git pull && cd ..
fi

# Find all BDF files and filter out localized ones and special fonts
for font in misc-misc/*.bdf; do
    if [[ ! "$font" =~ (ja|ko|nil2|k14) ]]; then
        echo "Processing $font..."
        $BDF_ATLAS_BIN "$font" --output ../../src
    fi
done

echo "Font generation complete!"

