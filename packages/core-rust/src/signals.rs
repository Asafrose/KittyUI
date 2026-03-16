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
