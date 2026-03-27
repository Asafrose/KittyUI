//! `KittyUI` Core — Rust rendering engine for the Kitty graphics protocol.
//!
//! This crate exposes a C ABI that Bun loads via `bun:ffi` (see issue #2).

pub mod animation;
pub mod ansi;
pub mod buffer;
pub mod caps;
pub mod cell;
pub mod cleanup;
pub mod ffi_bridge;
pub mod focus;
pub mod hit_test;
pub mod image;
pub mod image_placement;
pub mod keyboard;
pub mod layout;
pub mod mock_terminal;
pub mod mouse;
pub mod pixel_canvas;
pub mod raw_mode;
pub mod render_loop;
pub mod screen;
pub mod signals;
pub mod virtual_placement;

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
