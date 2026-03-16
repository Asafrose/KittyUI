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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_panic_hook_succeeds() {
        // Should not panic when installing the hook.
        install_panic_hook();
    }

    #[test]
    fn panic_hook_restores_state_on_caught_panic() {
        install_panic_hook();

        // Simulate raw mode being active via the flag.
        use std::sync::atomic::Ordering;
        crate::raw_mode::RAW_MODE_ACTIVE.store(true, Ordering::SeqCst);

        let result = std::panic::catch_unwind(|| {
            panic!("test panic for cleanup verification");
        });

        assert!(result.is_err(), "panic should have been caught");
        // After the panic hook runs, raw mode should be deactivated.
        assert!(
            !crate::raw_mode::is_active(),
            "raw mode should be deactivated after panic"
        );
    }
}
