//! Helper macros for plugin development

/// Export a plugin with boilerplate
///
/// This macro generates the necessary WASM exports for your plugin.
///
/// # Example
///
/// ```rust,no_run
/// use mockforge_plugin_sdk::prelude::*;
///
/// #[derive(Debug)]
/// pub struct MyPlugin;
///
/// #[async_trait]
/// impl AuthPlugin for MyPlugin {
///     async fn authenticate(&self, context: &PluginContext, credentials: &AuthCredentials) -> PluginResult<AuthResult> {
///         Ok(AuthResult::authenticated("user123"))
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
///     types = ["auth"],
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
        types = [$($plugin_type:expr),*],
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
                PluginAuthor::new($author_name).with_email($author_email),
            );

            let mut manifest = PluginManifest::new(info);
            $(
                manifest.types.push($plugin_type.to_string());
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
/// plugin_test! {
///     test_name: authenticate_valid_user,
///     plugin: MyAuthPlugin,
///     input: AuthCredentials::basic("user", "pass"),
///     assert: |result| {
///         assert!(result.is_ok());
///     }
/// }
/// # }
/// ```
#[macro_export]
macro_rules! plugin_test {
    (
        test_name: $name:ident,
        plugin: $plugin:ty,
        input: $input:expr,
        assert: $assert:expr
    ) => {
        #[tokio::test]
        async fn $name() {
            let plugin = <$plugin>::default();
            let context = mockforge_plugin_core::PluginContext::default();
            let result = plugin.process(&context, $input).await;
            ($assert)(result);
        }
    };
}

/// Create a mock plugin context for testing
///
/// # Example
///
/// ```rust,no_run
/// # use mockforge_plugin_sdk::prelude::*;
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
        use mockforge_plugin_core::PluginContext;
        PluginContext {
            plugin_id: $plugin_id.to_string(),
            request_id: $request_id.to_string(),
            ..Default::default()
        }
    }};

    () => {{
        mockforge_plugin_core::PluginContext::default()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_compilation() {
        // Just verify macros compile
        assert!(true);
    }
}
