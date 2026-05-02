# API Documentation TODO

This document tracks missing documentation for public API crates that must be completed before 1.0 release.

## mockforge-plugin-core

**Status**: ✅ COMPLETE - All documentation added

**Priority**: HIGH - This is the primary public API for plugin developers

### Completed:

1. **`src/manifest/schema.rs`**
   - ✅ Added documentation for all PropertyType enum variants

2. **`src/types.rs`**
   - ✅ Added documentation for all struct fields
   - ✅ Added documentation for all associated functions (constructors and builders)
   - ✅ Added documentation for all public methods
   - ✅ Added documentation for all PluginError enum variants

3. **`src/lib.rs`**
   - ✅ Enhanced module-level documentation with comprehensive examples
   - ✅ Added Quick Start guide
   - ✅ Documented key types and features

4. **Module-level docs**
   - ✅ All modules have comprehensive documentation
   - ✅ auth.rs, datasource.rs, template.rs, response.rs, runtime.rs, error.rs

### Results:
- 0 missing documentation errors
- Successfully compiles with `missing_docs = "deny"`
- Documentation generated at `target/doc/mockforge_plugin_core/index.html`
- Ready for 1.0 release

## mockforge-plugin-sdk

**Status**: ✅ Should already have `missing_docs = "deny"` enforced
- Verify no missing docs before 1.0

## Other Public API Crates

The following crates should have their documentation reviewed before 1.0:

- [ ] `mockforge-core` - Currently uses `missing_docs = "warn"`
- [ ] `mockforge-http` - No missing_docs enforcement
- [ ] `mockforge-ws` - No missing_docs enforcement
- [ ] `mockforge-grpc` - No missing_docs enforcement
- [ ] `mockforge-graphql` - No missing_docs enforcement
- [ ] `mockforge-data` - No missing_docs enforcement
- [ ] `mockforge-plugin-loader` - No missing_docs enforcement

### Recommendation for 1.0:

Consider enabling `missing_docs = "warn"` at workspace level for all public crates, and `missing_docs = "deny"` for:
- `mockforge-plugin-core`
- `mockforge-plugin-sdk`
- `mockforge-core` (as the foundational crate)

This ensures high-quality documentation for the most critical public APIs while allowing flexibility for protocol-specific crates.
