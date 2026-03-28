#!/usr/bin/env python3
"""
PTY-based visual test harness for the KittyUI dashboard.

Spawns the dashboard inside a real pseudo-terminal with a controlled size,
sends keystrokes to navigate between tabs, and relies on the app's built-in
screenshot timer (--screenshot-dir) to capture PNGs.

Usage:
    python3 examples/dashboard/visual-test.py

Screenshots are saved to /tmp/kittyui-visual-test/latest.png
"""

import fcntl
import os
import pty
import select
import signal
import struct
import sys
import termios
import time

SCREENSHOT_DIR = "/tmp/kittyui-visual-test"
ROWS = 50
COLS = 160
XPIXEL = 1280
YPIXEL = 800

# Escape sequences for arrow keys
RIGHT_ARROW = b"\x1b[C"
LEFT_ARROW = b"\x1b[D"
QUIT_KEY = b"q"


def set_pty_size(fd, rows, cols, xpixel=0, ypixel=0):
    """Set the terminal size on a PTY file descriptor."""
    winsize = struct.pack("HHHH", rows, cols, xpixel, ypixel)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)


def drain_output(master_fd, timeout=0.1):
    """Read and discard any pending output from the PTY master."""
    while True:
        ready, _, _ = select.select([master_fd], [], [], timeout)
        if not ready:
            break
        try:
            data = os.read(master_fd, 65536)
            if not data:
                break
        except OSError:
            break


def wait_for_screenshot(path, master_fd, timeout=15):
    """Wait until a screenshot file exists, continuously draining PTY output."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if os.path.exists(path) and os.path.getsize(path) > 0:
            return True
        # Keep draining PTY output to prevent buffer deadlock
        drain_output(master_fd, timeout=0.2)
    return False


def main():
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)

    screenshot_path = os.path.join(SCREENSHOT_DIR, "latest.png")

    # Remove stale screenshot so we can detect a fresh one
    if os.path.exists(screenshot_path):
        os.unlink(screenshot_path)

    # Open a PTY pair
    master_fd, slave_fd = pty.openpty()

    # Set the PTY size BEFORE forking so the child inherits it
    set_pty_size(slave_fd, ROWS, COLS, XPIXEL, YPIXEL)

    pid = os.fork()

    if pid == 0:
        # ---- Child process ----
        os.close(master_fd)
        os.setsid()

        # Make the slave our controlling terminal
        fcntl.ioctl(slave_fd, termios.TIOCSCTTY, 0)

        # Redirect stdio to the PTY slave
        os.dup2(slave_fd, 0)
        os.dup2(slave_fd, 1)
        os.dup2(slave_fd, 2)
        if slave_fd > 2:
            os.close(slave_fd)

        # Set environment so the app detects Kitty graphics support
        os.environ["TERM"] = "xterm-kitty"
        os.environ["TERM_PROGRAM"] = "kitty"
        os.environ["KITTY_WINDOW_ID"] = "1"
        os.environ["COLORTERM"] = "truecolor"
        os.environ["KITTYUI_SCREENSHOT"] = os.path.join(SCREENSHOT_DIR, "latest.png")

        os.execvp(
            "bun",
            [
                "bun",
                "run",
                "examples/dashboard/src/main.tsx",
                "--screenshot-dir",
                SCREENSHOT_DIR,
            ],
        )
        # execvp never returns on success
        sys.exit(1)

    # ---- Parent process ----
    os.close(slave_fd)

    print(f"[visual-test] Started dashboard (pid={pid}) in {COLS}x{ROWS} PTY")
    print(f"[visual-test] Pixel dimensions: {XPIXEL}x{YPIXEL}")
    print(f"[visual-test] Waiting for initial render...")

    # Drain child output continuously in a background thread to prevent
    # PTY buffer deadlock (pixel renderer outputs large Kitty images).
    import threading
    drain_stop = threading.Event()
    def drain_thread():
        while not drain_stop.is_set():
            drain_output(master_fd, timeout=0.1)
    drainer = threading.Thread(target=drain_thread, daemon=True)
    drainer.start()

    # Wait for the first screenshot to appear
    if not wait_for_screenshot(screenshot_path, master_fd, timeout=20):
        print("[visual-test] ERROR: No screenshot produced within 20 seconds.", file=sys.stderr)
        os.kill(pid, signal.SIGTERM)
        os.waitpid(pid, 0)
        os.close(master_fd)
        sys.exit(1)

    initial_size = os.path.getsize(screenshot_path)
    print(f"[visual-test] Frame 1 (Overview): screenshot captured ({initial_size} bytes)")
    drain_output(master_fd)

    # Navigate: right arrow -> Analytics tab
    os.write(master_fd, RIGHT_ARROW)
    time.sleep(1.5)
    drain_output(master_fd)
    print("[visual-test] Frame 2: Sent right arrow (Analytics tab)")

    # Navigate: right arrow -> Reports tab
    os.write(master_fd, RIGHT_ARROW)
    time.sleep(1.5)
    drain_output(master_fd)
    print("[visual-test] Frame 3: Sent right arrow (Reports tab)")

    # Navigate: left arrow -> back to Analytics
    os.write(master_fd, LEFT_ARROW)
    time.sleep(1.5)
    drain_output(master_fd)
    print("[visual-test] Frame 4: Sent left arrow (back to Analytics)")

    # Send quit
    os.write(master_fd, QUIT_KEY)
    time.sleep(1.0)
    drain_output(master_fd)

    # Give the child a moment to exit, then force-kill
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass  # Already exited

    try:
        os.waitpid(pid, 0)
    except ChildProcessError:
        pass

    os.close(master_fd)

    # Verify output
    if os.path.exists(screenshot_path) and os.path.getsize(screenshot_path) > 0:
        size = os.path.getsize(screenshot_path)
        print(f"\n[visual-test] SUCCESS: Screenshot saved to {screenshot_path} ({size} bytes)")

        # Try to read PNG dimensions (first 24 bytes contain the IHDR chunk)
        try:
            with open(screenshot_path, "rb") as f:
                header = f.read(32)
                if header[:8] == b"\x89PNG\r\n\x1a\n":
                    width = struct.unpack(">I", header[16:20])[0]
                    height = struct.unpack(">I", header[20:24])[0]
                    print(f"[visual-test] PNG dimensions: {width}x{height}")
                    if width > 640 and height > 384:
                        print("[visual-test] PASS: Resolution exceeds 640x384 threshold")
                    else:
                        print("[visual-test] WARN: Resolution below expected threshold", file=sys.stderr)
        except Exception as e:
            print(f"[visual-test] Could not read PNG header: {e}", file=sys.stderr)
    else:
        print("\n[visual-test] FAIL: No screenshot found.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
