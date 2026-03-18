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

    #[test]
    fn sigwinch_sets_resize_flag() {
        reset_flags();
        register().unwrap();

        // Send SIGWINCH to ourselves.
        unsafe {
            libc::raise(libc::SIGWINCH);
        }
        // Poll with back-off — CI runners can be slow to deliver signals.
        let mut received = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if resize_received() {
                received = true;
                break;
            }
        }
        assert!(received, "SIGWINCH was not delivered within 200 ms");
    }
}
