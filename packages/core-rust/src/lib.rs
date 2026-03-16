//! `KittyUI` Core — Rust rendering engine for the Kitty graphics protocol.
//!
//! This crate exposes a C ABI that Bun loads via `bun:ffi` (see issue #2).

pub mod buffer;
pub mod cleanup;
pub mod mock_terminal;
pub mod mouse;
pub mod raw_mode;
pub mod screen;
pub mod signals;

use std::ffi::c_char;

/// Placeholder function demonstrating the C ABI pattern used by bun:ffi.
#[no_mangle]
pub extern "C" fn hello() -> *const c_char {
    c"Hello from kittyui-core!".as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let ptr = hello();
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(cstr.to_str().unwrap(), "Hello from kittyui-core!");
    }
}
