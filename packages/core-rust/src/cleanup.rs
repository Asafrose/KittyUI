//! Panic and exit cleanup — ensures the terminal is restored even on crashes.

use crate::{raw_mode, screen};

/// Install a panic hook that restores the terminal before printing the
/// panic message.  Call this once at application startup.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort cleanup — ignore errors since we're panicking anyway.
        let _ = screen::exit();
        let _ = raw_mode::exit();
        default_hook(info);
    }));
}
