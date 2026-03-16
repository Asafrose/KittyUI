/// KittyUI Core — Rust rendering engine for the Kitty graphics protocol.
///
/// This crate exposes a C ABI that Bun loads via `bun:ffi` (see issue #2).

/// Placeholder function demonstrating the C ABI pattern used by bun:ffi.
#[no_mangle]
pub extern "C" fn hello() -> *const u8 {
    b"Hello from kittyui-core!\0".as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let ptr = hello();
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char) };
        assert_eq!(cstr.to_str().unwrap(), "Hello from kittyui-core!");
    }
}
