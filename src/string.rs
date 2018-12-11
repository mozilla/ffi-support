/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use log::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Convert a rust string into a NUL-terminated utf-8 string suitable for passing to C, or to things
/// ABI-compatible with C.
///
/// Important: This string must eventually be freed. You may either do that using the
/// [`destroy_c_string`] method (or, if you must, by dropping the underlying [`std::ffi::CString`]
/// after recovering it via [`std::ffi::CString::from_raw`]).
///
/// It's common to want to allow the consumer (e.g. on the "C" side of the FFI) to be allowed to
/// free this memory, and the macro [`define_string_destructor!`] may be used to do so.
///
/// ## Panics
///
/// This function may panic if the argument has an interior null byte. This is fairly rare, but
/// is possible in theory.
#[inline]
pub fn rust_string_to_c(rust_string: impl Into<String>) -> *mut c_char {
    CString::new(rust_string.into())
        .expect("Error: Rust string contained an interior null byte.")
        .into_raw()
}

/// Variant of [`rust_string_to_c`] which takes an Option, and returns null for None.
#[inline]
pub fn opt_rust_string_to_c(opt_rust_string: Option<impl Into<String>>) -> *mut c_char {
    if let Some(s) = opt_rust_string {
        rust_string_to_c(s)
    } else {
        ptr::null_mut()
    }
}

/// Free the memory of a string created by [`rust_string_to_c`] on the rust heap. If `c_string` is
/// null, this is a no-op.
///
/// See the [`define_string_destructor!`] macro which may be used for exposing this function over
/// the FFI.
///
/// ## Safety
///
/// This is inherently unsafe, since we're deallocating memory. Be sure
///
/// - Nobody can use the memory after it's deallocated.
/// - The memory was actually allocated on this heap (and it's not a string from the other side of
///   the FFI which was allocated on e.g. the C heap).
///     - If multiple separate rust libraries are in use (for example, as DLLs) in a single program,
///       you must also make sure that the rust library that allocated the memory is also the one
///       that frees it.
///
/// See documentation for [`define_string_destructor!`], which gives a more complete overview of the
/// potential issues.
#[inline]
pub unsafe fn destroy_c_string(cstring: *mut c_char) {
    // we're not guaranteed to be in a place where we can complain about this beyond logging,
    // and there's an obvious way to handle it.
    if !cstring.is_null() {
        drop(CString::from_raw(cstring))
    }
}

/// Convert a null-terminated C string to a rust `str`. This does not take ownership of the string,
/// and you should be careful about the lifetime of the resulting string. Note that strings
/// containing invalid UTF-8 are replaced with the empty string (for many cases, you will want to
/// use [`rust_string_from_c`] instead, which will do a lossy conversion).
///
/// If you actually need an owned rust `String`, you're encouraged to use [`rust_string_from_c`],
/// which, as mentioned, also behaves better in the face of invalid UTF-8.
///
/// ## Safety
///
/// This is unsafe because we read from a raw pointer, which may or may not be valid.
///
/// We also assume `c_string` is a null terminated string, and have no way of knowing if that's
/// actually true. If it's not, we'll read arbitrary memory from the heap until we see a '\0', which
/// can result in a enormous number of problems.
///
/// ## Panics
///
/// Panics if it's argument is null, see [`opt_rust_str_from_c`] for a variant that returns None in
/// this case instead.
///
/// Note: This means it's forbidden to call this outside of a `call_with_result` (or something else
/// that uses [`std::panic::catch_unwind`]), as it is UB to panic across the FFI boundary.
#[inline]
pub unsafe fn rust_str_from_c<'a>(c_string: *const c_char) -> &'a str {
    opt_rust_str_from_c(c_string).expect("Null string passed to `rust_str_from_c`")
}

/// Same as `rust_string_from_c`, but returns None if `c_string` is null instead of asserting.
///
/// ## Safety
///
/// This is unsafe because we read from a raw pointer, which may or may not be valid.
///
/// We also assume `c_string` is a null terminated string, and have no way of knowing if that's
/// actually true. If it's not, we'll read arbitrary memory from the heap until we see a '\0', which
/// can result in a enormous number of problems.
#[inline]
pub unsafe fn opt_rust_str_from_c<'a>(c_string: *const c_char) -> Option<&'a str> {
    if !c_string.is_null() {
        match CStr::from_ptr(c_string).to_str() {
            Ok(s) => Some(s),
            Err(e) => {
                // Currently I think this is impossible, so I'd like to know if it can happen
                // (Java/Swift shouldn't pass us invalid UTF-8... right?).
                error!(
                    "[Bug] Invalid UTF-8 was passed to rust! Report this! {:?}",
                    e
                );
                Some("")
            }
        }
    } else {
        None
    }
}

/// Convert a null-terminated C into an owned rust string, replacing invalid UTF-8 with the
/// unicode replacement character.
///
/// ## Safety
///
/// This is unsafe because we dereference a raw pointer, which may or may not be valid.
///
/// We also assume `c_string` is a null terminated string, and have no way of knowing if that's
/// actually true. If it's not, we'll read arbitrary memory from the heap until we see a '\0', which
/// can result in a enormous number of problems.
///
/// ## Panics
///
/// Panics if it's argument is null. See also [`opt_rust_string_from_c`], which returns None
/// instead.
///
/// Note: This means it's forbidden to call this outside of a `call_with_result` (or something else
/// that uses `std::panic::catch_unwind`), as it is UB to panic across the FFI boundary.
#[inline]
pub unsafe fn rust_string_from_c(c_string: *const c_char) -> String {
    opt_rust_string_from_c(c_string).expect("Null string passed to `rust_string_from_c`")
}

/// Same as `rust_string_from_c`, but returns None if `c_string` is null instead of asserting.
///
/// ## Safety
///
/// This is unsafe because we dereference a raw pointer, which may or may not be valid.
///
/// We also assume `c_string` is a null terminated string, and have no way of knowing if that's
/// actually true. If it's not, we'll read arbitrary memory from the heap until we see a '\0', which
/// can result in a enormous number of problems.
#[inline]
pub unsafe fn opt_rust_string_from_c(c_string: *const c_char) -> Option<String> {
    if !c_string.is_null() {
        Some(CStr::from_ptr(c_string).to_string_lossy().to_string())
    } else {
        None
    }
}
