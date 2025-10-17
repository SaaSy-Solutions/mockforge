//! Helper macros for plugin development

/// Export a plugin with boilerplate
///
/// This macro generates the necessary WASM exports for your plugin.
///
/// # Example
///
/// ```rust,no_run
/// use mockforge_plugin_sdk::{export_plugin, prelude::*, Result as PluginCoreResult};
/// use std::collections::HashMap;
///
/// #[derive(Debug, Default)]
/// pub struct MyPlugin;
///
/// #[async_trait]
/// impl AuthPlugin for MyPlugin {
///     fn capabilities(&self) -> PluginCapabilities {
///         PluginCapabilities::default()
///     }
///
///     async fn initialize(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
///         Ok(())
///     }
///
///     async fn authenticate(
///         &self,
///         _context: &PluginContext,
///         _request: &AuthRequest,
///         _config: &AuthPluginConfig,
///     ) -> PluginCoreResult<PluginResult<AuthResponse>> {
///         let identity = UserIdentity::new("user123");
///         let response = AuthResponse::success(identity, HashMap::new());
///         Ok(PluginResult::success(response, 0))
///     }
///
///     fn validate_config(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
///         Ok(())
///     }
///
///     fn supported_schemes(&self) -> Vec<String> {
///         vec!["basic".to_string()]
///     }
///
///     async fn cleanup(&self) -> PluginCoreResult<()> {
///         Ok(())
///     }
/// }
///
/// export_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        /// Create plugin instance
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut std::ffi::c_void {
            let plugin = Box::new(<$plugin_type>::default());
            Box::into_raw(plugin) as *mut std::ffi::c_void
        }

        /// Destroy plugin instance
        #[no_mangle]
        pub extern "C" fn destroy_plugin(ptr: *mut std::ffi::c_void) {
            if !ptr.is_null() {
                unsafe {
                    let _ = Box::from_raw(ptr as *mut $plugin_type);
                }
            }
        }
    };
}

/// Generate a plugin configuration struct
///
/// # Example
///
/// ```rust,no_run
/// use mockforge_plugin_sdk::plugin_config;
///
/// plugin_config! {
///     id = "my-plugin",
///     version = "1.0.0",
///     name = "My Plugin",
///     description = "A custom plugin",
///     capabilities = ["network:http"],
///     author = {
///         name = "Your Name",
///         email = "your.email@example.com",
///     },
/// }
/// ```
#[macro_export]
macro_rules! plugin_config {
    (
        id = $id:expr,
        version = $version:expr,
        name = $name:expr,
        description = $desc:expr,
        capabilities = [$($capability:expr),*],
        author = {
            name = $author_name:expr,
            email = $author_email:expr $(,)?
        } $(,)?
    ) => {
        /// Plugin configuration
        pub fn plugin_config() -> mockforge_plugin_core::PluginManifest {
            use mockforge_plugin_core::*;

            let info = PluginInfo::new(
                PluginId::new($id),
                PluginVersion::parse($version).expect("Invalid version"),
                $name,
                $desc,
                PluginAuthor::with_email($author_name, $author_email),
            );

            let mut manifest = PluginManifest::new(info);
            $(
                manifest.capabilities.push($capability.to_string());
            )*
            manifest
        }
    };
}

/// Quick test macro for plugin functions
///
/// # Example
///
/// ```rust,no_run
/// # use mockforge_plugin_sdk::prelude::*;
/// # async fn test_auth() {
/// use axum::http::{HeaderMap, Method, Uri};
/// use mockforge_plugin_sdk::{
///     mock_context, plugin_test, prelude::*, Result as PluginCoreResult,
/// };
/// use std::collections::HashMap;
///
/// #[derive(Default)]
/// struct TestPlugin;
///
/// #[async_trait]
/// impl AuthPlugin for TestPlugin {
///     fn capabilities(&self) -> PluginCapabilities {
///         PluginCapabilities::default()
///     }
///
///     async fn initialize(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
///         Ok(())
///     }
///
///     async fn authenticate(
///         &self,
///         _context: &PluginContext,
///         _request: &AuthRequest,
///         _config: &AuthPluginConfig,
///     ) -> PluginCoreResult<PluginResult<AuthResponse>> {
///         let identity = UserIdentity::new("user");
///         let response = AuthResponse::success(identity, HashMap::new());
///         Ok(PluginResult::success(response, 0))
///     }
///
///     fn validate_config(&self, _config: &AuthPluginConfig) -> PluginCoreResult<()> {
///         Ok(())
///     }
///
///     fn supported_schemes(&self) -> Vec<String> {
///         vec!["basic".to_string()]
///     }
///
///     async fn cleanup(&self) -> PluginCoreResult<()> {
///         Ok(())
///     }
/// }
///
/// plugin_test! {
///     test_name: authenticate_valid_user,
///     plugin: TestPlugin,
///     context: mock_context! {
///         plugin_id: "test-plugin",
///         request_id: "req-123",
///     },
///     request: AuthRequest::from_axum(
///         Method::GET,
///         Uri::from_static("/login"),
///         HeaderMap::new(),
///         None
///     ),
///     config: AuthPluginConfig::default(),
///     assert: |result| {
///         assert!(result.unwrap().is_success());
///     }
/// }
/// # }
/// ```
#[macro_export]
macro_rules! plugin_test {
    (
        test_name: $name:ident,
        plugin: $plugin:ty,
        context: $context:expr,
        request: $request:expr,
        config: $config:expr,
        assert: $assert:expr
    ) => {
        #[tokio::test]
        async fn $name() {
            let plugin = <$plugin>::default();
            let context = $context;
            let request = $request;
            let config = $config;
            let result = plugin.authenticate(&context, &request, &config).await;
            ($assert)(result);
        }
    };
}

/// Create a mock plugin context for testing
///
/// # Example
///
/// ```rust,no_run
/// # use mockforge_plugin_sdk::{mock_context, prelude::*};
/// let context = mock_context! {
///     plugin_id: "test-plugin",
///     request_id: "req-123",
/// };
/// ```
#[macro_export]
macro_rules! mock_context {
    (
        plugin_id: $plugin_id:expr,
        request_id: $request_id:expr $(,)?
    ) => {{
        use mockforge_plugin_core::{PluginContext, PluginId, PluginVersion};
        let mut context =
            PluginContext::new(PluginId::new($plugin_id), PluginVersion::new(0, 1, 0));
        context.request_id = $request_id.to_string();
        context
    }};

    () => {{
        use mockforge_plugin_core::{PluginContext, PluginId, PluginVersion};
        PluginContext::new(PluginId::new("mockforge-plugin"), PluginVersion::new(0, 1, 0))
    }};
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_macro_compilation() {
        // Just verify macros compile
        // Test passes if compilation succeeds
    }
}
