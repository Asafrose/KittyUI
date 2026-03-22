//! Signal handling for terminal lifecycle events.
//!
//! - `SIGWINCH`: terminal resize — readers poll `resize_received()` to detect changes.
//! - `SIGTERM` / `SIGINT`: graceful shutdown — triggers terminal cleanup.

use std::sync::atomic::{AtomicBool, Ordering};

use signal_hook::consts::{SIGINT, SIGTERM, SIGWINCH};

static RESIZE_FLAG: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Register signal handlers for `SIGWINCH`, `SIGTERM`, and `SIGINT`.
///
/// # Errors
///
/// Returns an error if signal registration fails.
pub fn register() -> std::io::Result<()> {
    unsafe {
        signal_hook::low_level::register(SIGWINCH, || {
            RESIZE_FLAG.store(true, Ordering::SeqCst);
        })?;
        signal_hook::low_level::register(SIGTERM, || {
            SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
        })?;
        signal_hook::low_level::register(SIGINT, || {
            SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
        })?;
    }
    Ok(())
}

/// Check if a terminal resize signal was received since the last call.
///
/// Clears the flag after reading, so subsequent calls return `false`
/// until the next `SIGWINCH`.
pub fn resize_received() -> bool {
    RESIZE_FLAG.swap(false, Ordering::SeqCst)
}

/// Check if a shutdown signal (`SIGTERM` or `SIGINT`) was received.
pub fn shutdown_requested() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reset flags between tests.
    fn reset_flags() {
        RESIZE_FLAG.store(false, Ordering::SeqCst);
        SHUTDOWN_FLAG.store(false, Ordering::SeqCst);
    }

    #[test]
    fn flags_initially_false() {
        reset_flags();
        assert!(!resize_received());
        assert!(!shutdown_requested());
    }

    #[test]
    fn resize_received_clears_flag() {
        reset_flags();
        RESIZE_FLAG.store(true, Ordering::SeqCst);
        assert!(resize_received());
        // Second call should return false (flag was cleared).
        assert!(!resize_received());
    }

    #[test]
    fn shutdown_flag_persists() {
        reset_flags();
        SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
        assert!(shutdown_requested());
        // shutdown_requested does NOT clear the flag (unlike resize).
        assert!(shutdown_requested());
    }

    #[test]
    fn register_succeeds() {
        reset_flags();
        assert!(register().is_ok());
    }

    /// Verify that `SIGWINCH` delivery sets the resize flag.
    ///
    /// This test sends a real signal and polls for the flag.  Because Cargo
    /// runs tests in parallel and other tests (e.g. `ffi_bridge` init/shutdown)
    /// can call `register()` and `reset_flags()` concurrently, we retry the
    /// whole raise-and-poll cycle to tolerate a stolen flag.
    #[test]
    fn sigwinch_sets_resize_flag() {
        register().unwrap();

        let mut received = false;
        // Outer retry: if another parallel test consumed our flag, try again.
        for _attempt in 0..5 {
            reset_flags();
            unsafe {
                libc::raise(libc::SIGWINCH);
            }
            // Inner poll: wait for signal delivery.
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(5));
                if resize_received() {
                    received = true;
                    break;
                }
            }
            if received {
                break;
            }
        }
        assert!(received, "SIGWINCH was not delivered after 5 attempts");
    }
}
