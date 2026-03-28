#!/usr/bin/env python3
"""
Screenshot capture for KittyUI dashboard.

Run this script DIRECTLY in your terminal (not from Claude Code):

    python3 examples/dashboard/screenshot.py

It will:
1. Launch the dashboard in your terminal
2. Wait for the first pixel-rendered frame
3. Save a screenshot to /tmp/kittyui-screenshot.png
4. Quit the app

The screenshot is captured via the KITTYUI_SCREENSHOT env var which
tells the Rust pixel renderer to save each frame as a PNG.
"""
import os
import sys
import time
import subprocess
import signal

SCREENSHOT = "/tmp/kittyui-screenshot.png"
TIMEOUT = 15

def main():
    # Remove stale screenshot
    try:
        os.unlink(SCREENSHOT)
    except FileNotFoundError:
        pass

    env = dict(os.environ)
    env["KITTYUI_SCREENSHOT"] = SCREENSHOT

    print(f"Starting dashboard... (screenshot will be saved to {SCREENSHOT})")
    print(f"Press Ctrl+C or wait {TIMEOUT}s for auto-capture.\n")

    # Run the app directly (inherits the current TTY)
    proc = subprocess.Popen(
        ["bun", "examples/dashboard/src/main.tsx"],
        env=env,
    )

    try:
        for i in range(TIMEOUT):
            time.sleep(1)
            if os.path.exists(SCREENSHOT) and os.path.getsize(SCREENSHOT) > 0:
                size = os.path.getsize(SCREENSHOT)
                proc.send_signal(signal.SIGTERM)
                proc.wait(timeout=3)
                print(f"\nScreenshot saved: {SCREENSHOT} ({size} bytes)")
                return 0
    except KeyboardInterrupt:
        pass

    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()

    if os.path.exists(SCREENSHOT) and os.path.getsize(SCREENSHOT) > 0:
        size = os.path.getsize(SCREENSHOT)
        print(f"\nScreenshot saved: {SCREENSHOT} ({size} bytes)")
        return 0

    print(f"\nNo screenshot produced. Make sure you're running in a terminal.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
