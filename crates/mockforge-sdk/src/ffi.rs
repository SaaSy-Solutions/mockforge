//! FFI bindings for using MockForge from other languages (Python, Node.js, Go)
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

/// Opaque handle to a MockServer
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
    let server = runtime.block_on(async {
        MockServer::new()
            .port(port)
            .start()
            .await
    });

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
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get the server URL
///
/// # Safety
/// - handle must be valid
/// - Returns a C string that must be freed with mockforge_free_string
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

/// Free a string returned by MockForge
///
/// # Safety
/// The string must have been allocated by MockForge
#[no_mangle]
pub unsafe extern "C" fn mockforge_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Get the last error message
///
/// # Safety
/// Returns a C string that must be freed with mockforge_free_string
#[no_mangle]
pub unsafe extern "C" fn mockforge_last_error() -> *mut c_char {
    // Thread-local error storage could be implemented here
    ptr::null_mut()
}
