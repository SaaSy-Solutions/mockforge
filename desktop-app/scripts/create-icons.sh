#!/bin/bash
# Script to generate app icons from a source image
# Usage: ./create-icons.sh source-icon.png

set -e

SOURCE_IMAGE="$1"

if [ -z "$SOURCE_IMAGE" ]; then
    echo "Usage: $0 <source-image.png>"
    echo "Source image should be at least 1024x1024 pixels"
    exit 1
fi

if [ ! -f "$SOURCE_IMAGE" ]; then
    echo "Error: Source image not found: $SOURCE_IMAGE"
    exit 1
fi

ICON_DIR="$(dirname "$0")/../icons"
mkdir -p "$ICON_DIR"

echo "Generating icons from $SOURCE_IMAGE..."

# Check if ImageMagick is available
if ! command -v convert &> /dev/null; then
    echo "Error: ImageMagick (convert) not found"
    echo "Install with: sudo apt install imagemagick (Linux) or brew install imagemagick (macOS)"
    exit 1
fi

# Generate Linux PNGs
echo "Creating Linux icons..."
convert "$SOURCE_IMAGE" -resize 32x32 "$ICON_DIR/32x32.png"
convert "$SOURCE_IMAGE" -resize 128x128 "$ICON_DIR/128x128.png"
convert "$SOURCE_IMAGE" -resize 256x256 "$ICON_DIR/128x128@2x.png"
convert "$SOURCE_IMAGE" -resize 512x512 "$ICON_DIR/icon.png"

# Generate Windows ICO (if icotool is available)
if command -v icotool &> /dev/null; then
    echo "Creating Windows ICO..."
    convert "$SOURCE_IMAGE" -resize 16x16 "$ICON_DIR/tmp-16.png"
    convert "$SOURCE_IMAGE" -resize 32x32 "$ICON_DIR/tmp-32.png"
    convert "$SOURCE_IMAGE" -resize 48x48 "$ICON_DIR/tmp-48.png"
    convert "$SOURCE_IMAGE" -resize 64x64 "$ICON_DIR/tmp-64.png"
    convert "$SOURCE_IMAGE" -resize 128x128 "$ICON_DIR/tmp-128.png"
    convert "$SOURCE_IMAGE" -resize 256x256 "$ICON_DIR/tmp-256.png"
    icotool -c "$ICON_DIR/tmp-"*.png -o "$ICON_DIR/icon.ico"
    rm "$ICON_DIR/tmp-"*.png
else
    echo "Warning: icotool not found, skipping Windows ICO generation"
    echo "Install with: sudo apt install icoutils (Linux)"
fi

# Generate macOS ICNS (macOS only)
if [[ "$OSTYPE" == "darwin"* ]]; then
    if command -v iconutil &> /dev/null; then
        echo "Creating macOS ICNS..."
        ICONSET="$ICON_DIR/icon.iconset"
        rm -rf "$ICONSET"
        mkdir -p "$ICONSET"

        sips -z 16 16 "$SOURCE_IMAGE" --out "$ICONSET/icon_16x16.png"
        sips -z 32 32 "$SOURCE_IMAGE" --out "$ICONSET/icon_16x16@2x.png"
        sips -z 32 32 "$SOURCE_IMAGE" --out "$ICONSET/icon_32x32.png"
        sips -z 64 64 "$SOURCE_IMAGE" --out "$ICONSET/icon_32x32@2x.png"
        sips -z 128 128 "$SOURCE_IMAGE" --out "$ICONSET/icon_128x128.png"
        sips -z 256 256 "$SOURCE_IMAGE" --out "$ICONSET/icon_128x128@2x.png"
        sips -z 256 256 "$SOURCE_IMAGE" --out "$ICONSET/icon_256x256.png"
        sips -z 512 512 "$SOURCE_IMAGE" --out "$ICONSET/icon_256x256@2x.png"
        sips -z 512 512 "$SOURCE_IMAGE" --out "$ICONSET/icon_512x512.png"
        sips -z 1024 1024 "$SOURCE_IMAGE" --out "$ICONSET/icon_512x512@2x.png"

        iconutil -c icns "$ICONSET" -o "$ICON_DIR/icon.icns"
        rm -rf "$ICONSET"
    else
        echo "Warning: iconutil not found (should be available on macOS)"
    fi
else
    echo "Skipping macOS ICNS (requires macOS with iconutil)"
fi

echo "Icons generated in $ICON_DIR"
ls -lh "$ICON_DIR"
