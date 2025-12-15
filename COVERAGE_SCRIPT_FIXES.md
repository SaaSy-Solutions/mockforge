# Coverage Baseline Script Fixes

## Issues Fixed

### 1. `--lcov` and `--json` Cannot Be Used Together
**Problem**: The script was trying to use both `--lcov` and `--json` flags together, which cargo-llvm-cov doesn't allow.

**Fix**: Changed to use LCOV format first (easier to parse), then generate JSON separately if needed.

### 2. Error Detection Too Aggressive
**Problem**: Script was detecting compilation warnings as errors.

**Fix**: Improved error detection to only flag actual compilation errors (error[E...], "could not compile", etc.), not warnings.

### 3. JSON Format Parsing
**Problem**: cargo-llvm-cov JSON format is complex (file-level data), not summary-level.

**Fix**: Switched to parsing LCOV format which is simpler and more reliable:
- `LF:` = Lines Found (total)
- `LH:` = Lines Hit (covered)

## Current Implementation

The script now:
1. Runs `cargo llvm-cov --lcov` to generate LCOV report
2. Parses LCOV format for coverage statistics
3. Generates JSON separately for detailed analysis
4. Only flags actual compilation errors, not warnings

## Testing

To test the script:
```bash
# Test with a single crate
rm -rf coverage/crates/mockforge-bench
./scripts/coverage-baseline.sh --format text 2>&1 | grep mockforge-bench

# Full run (takes 30-60 minutes)
./scripts/coverage-baseline.sh
```

## Status

✅ Fixed: `--lcov` and `--json` conflict
✅ Fixed: Error detection logic
✅ Fixed: Coverage calculation from LCOV format

The script should now work correctly for all crates that compile successfully.

