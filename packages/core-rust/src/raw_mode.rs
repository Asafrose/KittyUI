//! Enter and exit terminal raw mode via termios.
//!
//! Raw mode disables line buffering and echo so the application receives
//! each keypress immediately.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static RAW_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Saved original termios state so we can restore it later.
static mut ORIGINAL_TERMIOS: Option<libc::termios> = None;

/// Enter raw mode on the terminal attached to stdin.
///
/// # Errors
///
/// Returns an error if the fd is not a terminal or if `tcgetattr`/`tcsetattr` fails.
///
/// # Safety
///
/// Modifies global terminal state. Only one raw mode session should be active at a time.
pub fn enter() -> io::Result<()> {
    if RAW_MODE_ACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }

    let fd = libc::STDIN_FILENO;

    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &raw mut termios) != 0 {
            return Err(io::Error::last_os_error());
        }

        ORIGINAL_TERMIOS = Some(termios);

        let mut raw = termios;
        libc::cfmakeraw(&raw mut raw);
        // Keep SIGINT/SIGTSTP working
        raw.c_lflag |= libc::ISIG;
        // Read returns after 1 byte, no timeout
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(fd, libc::TCSAFLUSH, &raw const raw) != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    RAW_MODE_ACTIVE.store(true, Ordering::SeqCst);
    Ok(())
}

/// Exit raw mode, restoring the original terminal settings.
///
/// # Errors
///
/// Returns an error if `tcsetattr` fails.
pub fn exit() -> io::Result<()> {
    if !RAW_MODE_ACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }

    unsafe {
        if let Some(ref original) = ORIGINAL_TERMIOS {
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, original) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    RAW_MODE_ACTIVE.store(false, Ordering::SeqCst);
    Ok(())
}

/// Returns whether raw mode is currently active.
#[must_use]
pub fn is_active() -> bool {
    RAW_MODE_ACTIVE.load(Ordering::SeqCst)
}
