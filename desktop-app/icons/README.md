# MockForge Desktop App Icons

This directory should contain app icons for all platforms.

## Required Icons

### Windows
- `icon.ico` - Main icon (256x256 recommended, with multiple sizes embedded)

### macOS
- `icon.icns` - Icon set (512x512 base, with @2x variants)

### Linux
- `32x32.png` - 32x32 icon
- `128x128.png` - 128x128 icon
- `128x128@2x.png` - 256x256 icon (high DPI)

### All Platforms
- `icon.png` - 512x512 base icon (used for system tray)

## Icon Creation

### Using ImageMagick (from a 512x512 source)

```bash
# Create Windows ICO (requires imagemagick and icotool)
convert icon-512.png -resize 256x256 icon-256.png
convert icon-512.png -resize 128x128 icon-128.png
convert icon-512.png -resize 64x64 icon-64.png
convert icon-512.png -resize 48x48 icon-48.png
convert icon-512.png -resize 32x32 icon-32.png
convert icon-512.png -resize 16x16 icon-16.png
icotool -c icon-16.png icon-32.png icon-48.png icon-64.png icon-128.png icon-256.png -o icon.ico

# Create macOS ICNS (requires iconutil on macOS)
mkdir icon.iconset
sips -z 16 16 icon-512.png --out icon.iconset/icon_16x16.png
sips -z 32 32 icon-512.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32 icon-512.png --out icon.iconset/icon_32x32.png
sips -z 64 64 icon-512.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128 icon-512.png --out icon.iconset/icon_128x128.png
sips -z 256 256 icon-512.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256 icon-512.png --out icon.iconset/icon_256x256.png
sips -z 512 512 icon-512.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512 icon-512.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon-512.png --out icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset -o icon.icns

# Create Linux PNGs
convert icon-512.png -resize 32x32 32x32.png
convert icon-512.png -resize 128x128 128x128.png
convert icon-512.png -resize 256x256 128x128@2x.png
cp icon-512.png icon.png
```

### Using Online Tools

- **ICO**: https://convertio.co/png-ico/ or https://icoconvert.com/
- **ICNS**: https://cloudconvert.com/png-to-icns (macOS only) or use `iconutil` on macOS
- **PNG**: Any image editor

## Icon Design Guidelines

- **Size**: Start with 1024x1024 source image
- **Format**: PNG with transparency
- **Style**: Modern, recognizable, works at small sizes
- **Colors**: Should work on light and dark backgrounds
- **Details**: Keep simple for small sizes (32x32, 48x48)

## Temporary Icons

Until proper icons are created, you can use placeholder icons:
- Simple geometric shapes
- Text-based icons (MF for MockForge)
- SVG converted to PNG

## Current Status

⚠️ **Icons need to be created** - Placeholder icons are currently referenced in `tauri.conf.json`
