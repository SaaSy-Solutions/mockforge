# MockForge v0.1.4 Publishing Guide

## Summary

This release includes new features and crates ready for publishing to crates.io.

### Version
- **Current version**: 0.1.4
- **Previous version**: 0.1.3

### New Crates (First Time Publishing)
1. **mockforge-sdk** - Developer SDK for embedding MockForge in tests and applications
2. **mockforge-analytics** - Traffic analytics and metrics dashboard
3. **mockforge-collab** - Cloud collaboration features

## Prerequisites

1. **Crates.io Account**: You need a crates.io account
2. **API Token**: Get your token from https://crates.io/me

## Publishing Steps

### Step 1: Set Your Crates.io Token

```bash
export CRATES_IO_TOKEN='your_token_here'
```

### Step 2: Dry Run (Recommended)

```bash
./scripts/publish-crates.sh --dry-run
```

### Step 3: Publish to Crates.io

```bash
./scripts/publish-crates.sh
```

The script will:
- Publish crates in correct dependency order
- Skip crates already published
- Wait 30 seconds between publishes
- Handle all 25 workspace crates

---

**Ready to publish?** Run `./scripts/publish-crates.sh --dry-run` to get started!
