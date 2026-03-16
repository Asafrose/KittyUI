/// KittyUI Core — Rust rendering engine for the Kitty graphics protocol.
///
/// This crate will expose native bindings via napi-rs (see issue #2).

pub fn hello() -> String {
    "Hello from kittyui-core!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(hello(), "Hello from kittyui-core!");
    }
}
