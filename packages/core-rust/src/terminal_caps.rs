//! Environment-based terminal capability detection.
//!
//! Provides a synchronous, non-blocking capability detection layer that uses
//! environment variables and `ioctl` (on Unix) to determine terminal features
//! at startup.  Unlike [`crate::caps`] which uses escape-sequence queries,
//! this module never writes to the terminal.

use serde::Serialize;

/// Terminal capabilities detected at startup.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TerminalCaps {
    /// Whether the terminal supports the Kitty graphics protocol.
    pub kitty_graphics: bool,
    /// Whether the terminal supports 24-bit true color.
    pub true_color: bool,
    /// Terminal cell width in pixels (0 if unknown).
    pub cell_pixel_width: u32,
    /// Terminal cell height in pixels (0 if unknown).
    pub cell_pixel_height: u32,
    /// Terminal name if detected.
    pub terminal_name: Option<String>,
}

/// Detect capabilities from environment variables only (no I/O).
#[must_use]
pub fn detect_from_env() -> TerminalCaps {
    let mut caps = TerminalCaps::default();

    // TERM_PROGRAM
    if let Ok(prog) = std::env::var("TERM_PROGRAM") {
        caps.terminal_name = Some(prog.clone());
        if prog == "kitty" || prog == "WezTerm" {
            caps.kitty_graphics = true;
        }
    }

    // COLORTERM=truecolor
    if let Ok(ct) = std::env::var("COLORTERM") {
        if ct == "truecolor" || ct == "24bit" {
            caps.true_color = true;
        }
    }

    // TERM contains "256color"
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") || term.contains("kitty") {
            caps.true_color = true;
        }
        if term.contains("kitty") {
            caps.kitty_graphics = true;
        }
    }

    // KITTY_WINDOW_ID presence
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        caps.kitty_graphics = true;
    }

    // Check for KITTY_PID (set by Kitty even in tmux)
    if std::env::var("KITTY_PID").is_ok() {
        caps.kitty_graphics = true;
    }

    caps
}

/// Detect cell pixel dimensions via `ioctl(TIOCGWINSZ)`.
///
/// Returns `(cell_width, cell_height)` in pixels, or `(0, 0)` if unknown.
#[cfg(unix)]
#[must_use]
pub fn detect_cell_pixels() -> (u32, u32) {
    use std::mem::MaybeUninit;

    #[repr(C)]
    #[allow(clippy::struct_field_names)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    // TIOCGWINSZ = 0x5413 on Linux, 0x40087468 on macOS
    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: libc::c_ulong = 0x4008_7468;
    #[cfg(target_os = "linux")]
    const TIOCGWINSZ: libc::c_ulong = 0x5413;

    unsafe {
        let mut ws = MaybeUninit::<Winsize>::uninit();

        if libc::ioctl(libc::STDOUT_FILENO, TIOCGWINSZ, ws.as_mut_ptr()) == 0 {
            let ws = ws.assume_init();
            if ws.ws_xpixel > 0 && ws.ws_ypixel > 0 && ws.ws_col > 0 && ws.ws_row > 0 {
                return (
                    u32::from(ws.ws_xpixel) / u32::from(ws.ws_col),
                    u32::from(ws.ws_ypixel) / u32::from(ws.ws_row),
                );
            }
        }
    }
    (0, 0) // unknown
}

/// Combined detection: environment variables + ioctl pixel dimensions.
#[must_use]
pub fn detect() -> TerminalCaps {
    let mut caps = detect_from_env();

    #[cfg(unix)]
    {
        let (cw, ch) = detect_cell_pixels();
        caps.cell_pixel_width = cw;
        caps.cell_pixel_height = ch;
    }

    // Fallback pixel sizes if ioctl didn't work
    if caps.cell_pixel_width == 0 {
        caps.cell_pixel_width = 8; // standard VT100
        caps.cell_pixel_height = 16;
    }

    caps
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_caps_have_sensible_fallbacks() {
        let caps = TerminalCaps::default();
        assert!(!caps.kitty_graphics);
        assert!(!caps.true_color);
        assert_eq!(caps.cell_pixel_width, 0);
        assert_eq!(caps.cell_pixel_height, 0);
        assert!(caps.terminal_name.is_none());
    }

    #[test]
    fn detect_from_env_respects_term_program() {
        // Save and set env
        let prev = std::env::var("TERM_PROGRAM").ok();
        std::env::set_var("TERM_PROGRAM", "kitty");
        let caps = detect_from_env();
        assert!(caps.kitty_graphics);
        assert_eq!(caps.terminal_name.as_deref(), Some("kitty"));
        // Restore
        match prev {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
    }

    #[test]
    fn detect_from_env_respects_colorterm() {
        let prev = std::env::var("COLORTERM").ok();
        std::env::set_var("COLORTERM", "truecolor");
        let caps = detect_from_env();
        assert!(caps.true_color);
        match prev {
            Some(v) => std::env::set_var("COLORTERM", v),
            None => std::env::remove_var("COLORTERM"),
        }
    }

    #[test]
    fn detect_from_env_respects_kitty_window_id() {
        let prev = std::env::var("KITTY_WINDOW_ID").ok();
        std::env::set_var("KITTY_WINDOW_ID", "1");
        let caps = detect_from_env();
        assert!(caps.kitty_graphics);
        match prev {
            Some(v) => std::env::set_var("KITTY_WINDOW_ID", v),
            None => std::env::remove_var("KITTY_WINDOW_ID"),
        }
    }

    #[test]
    fn detect_from_env_wezterm() {
        let prev = std::env::var("TERM_PROGRAM").ok();
        std::env::set_var("TERM_PROGRAM", "WezTerm");
        let caps = detect_from_env();
        assert!(caps.kitty_graphics);
        assert_eq!(caps.terminal_name.as_deref(), Some("WezTerm"));
        match prev {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
    }

    #[test]
    fn detect_applies_pixel_fallback() {
        // detect() should always produce non-zero pixel sizes
        let caps = detect();
        assert!(caps.cell_pixel_width > 0);
        assert!(caps.cell_pixel_height > 0);
    }

    #[test]
    fn json_serialization_round_trips() {
        let caps = TerminalCaps {
            kitty_graphics: true,
            true_color: true,
            cell_pixel_width: 10,
            cell_pixel_height: 20,
            terminal_name: Some("kitty".to_string()),
        };
        let json = serde_json::to_string(&caps).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["kitty_graphics"], true);
        assert_eq!(parsed["true_color"], true);
        assert_eq!(parsed["cell_pixel_width"], 10);
        assert_eq!(parsed["cell_pixel_height"], 20);
        assert_eq!(parsed["terminal_name"], "kitty");
    }
}
