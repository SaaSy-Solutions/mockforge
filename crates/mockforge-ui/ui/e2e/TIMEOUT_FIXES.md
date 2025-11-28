# Timeout Fixes Applied

## Summary
All timeouts have been significantly reduced to prevent tests from hanging. Tests now fail faster with clearer error messages.

## Changes Made

### 1. **waitForAppLoad** (`helpers.ts`)
- Reduced `MAX_WAIT_TIME` from 20s to 12s
- Reduced `domcontentloaded` timeout from 10s to 5s
- Reduced React hydration wait from 1000ms to 500ms
- Reduced login timeout from 15s to 8s
- Reduced selector wait timeouts from 3000ms to 2000ms
- Reduced final wait from 500ms to 300ms

### 2. **loginAsAdmin** (`helpers.ts`)
- Reduced Demo Admin button timeout from 2000ms to 1500ms
- Added explicit navigation wait with 8000ms timeout
- Reduced form login timeout from 3000ms to 2000ms
- Reduced wait times after login from 1000ms to 500ms

### 3. **navigateToTab** (`helpers.ts`)
- Reduced React render wait from 3000ms to 1000ms

### 4. **waitForDashboardLoad** (`helpers.ts`)
- Reduced selector wait from 2000ms to 1500ms
- Added 3000ms timeout for API response
- Changed fallback from `networkidle` to `domcontentloaded` with 2000ms timeout
- Added body selector as fallback

### 5. **setupTest** (`test-helpers.ts`)
- Reduced navigation timeout to max 10s
- Reduced APP_LOAD_TIMEOUT to 10s (from 25s)
- Better error handling for page closure

### 6. **dashboard.spec.ts**
- Reduced dashboard load timeout from 15s to 8s
- Added fallback wait for page stabilization

## Result
Tests should now:
- Timeout much faster (10-12s instead of 60s)
- Provide clearer error messages
- Still handle slow servers gracefully
- Continue even if some steps fail

## Testing
Run a single test to verify:
```bash
cd crates/mockforge-ui/ui
npm run test:e2e -- e2e/dashboard.spec.ts --timeout=30000
```
