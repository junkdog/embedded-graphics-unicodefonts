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

# Create raw directory if it doesn't exist
mkdir -p ../../src/raw

# Find all BDF files and filter out localized ones and special fonts
for font in misc-misc/*.bdf; do
    if [[ ! "$font" =~ (ja|ko|nil2|k14) ]]; then
        echo "Processing $font..."
        $BDF_ATLAS_BIN "$font" --output ../../src
    fi
done

# Bulk move all .data files to raw/ directory
echo "Moving .data files to raw/ directory..."
mv ../../src/*.data ../../src/raw/ 2>/dev/null || true

# Update all .rs files to use raw/ prefix for includes that don't already have it
echo "Updating .rs files to use raw/ prefix..."
sed -i '/include_bytes!("raw\/m/!s/include_bytes!("m\([^"]*\)\.data")/include_bytes!("raw\/m\1.data")/g' ../../src/mono_*.rs

echo "Font generation complete!"

