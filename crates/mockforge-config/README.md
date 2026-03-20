# mockforge-config

Configuration types for [MockForge](https://mockforge.dev).

This crate contains pure configuration data types (structs and enums) used across the MockForge workspace. It is a leaf crate with no internal MockForge dependencies.

## Usage

```rust
use mockforge_config::{HttpConfig, ServerConfig, AdminConfig};
```

Types that require I/O, validation logic, or depend on core-specific types remain in `mockforge-core`, which re-exports everything from this crate for backward compatibility.
