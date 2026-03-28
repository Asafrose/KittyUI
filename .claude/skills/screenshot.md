---
description: Take screenshots of terminal apps running in cmux. Use this skill whenever you need to see what a terminal app looks like, verify visual output, or iterate on UI design.
---

# Screenshot: Capture Terminal App Output via cmux

This skill enables visual QA by running terminal apps in a cmux split and capturing their pixel-rendered output as PNG screenshots.

## Prerequisites

- cmux is installed and running (`cmux identify` should return your current context)
- The app uses the Kitty graphics protocol for pixel rendering
- `KITTYUI_SCREENSHOT` env var triggers PNG frame capture in KittyUI apps

## Workflow

### 1. Create a new terminal split

```bash
cmux new-split right
# Returns: OK surface:<ID> workspace:<WS>
# Save the surface ID for subsequent commands
```

### 2. Run the app in the new surface

```bash
cmux send --surface surface:<ID> "cd /Users/asafrosentswaig/dev/KittyUI && KITTYUI_SCREENSHOT=/tmp/cmux-screenshot.png bun run examples/dashboard/src/main.tsx"
cmux send-key --surface surface:<ID> Enter
```

For other apps, replace the command. The key is setting `KITTYUI_SCREENSHOT` to a path where the pixel renderer will save each frame as a PNG.

### 3. Wait for the app to render

```bash
sleep 5
```

The app needs a few seconds to:
- Initialize the Rust engine
- Load fonts (fontdue loads Arial/system sans-serif)
- Render the first pixel frame
- Save the screenshot PNG

### 4. Read the screenshot

```bash
# Verify it exists
file /tmp/cmux-screenshot.png

# Read it with Claude's image tool
Read /tmp/cmux-screenshot.png
```

The Read tool can display PNG images directly, allowing visual inspection.

### 5. Interact with the app

Send keystrokes to navigate:

```bash
# Right arrow (switch tab)
cmux send-key --surface surface:<ID> Right

# Left arrow
cmux send-key --surface surface:<ID> Left

# Any character
cmux send --surface surface:<ID> "q"
```

Wait 1-2 seconds after each keystroke for the frame to re-render, then re-read the screenshot.

### 6. Clean up

```bash
# Quit the app (if it responds to 'q')
cmux send --surface surface:<ID> "q"
sleep 1

# Close the split
cmux close-surface --surface surface:<ID>
```

## Complete Example: Screenshot + Navigate + Screenshot

```bash
# Setup
SURFACE=$(cmux new-split right 2>&1 | awk '{print $2}')
cmux send --surface $SURFACE "cd /Users/asafrosentswaig/dev/KittyUI && KITTYUI_SCREENSHOT=/tmp/app-frame.png bun run examples/dashboard/src/main.tsx"
cmux send-key --surface $SURFACE Enter
sleep 5

# Take first screenshot
Read /tmp/app-frame.png

# Navigate to next tab
cmux send-key --surface $SURFACE Right
sleep 2
Read /tmp/app-frame.png

# Clean up
cmux send --surface $SURFACE "q"
sleep 1
cmux close-surface --surface $SURFACE
```

## Reading terminal text (non-pixel)

For text-based output without pixel rendering:

```bash
cmux read-screen --surface surface:<ID>
# or
cmux capture-pane --surface surface:<ID>
```

## Tips

- The screenshot is overwritten on every frame — always read the latest after waiting
- If the app hangs, the screenshot won't be produced — check with `file /tmp/cmux-screenshot.png`
- For larger screenshots, maximize the split before running the app
- `cmux send-key` supports: Enter, Escape, Tab, Up, Down, Left, Right, Backspace, Delete, Home, End, PageUp, PageDown, F1-F12
- Use `cmux resize-pane` to adjust the split size before running the app
