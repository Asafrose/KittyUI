/**
 * Visual test runner for the dashboard.
 * Runs the app in a PTY, sends keystrokes, and saves screenshots.
 *
 * Usage:
 *   bun run examples/dashboard/visual-test.ts
 *
 * Screenshots are saved to /tmp/kittyui-visual-test/latest.png
 */
import { spawn } from "bun";
import { existsSync, mkdirSync } from "fs";

const SCREENSHOT_DIR = "/tmp/kittyui-visual-test";
const APP_PATH = "examples/dashboard/src/main.tsx";

async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  mkdirSync(SCREENSHOT_DIR, { recursive: true });

  // Start the app with pixel mode and screenshot capture
  const proc = spawn({
    cmd: [
      "bun",
      "run",
      APP_PATH,
      "--pixel",
      "--screenshot-dir",
      SCREENSHOT_DIR,
    ],
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      TERM_PROGRAM: "kitty",
      KITTY_WINDOW_ID: "1",
      COLUMNS: "130",
      LINES: "54",
    },
  });

  // Wait for first render
  await sleep(3000);
  console.log(`Frame 1: Overview page saved`);

  // Send right arrow to switch to Analytics tab
  proc.stdin.write("\x1b[C");
  await sleep(1000);
  console.log("Frame 2: After right arrow");

  // Send right arrow again for Reports tab
  proc.stdin.write("\x1b[C");
  await sleep(1000);
  console.log("Frame 3: After second right arrow");

  // Send left arrow back
  proc.stdin.write("\x1b[D");
  await sleep(1000);
  console.log("Frame 4: After left arrow");

  // Quit
  proc.stdin.write("q");
  await sleep(500);

  const screenshotPath = `${SCREENSHOT_DIR}/latest.png`;
  if (existsSync(screenshotPath)) {
    console.log(`\nScreenshots saved to ${SCREENSHOT_DIR}/`);
    console.log(`Latest: ${screenshotPath}`);
  } else {
    console.error(
      "\nWarning: No screenshot found. The pixel renderer may not have been active.",
    );
    console.error(
      "Make sure the terminal supports Kitty graphics or pass --pixel to force it.",
    );
  }

  proc.kill();
  process.exit(0);
}

main().catch(console.error);
