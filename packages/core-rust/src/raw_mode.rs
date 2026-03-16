//! Enter and exit terminal raw mode via termios.
//!
//! Raw mode disables line buffering and echo so the application receives
//! each keypress immediately.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) static RAW_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

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

/// Apply raw mode settings to a termios struct (for testing).
///
/// This replicates the flag changes that `enter()` makes, without
/// touching any real file descriptor.
#[cfg(test)]
fn make_raw(termios: &mut libc::termios) {
    unsafe {
        libc::cfmakeraw(&raw mut *termios);
    }
    termios.c_lflag |= libc::ISIG;
    termios.c_cc[libc::VMIN] = 1;
    termios.c_cc[libc::VTIME] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reset the module's global state between tests.
    ///
    /// # Safety
    ///
    /// Must not be called concurrently with `enter`/`exit`.
    unsafe fn reset_state() {
        RAW_MODE_ACTIVE.store(false, Ordering::SeqCst);
        unsafe { ORIGINAL_TERMIOS = None };
    }

    #[test]
    fn is_active_initially_false() {
        assert!(!is_active());
    }

    #[test]
    fn exit_is_noop_when_not_active() {
        // Should succeed without error even when not in raw mode.
        assert!(exit().is_ok());
    }

    #[test]
    fn make_raw_disables_echo_and_icanon() {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        // Set ECHO and ICANON so we can verify they get cleared.
        t.c_lflag = libc::ECHO | libc::ICANON | libc::ISIG;

        make_raw(&mut t);

        // cfmakeraw clears ECHO and ICANON
        assert_eq!(t.c_lflag & libc::ECHO, 0, "ECHO should be cleared");
        assert_eq!(t.c_lflag & libc::ICANON, 0, "ICANON should be cleared");
    }

    #[test]
    fn make_raw_preserves_isig() {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        make_raw(&mut t);

        // We explicitly re-enable ISIG after cfmakeraw
        assert_ne!(t.c_lflag & libc::ISIG, 0, "ISIG should be set");
    }

    #[test]
    fn make_raw_sets_vmin_and_vtime() {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        make_raw(&mut t);

        assert_eq!(t.c_cc[libc::VMIN], 1, "VMIN should be 1");
        assert_eq!(t.c_cc[libc::VTIME], 0, "VTIME should be 0");
    }

    #[test]
    fn make_raw_disables_input_processing() {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        t.c_iflag = libc::IXON | libc::ICRNL | libc::BRKINT;

        make_raw(&mut t);

        // cfmakeraw clears software flow control and CR-to-NL mapping
        assert_eq!(t.c_iflag & libc::IXON, 0, "IXON should be cleared");
        assert_eq!(t.c_iflag & libc::ICRNL, 0, "ICRNL should be cleared");
    }

    #[test]
    fn make_raw_disables_output_processing() {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        t.c_oflag = libc::OPOST;

        make_raw(&mut t);

        assert_eq!(t.c_oflag & libc::OPOST, 0, "OPOST should be cleared");
    }

    #[test]
    fn double_enter_does_not_corrupt_state() {
        // Simulate the logic: if already active, enter() returns early.
        // We test this by checking the state flag.
        unsafe { reset_state() };

        RAW_MODE_ACTIVE.store(true, Ordering::SeqCst);
        // Second enter should be a no-op (returns Ok immediately).
        assert!(enter().is_ok());
        assert!(is_active());

        unsafe { reset_state() };
    }
}
