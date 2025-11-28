# PWA Functionality Test Guide

## Testing Checklist

### 1. Build Verification
```bash
cd crates/mockforge-ui/ui
pnpm build
```

**Expected Results:**
- `dist/manifest.json` should exist
- `dist/sw.js` should exist
- `dist/index.html` should include manifest link
- No build errors

### 2. Service Worker Registration
1. Start the MockForge server with Admin UI enabled
2. Open browser DevTools → Application → Service Workers
3. Check that service worker is registered
4. Verify cache storage shows `mockforge-admin-v1` and `mockforge-runtime-v1`

**Expected Results:**
- Service worker status: "activated and running"
- Cache entries present for static assets

### 3. Offline Functionality
1. Open Admin UI in browser
2. Go to DevTools → Network → Check "Offline"
3. Refresh the page
4. Navigate between pages

**Expected Results:**
- Page loads from cache when offline
- Static assets (CSS, JS) load from cache
- API calls show offline indicator or cached responses

### 4. Install Prompt
1. Open Admin UI in Chrome/Edge
2. Look for install icon in address bar
3. Click install icon
4. Verify app installs as standalone app

**Expected Results:**
- Install prompt appears (may take a few visits)
- App installs successfully
- App opens in standalone window (no browser UI)

### 5. Update Detection
1. Make a change to the service worker (increment version)
2. Rebuild and restart server
3. Reload the page
4. Check for update notification

**Expected Results:**
- Update notification appears
- New service worker activates after reload

### 6. Manifest Validation
1. Open `http://localhost:9080/manifest.json`
2. Verify JSON is valid
3. Check all required fields are present

**Expected Results:**
- Valid JSON response
- All icon sizes present
- Theme colors configured
- Shortcuts defined

## Browser Compatibility

- ✅ Chrome/Edge (Chromium) - Full support
- ✅ Firefox - Full support
- ✅ Safari - Full support (iOS 11.3+)
- ⚠️ Safari (Desktop) - Limited support

## Known Limitations

1. Service worker only registers in production builds (`import.meta.env.PROD`)
2. API caching is conservative (network-first strategy)
3. Some browsers may require HTTPS for service workers (localhost is exception)

## Troubleshooting

### Service Worker Not Registering
- Check browser console for errors
- Verify `sw.js` is accessible at `/sw.js`
- Check that build includes service worker file
- Ensure running in production mode

### Offline Not Working
- Clear browser cache
- Unregister old service worker
- Check service worker scope matches app URL
- Verify cache names match in service worker

### Install Prompt Not Appearing
- Must visit site multiple times (browser heuristic)
- Must have valid manifest.json
- Must be served over HTTPS (or localhost)
- Browser must support PWA install
