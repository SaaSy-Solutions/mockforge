#!/bin/bash
# Generate placeholder icons for development
# These are simple geometric icons - replace with proper designs later

set -e

ICON_DIR="$(dirname "$0")/../icons"
mkdir -p "$ICON_DIR"

echo "Generating placeholder icons..."

# Check if ImageMagick is available
if ! command -v convert &> /dev/null; then
    echo "Error: ImageMagick (convert) not found"
    exit 1
fi

# Create a simple 512x512 icon with "MF" text
convert -size 512x512 xc:'#3b82f6' \
  -gravity center \
  -pointsize 200 \
  -fill white \
  -font Arial-Bold \
  -annotate +0+0 'MF' \
  "$ICON_DIR/icon-512.png"

echo "Base icon created: $ICON_DIR/icon-512.png"
echo "Run create-icons.sh to generate all platform icons"
