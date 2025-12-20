//! FFI bindings for Code Bridge
//!
//! This crate provides C-compatible FFI bindings for use with Swift via swift-bridge
//! or direct C FFI.

use bridge_core::{Bridge, Config};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Opaque handle to a Bridge instance
pub struct BridgeHandle {
    bridge: Bridge,
    runtime: tokio::runtime::Runtime,
}

/// Create a new Bridge instance
///
/// Returns a pointer to the bridge handle, or null on failure.
/// The caller is responsible for calling `bridge_destroy` to free the memory.
#[no_mangle]
pub extern "C" fn bridge_create() -> *mut BridgeHandle {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return ptr::null_mut(),
    };

    let bridge = match runtime.block_on(async { Bridge::new().await }) {
        Ok(b) => b,
        Err(_) => return ptr::null_mut(),
    };

    let handle = Box::new(BridgeHandle { bridge, runtime });
    Box::into_raw(handle)
}

/// Destroy a Bridge instance
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_destroy(handle: *mut BridgeHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// Start the bridge (networking, syncing)
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_start(handle: *mut BridgeHandle) -> bool {
    if handle.is_null() {
        return false;
    }

    let handle = &mut *handle;
    handle
        .runtime
        .block_on(async { handle.bridge.start().await })
        .is_ok()
}

/// Stop the bridge
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_stop(handle: *mut BridgeHandle) -> bool {
    if handle.is_null() {
        return false;
    }

    let handle = &mut *handle;
    handle
        .runtime
        .block_on(async { handle.bridge.stop().await })
        .is_ok()
}

/// Get the device name
///
/// Returns a newly allocated C string that must be freed with `bridge_free_string`.
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_get_device_name(handle: *const BridgeHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = &*handle;
    let name = &handle.bridge.config().device_name;

    match CString::new(name.as_str()) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Get the device ID
///
/// Returns a newly allocated C string that must be freed with `bridge_free_string`.
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_get_device_id(handle: *const BridgeHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = &*handle;
    let id = &handle.bridge.config().device_id;

    match CString::new(id.as_str()) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Get the number of connected peers
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_get_peer_count(handle: *const BridgeHandle) -> usize {
    if handle.is_null() {
        return 0;
    }

    let handle = &*handle;
    handle.bridge.network().peers().len()
}

/// Add a file to the content store
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
/// The path must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn bridge_add_file(
    handle: *mut BridgeHandle,
    path: *const c_char,
) -> bool {
    if handle.is_null() || path.is_null() {
        return false;
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return false;
    }

    // TODO: Actually add to storage
    // For now, just return success if file exists
    true
}

/// Free a string allocated by this library
///
/// # Safety
/// The string must have been allocated by this library (e.g., from `bridge_get_device_name`).
#[no_mangle]
pub unsafe extern "C" fn bridge_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// Get the local peer ID as a string
///
/// Returns a newly allocated C string that must be freed with `bridge_free_string`.
///
/// # Safety
/// The handle must be a valid pointer returned by `bridge_create`.
#[no_mangle]
pub unsafe extern "C" fn bridge_get_local_peer_id(handle: *const BridgeHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    let handle = &*handle;
    let peer_id = handle.bridge.network().local_peer_id().to_string();

    match CString::new(peer_id) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_destroy() {
        unsafe {
            let handle = bridge_create();
            assert!(!handle.is_null());
            bridge_destroy(handle);
        }
    }
}
