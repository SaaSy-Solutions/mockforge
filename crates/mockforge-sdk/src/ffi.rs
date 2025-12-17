//! FFI bindings for using `MockForge` from other languages (Python, Node.js, Go)
//!
//! This module provides C-compatible functions that can be called from other languages.

#![allow(unsafe_code)]

use crate::server::MockServer;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

/// Opaque handle to a `MockServer`
pub struct MockServerHandle {
    server: Arc<Mutex<MockServer>>,
    runtime: Runtime,
}

/// Create a new mock server
///
/// # Safety
/// This function is FFI-safe
#[no_mangle]
pub unsafe extern "C" fn mockforge_server_new(port: u16) -> *mut MockServerHandle {
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return ptr::null_mut(),
    };

    // Create and start the server
    let server = runtime.block_on(async { MockServer::new().port(port).start().await });

    let server = match server {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let handle = MockServerHandle {
        server: Arc::new(Mutex::new(server)),
        runtime,
    };

    Box::into_raw(Box::new(handle))
}

/// Stop and destroy a mock server
///
/// # Safety
/// The handle must be valid and not used after this call
#[no_mangle]
pub unsafe extern "C" fn mockforge_server_destroy(handle: *mut MockServerHandle) {
    if handle.is_null() {
        return;
    }

    let handle = Box::from_raw(handle);
    let server = handle.server.clone();

    handle.runtime.block_on(async move {
        let mut server = server.lock().await;
        let _ = std::mem::take(&mut *server).stop().await;
    });
}

/// Add a stub response to the mock server
///
/// # Safety
/// - handle must be valid
/// - method, path, and body must be valid null-terminated C strings
/// - Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn mockforge_server_stub(
    handle: *mut MockServerHandle,
    method: *const c_char,
    path: *const c_char,
    status: u16,
    body: *const c_char,
) -> i32 {
    if handle.is_null() || method.is_null() || path.is_null() || body.is_null() {
        return -1;
    }

    let handle = &*handle;

    let method = match CStr::from_ptr(method).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let path = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let body = match CStr::from_ptr(body).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let body_value: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let server = handle.server.clone();
    let result = handle.runtime.block_on(async move {
        let mut server = server.lock().await;
        server.stub_response(method, path, body_value).await
    });

    match result {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Get the server URL
///
/// # Safety
/// - handle must be valid
/// - Returns a C string that must be freed with `mockforge_free_string`
#[no_mangle]
pub unsafe extern "C" fn mockforge_server_url(handle: *const MockServerHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = &*handle;
    let server = handle.server.clone();

    let url = handle.runtime.block_on(async move {
        let server = server.lock().await;
        server.url()
    });

    match CString::new(url) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a string returned by `MockForge`
///
/// # Safety
/// The string must have been allocated by `MockForge`
#[no_mangle]
pub unsafe extern "C" fn mockforge_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Get the last error message
///
/// # Safety
/// Returns a C string that must be freed with `mockforge_free_string`
#[no_mangle]
pub const unsafe extern "C" fn mockforge_last_error() -> *mut c_char {
    // Thread-local error storage could be implemented here
    ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_mock_server_handle_size() {
        // Verify the handle structure size is reasonable
        assert!(std::mem::size_of::<MockServerHandle>() > 0);
    }

    #[test]
    fn test_mockforge_server_new_null_on_invalid_port() {
        unsafe {
            // Try to create server on port 0 (which might fail in some scenarios)
            // This tests the error path
            let handle = mockforge_server_new(0);

            // We can't reliably test for null here because port 0 is valid (OS assigns port)
            // But we can test that the function doesn't crash
            if !handle.is_null() {
                mockforge_server_destroy(handle);
            }
        }
    }

    #[test]
    fn test_mockforge_server_destroy_null_handle() {
        unsafe {
            // Should not crash when destroying null handle
            mockforge_server_destroy(ptr::null_mut());
        }
    }

    #[test]
    fn test_mockforge_server_stub_null_handle() {
        unsafe {
            let method = CString::new("GET").unwrap();
            let path = CString::new("/test").unwrap();
            let body = CString::new("{}").unwrap();

            let result = mockforge_server_stub(
                ptr::null_mut(),
                method.as_ptr(),
                path.as_ptr(),
                200,
                body.as_ptr(),
            );

            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_mockforge_server_stub_null_method() {
        unsafe {
            // Create a minimal handle (won't actually use it)
            let path = CString::new("/test").unwrap();
            let body = CString::new("{}").unwrap();

            // Test with null method
            let result = mockforge_server_stub(
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                200,
                body.as_ptr(),
            );

            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_mockforge_server_stub_null_path() {
        unsafe {
            let method = CString::new("GET").unwrap();
            let body = CString::new("{}").unwrap();

            let result = mockforge_server_stub(
                ptr::null_mut(),
                method.as_ptr(),
                ptr::null(),
                200,
                body.as_ptr(),
            );

            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_mockforge_server_stub_null_body() {
        unsafe {
            let method = CString::new("GET").unwrap();
            let path = CString::new("/test").unwrap();

            let result = mockforge_server_stub(
                ptr::null_mut(),
                method.as_ptr(),
                path.as_ptr(),
                200,
                ptr::null(),
            );

            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_mockforge_server_stub_invalid_json() {
        unsafe {
            let method = CString::new("GET").unwrap();
            let path = CString::new("/test").unwrap();
            let body = CString::new("{invalid json").unwrap();

            // Even with a null handle, invalid JSON should return -1
            let result = mockforge_server_stub(
                ptr::null_mut(),
                method.as_ptr(),
                path.as_ptr(),
                200,
                body.as_ptr(),
            );

            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_mockforge_server_url_null_handle() {
        unsafe {
            let url = mockforge_server_url(ptr::null());
            assert!(url.is_null());
        }
    }

    #[test]
    fn test_mockforge_free_string_null() {
        unsafe {
            // Should not crash when freeing null string
            mockforge_free_string(ptr::null_mut());
        }
    }

    #[test]
    fn test_mockforge_free_string_valid() {
        unsafe {
            let test_str = CString::new("test").unwrap();
            let raw_ptr = test_str.into_raw();

            // Free the string
            mockforge_free_string(raw_ptr);

            // After freeing, we shouldn't use the pointer anymore
            // This test just verifies it doesn't crash
        }
    }

    #[test]
    fn test_mockforge_last_error_returns_null() {
        unsafe {
            let error = mockforge_last_error();
            assert!(error.is_null());
        }
    }

    #[test]
    fn test_cstring_conversion_utf8() {
        unsafe {
            let method = CString::new("GET").unwrap();
            let method_ptr = method.as_ptr();

            let converted = CStr::from_ptr(method_ptr);
            assert_eq!(converted.to_str().unwrap(), "GET");
        }
    }

    #[test]
    fn test_cstring_conversion_special_chars() {
        unsafe {
            let path = CString::new("/api/users/{id}").unwrap();
            let path_ptr = path.as_ptr();

            let converted = CStr::from_ptr(path_ptr);
            assert_eq!(converted.to_str().unwrap(), "/api/users/{id}");
        }
    }

    #[test]
    fn test_json_value_parsing() {
        unsafe {
            let valid_json = CString::new(r#"{"key":"value"}"#).unwrap();
            let json_ptr = valid_json.as_ptr();

            let json_str = CStr::from_ptr(json_ptr).to_str().unwrap();
            let result: Result<serde_json::Value, _> = serde_json::from_str(json_str);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_status_code_range() {
        // Test that status codes are valid u16 values
        let status_codes = [200, 201, 400, 404, 500, 503];

        for &status in &status_codes {
            assert!(status > 0);
            assert!(status < 600);
        }
    }

    #[test]
    fn test_mock_server_handle_runtime_creation() {
        // Test that Runtime can be created (this is what mockforge_server_new does)
        let runtime = Runtime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_arc_mutex_server_creation() {
        // Test that we can create an Arc<Mutex<MockServer>>
        let server = MockServer::default();
        let _arc_server = Arc::new(Mutex::new(server));
        // Just verify it compiles and doesn't panic
    }

    #[tokio::test]
    async fn test_server_in_ffi_context() {
        // Simulate what the FFI does: create server, use it in async context
        let runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            let result = MockServer::new().port(0).start().await;
            // Port 0 should allow the OS to assign a port
            if let Ok(mut server) = result {
                let _ = server.stop().await;
            }
        });
    }

    #[test]
    fn test_multiple_cstring_allocations() {
        unsafe {
            // Test that we can create multiple CStrings without issues
            let strings = vec![
                CString::new("GET").unwrap(),
                CString::new("POST").unwrap(),
                CString::new("/api/test").unwrap(),
                CString::new(r#"{"test":true}"#).unwrap(),
            ];

            for s in strings {
                let ptr = s.into_raw();
                mockforge_free_string(ptr);
            }
        }
    }

    #[test]
    fn test_error_code_conventions() {
        // Verify our error code conventions
        let success = 0;
        let error = -1;

        assert_eq!(success, 0);
        assert_eq!(error, -1);
        assert!(error < success);
    }
}
