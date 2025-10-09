# Plugin Update Implementation

## Overview

This document describes the implementation of the plugin update functionality for MockForge's plugin system.

## Problem Statement

The TODO at `crates/mockforge-plugin-loader/src/installer.rs:233` required:
- Track the original source of each installed plugin
- Fetch the latest version from that source
- Implement update logic for individual and bulk plugin updates

## Solution

### 1. Plugin Metadata Storage System

Created `crates/mockforge-plugin-loader/src/metadata.rs` with the following components:

#### `PluginMetadata`
Tracks installation information for each plugin:
- `plugin_id`: Plugin identifier
- `source`: Original installation source (URL, Git, Local, Registry)
- `installed_at`: Unix timestamp of installation
- `updated_at`: Optional Unix timestamp of last update
- `version`: Currently installed version

#### `MetadataStore`
Manages plugin metadata persistence:
- Stores metadata as JSON files in `~/.mockforge/plugin-metadata/`
- Provides async methods for CRUD operations
- Maintains in-memory cache for performance
- Supports serialization/deserialization of all `PluginSource` types

### 2. Integration with Plugin Installer

Modified `crates/mockforge-plugin-loader/src/installer.rs`:

#### Added to `PluginInstaller`:
- `metadata_store`: Arc-wrapped RwLock for thread-safe metadata access
- `init()`: Loads existing metadata on startup

#### Updated Methods:
- `install_from_source()`: Saves metadata after successful installation
- `uninstall()`: Removes metadata when plugin is uninstalled

### 3. Update Implementation

#### `update(&self, plugin_id: &PluginId)`
Updates a single plugin to its latest version:
1. Retrieves plugin metadata to get original source
2. Unloads the current plugin version
3. Reinstalls from the original source with `force: true`
4. Verifies plugin ID matches after update
5. Updates metadata with new version and timestamp
6. Logs progress and errors

#### `update_all(&self)`
Updates all installed plugins:
1. Gets list of all plugins with metadata
2. Iterates through each plugin calling `update()`
3. Tracks successful and failed updates
4. Returns list of successfully updated plugin IDs
5. Logs comprehensive update statistics

### 4. Helper Methods

Added utility methods for querying metadata:
- `get_plugin_metadata(&self, plugin_id)`: Get metadata for a specific plugin
- `list_plugins_with_metadata(&self)`: List all plugins with their metadata

## Usage Example

```rust
use mockforge_plugin_loader::{PluginInstaller, PluginLoaderConfig};

// Create installer
let installer = PluginInstaller::new(PluginLoaderConfig::default())?;
installer.init().await?;

// Install a plugin
installer.install("https://example.com/my-plugin.zip", Default::default()).await?;

// Update a specific plugin
let plugin_id = PluginId::new("my-plugin");
installer.update(&plugin_id).await?;

// Update all plugins
let updated = installer.update_all().await?;
println!("Updated {} plugins", updated.len());

// Query plugin metadata
if let Some(metadata) = installer.get_plugin_metadata(&plugin_id).await {
    println!("Plugin source: {}", metadata.source);
    println!("Installed version: {}", metadata.version);
}
```

## File Changes

1. **Created**: `crates/mockforge-plugin-loader/src/metadata.rs`
   - New module for metadata tracking
   - ~350 lines including tests

2. **Modified**: `crates/mockforge-plugin-loader/src/lib.rs`
   - Added metadata module export

3. **Modified**: `crates/mockforge-plugin-loader/src/installer.rs`
   - Added MetadataStore field
   - Implemented `init()`, `update()`, and `update_all()` methods
   - Updated `install_from_source()` and `uninstall()` methods
   - Added helper methods for metadata queries

## Benefits

1. **Automatic Update Support**: Users can now update plugins without manually tracking sources
2. **Bulk Updates**: Single command to update all installed plugins
3. **Version Tracking**: Know when plugins were installed and updated
4. **Source Transparency**: See where each plugin came from
5. **Safe Updates**: Validates plugin ID remains consistent after update
6. **Comprehensive Logging**: Detailed progress and error reporting

## Testing

- All existing tests pass
- Includes unit tests for metadata serialization/deserialization
- Tests for metadata store CRUD operations
- Tests verify PluginSource serialization for all variants

## Future Enhancements

Potential improvements for the future:
1. Version comparison to skip updates if already on latest version
2. Rollback functionality to revert to previous version
3. Update notifications/checking for new versions
4. Differential updates (only download changed files)
5. Plugin dependency update coordination
