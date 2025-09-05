#!/bin/bash

# generate-fonts.sh - Generate font files from BDF fonts using bdf-atlas-converter

set -e

# Change to the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Path to the bdf-atlas-converter binary
ATLAS_CONVERTER="../../target/release/bdf-atlas-converter"

# <describe>
BUILD_OPTIMIZED_FONTS=(
    "misc-misc/6x10.bdf"
    "misc-misc/7x12.bdf"
)

# Check if binary exists, if not build it
if [ ! -f "$ATLAS_CONVERTER" ]; then
    echo "Binary not found at $ATLAS_CONVERTER, building with cargo build --release..."
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

### Build atlas-compatible fonts ###
# build fonts with a subset of glyphs, suitable for most ratatui use cases
if [ "$BUILD_OPTIMIZED_FONTS" -ne 0 ]; then
    for font in misc-misc/*.bdf; do
        if [[ ! "$font" =~ (ja|ko|nil2|k14) ]]; then
            $ATLAS_CONVERTER "$font" \
                --save-png \
                --suffix="_optimized" \
                --gap-threshold=2 \
                --output ../../src
        fi
    done
fi

# rebuild existing fonts for faster lookups. retains all glyphs.
for font in misc-misc/*.bdf; do
    if [[ ! "$font" =~ (k14) ]]; then # has almost no glyphs
        echo "Processing font: $font"
        $ATLAS_CONVERTER "$font" \
            --save-png \
            --range 0x0020..0xffff \
            --gap-threshold=0 \
            --output ../../src
    fi
done

# Bulk move all .data files to raw/ directory
echo "Moving .data files to raw/ directory..."
mv ../../src/*.data ../../src/raw/

# Bulk move all .png files to assets/ directory
echo "Moving .png files to assets/ directory..."
mv ../../src/*.png ../../assets/

# Update all .rs files to use raw/ prefix for includes that don't already have it
echo "Updating .rs files to use raw/ prefix..."
sed -i '/include_bytes!("raw\/m/!s/include_bytes!("m\([^"]*\)\.data")/include_bytes!("raw\/m\1.data")/g' ../../src/mono_*.rs

echo "Font generation complete!"

