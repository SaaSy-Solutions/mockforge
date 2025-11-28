# Playwright browser dependencies on Arch Linux

This guide lists the correct Arch package names for Playwright browser runtime dependencies and adds Arch-specific notes for mirrors and verification. Do not use Debian/Ubuntu commands on Arch.

## 1) Install core dependencies (Arch package names)

```bash
sudo pacman -S --needed \
  nss nspr alsa-lib at-spi2-core atk cups dbus libdrm \
  libxkbcommon libxcomposite libxdamage libxfixes libxrandr \
  mesa libxss gtk3 gtk4 gdk-pixbuf2 pango cairo wayland \
  libxrender libxtst libxshmfence
```

## 2) WebKit-specific dependencies (Arch package names)

```bash
sudo pacman -S --needed webkit2gtk libsoup libepoxy libwpe wpebackend-fdo
```

## 3) Recommended fonts/codecs for consistent headless rendering

```bash
sudo pacman -S --needed fontconfig freetype2 ttf-dejavu noto-fonts libjpeg-turbo libpng
```

## 4) Playwright browser install

```bash
cd crates/mockforge-ui/ui
npx playwright install
```

Notes for Arch Linux:
- Do NOT run `npx playwright install-deps` on Arch; it targets Debian/Ubuntu and will fail (tries `apt-get`).
- You may see Playwright warnings about using Ubuntu fallback builds on Arch; these warnings are expected and generally harmless.

## 5) If you hit 523/failed mirror errors (Arch mirrors)

Switch to healthy HTTPS mirrors, then refresh package databases and retry:

```bash
sudo pacman -S --needed reflector
sudo reflector --latest 20 --protocol https --sort rate --save /etc/pacman.d/mirrorlist
sudo pacman -Syyu
```

If you still hit a specific bad mirror, temporarily comment it out in `/etc/pacman.d/mirrorlist`, then re-run the installs above.

## 6) Verify installation

```bash
# Show Playwright version
npx playwright --version

# Dry run: list what would be installed (should be no changes after success)
npx playwright install --dry-run

# Smoke test Chromium can launch headless and navigate
node -e "const { chromium } = require('playwright');(async()=>{const b=await chromium.launch();const p=await (await b.newContext()).newPage();await p.goto('https://example.com');console.log(await p.title());await b.close();})();"
```

Troubleshooting hints:
- If WebKit reports being a frozen Ubuntu 20.04 build, that’s Playwright’s fallback for Arch; ensure the WebKit packages above are installed. Chromium/Firefox typically work out of the box with the core set.
- If the `cd crates/mockforge-ui/ui` step fails, run commands from the repository root and ensure the UI path exists.


