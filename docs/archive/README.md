# Archive Directory

This directory contains archived documentation files that were moved from the project root to keep it clean and organized.

## Directory Structure

- `status-reviews/` - Status, verification, review, and completion documents
- `implementation-reviews/` - Implementation review documents (currently empty, files moved to status-reviews)

## What Was Archived

### Status/Verification Documents

These files document the status, verification, and completion of various features and implementations:

- Status documents (`*_STATUS.md`)
- Verification documents (`*_VERIFICATION.md`)
- Review documents (`*_REVIEW.md`)
- Completion documents (`*_COMPLETE.md`)
- Summary documents (`*_SUMMARY.md`)
- Coverage documents (`*_COVERAGE.md`)
- Code review documents (`CODE_REVIEW*.md`)

### Why These Files Were Archived

These files were moved from the project root to:

1. **Reduce Clutter**: The project root had accumulated 80+ status/review documents, making it difficult to find essential files
2. **Maintain History**: Files are preserved for historical reference and context
3. **Improve Organization**: Related documents are now grouped together
4. **Keep Root Clean**: Only essential project files remain in the root directory

### Important Notes

- **These files are still tracked in Git** - They are not deleted, just moved
- **Historical Context Preserved** - All implementation history is maintained
- **No Functionality Lost** - These are documentation files, not code
- **Easy to Find** - Files are organized by type in subdirectories

## Finding Archived Files

### By Feature

If you're looking for information about a specific feature:

- **VBR**: `VBR_IMPLEMENTATION_SUMMARY.md`
- **Temporal Simulation**: `TEMPORAL_SIMULATION_REVIEW.md`, `TEMPORAL_SIMULATION_VERIFICATION.md`
- **MockAI**: `MOCKAI_IMPLEMENTATION_REVIEW.md`, `MOCKAI_FEATURES_IMPLEMENTATION_REVIEW.md`
- **Chaos Lab**: `CHAOS_LAB_IMPLEMENTATION_STATUS.md`, `CHAOS_LAB_VERIFICATION.md`
- **Collaboration**: `COLLABORATION_COMPLETE.md`, `COLLABORATION_CLOUD_COVERAGE.md`
- **ForgeConnect**: `FORGECONNECT_COMPLETE.md`, `FORGECONNECT_CODE_REVIEW.md`
- **Voice + LLM**: `VOICE_LLM_IMPLEMENTATION_REVIEW.md`
- **Smart Personas**: `SMART_PERSONAS_CODE_REVIEW.md`
- **Scenario State Machines**: `SCENARIO_STATE_MACHINES_IMPLEMENTATION_REVIEW.md`

### By Type

- **Implementation Reviews**: Files ending in `_IMPLEMENTATION_REVIEW.md` or `_IMPLEMENTATION_SUMMARY.md`
- **Code Reviews**: Files starting with `CODE_REVIEW` or `*_CODE_REVIEW.md`
- **Verification**: Files ending in `_VERIFICATION.md`
- **Status**: Files ending in `_STATUS.md`
- **Completion**: Files ending in `_COMPLETE.md`

## Current Documentation

For current, up-to-date documentation on features:

- **User Documentation**: See `book/src/user-guide/` directory
- **Feature Documentation**: See `book/src/user-guide/advanced-features.md` and related files
- **API Documentation**: See `book/src/api/` directory
- **Configuration**: See `book/src/configuration/` directory

## Restoration

If you need to restore a file to the root directory:

```bash
# Restore a specific file
mv docs/archive/status-reviews/FILENAME.md .

# Or restore all files (not recommended)
mv docs/archive/status-reviews/*.md .
```

## Last Updated

2025-01-27 - Initial archive organization
