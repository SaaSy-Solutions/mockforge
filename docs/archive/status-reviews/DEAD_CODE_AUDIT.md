# Dead Code Annotations Audit

**Date**: 2025-01-27
**Last Updated**: 2025-01-27
**Original Instances**: 44 `#[allow(dead_code)]` annotations across 27 files
**Status**: ✅ **IMPROVED** - Added TODO comments and documentation to all instances

## Summary

All `#[allow(dead_code)]` annotations have been reviewed and improved:
- ✅ Added TODO comments explaining when code should be integrated
- ✅ Added documentation explaining the purpose of future code
- ✅ Categorized by intended use (future features, platform-specific, extensibility)
- ✅ Module-level allows replaced with targeted comments explaining platform-specific code

## Changes Made

1. **Module-level allows** - Replaced with explanatory comments about platform-specific code
2. **Future features** - Added TODO comments with specific integration points
3. **Struct fields** - Documented why fields are reserved for future use
4. **Functions** - Added documentation and TODO comments explaining intended usage

## Categorization

### Platform-Specific Code
- `mockforge-core/src/encryption.rs` - Windows/macOS keychain code (conditional compilation)

### Future Features (with TODO comments)
- Date/time templating (`templating.rs`)
- JavaScript scripting (`request_scripting.rs`)
- Generic mock server handlers (`mock_server.rs`)
- Chaos engineering fine-grained controls (`main.rs`)
- Smart generation features (`smart_mock_generator.rs`)
- HTTP bridge implementations (`http_bridge/mod.rs`)
- Protobuf-JSON conversion (`converters.rs`)
- Schema relationship analysis (`schema_graph.rs`)

### Reserved for Extensibility
- Mock server state fields (config, handlers)
- Endpoint matching utilities

## Next Steps

When implementing features:
1. Remove `#[allow(dead_code)]` annotation
2. Remove or update TODO comment
3. Integrate the code into the feature

## Priority

✅ **Addressed** - All annotations now have clear justification and TODO comments for future integration.
