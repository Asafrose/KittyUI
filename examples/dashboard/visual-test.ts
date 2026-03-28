/**
 * Visual test runner for the dashboard.
 *
 * This is a thin wrapper that invokes the Python PTY-based test harness.
 * The Python script creates a real pseudo-terminal, which solves:
 *   - process.stdin raw mode (requires a TTY)
 *   - ioctl terminal size (TIOCSWINSZ sets custom dimensions)
 *   - process.stdout.columns being undefined
 *
 * Usage:
 *   bun run examples/dashboard/visual-test.ts
 *   # or directly:
 *   python3 examples/dashboard/visual-test.py
 *
 * Screenshots are saved to /tmp/kittyui-visual-test/latest.png
 */
import { spawnSync } from "bun";
import { resolve } from "path";

const scriptDir = import.meta.dir;
const pythonScript = resolve(scriptDir, "visual-test.py");

console.log("Starting PTY-based visual test harness...\n");

const result = spawnSync({
  cmd: ["python3", pythonScript],
  cwd: resolve(scriptDir, "../.."),
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
  env: {
    ...process.env,
  },
});

process.exit(result.exitCode ?? 1);
