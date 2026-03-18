/**
 * KittyUI Demo — entry point.
 *
 * Builds a dashboard layout using the imperative core API directly,
 * bypassing the React reconciler (whose host config does not yet
 * support nested appendChild).
 */

import {
  Bridge,
  MutationEncoder,
  RenderableTree,
  BoxRenderable,
  TextRenderable,
} from "@kittyui/core";

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

const bridge = new Bridge();

let nativeReady = false;

if (bridge.nativeAvailable) {
  const info = bridge.init();
  console.log(
    `KittyUI native engine v${info.versionMajor}.${info.versionMinor}.${info.versionPatch}` +
      ` (batched FFI: ${info.batchedFfi})`,
  );
  nativeReady = true;
} else {
  console.log("Native library not found — running in tree-only mode.");
  console.log("Build the native library with: bun run build:native");
}

// ---------------------------------------------------------------------------
// Create the rendering pipeline
// ---------------------------------------------------------------------------

const encoder = nativeReady ? bridge.getEncoder() : new MutationEncoder();
const tree = new RenderableTree(encoder);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const addBox = (
  parentId: number,
  style: Parameters<BoxRenderable["setBoxStyle"]>[0],
): BoxRenderable => {
  const box = BoxRenderable.create(style);
  tree.appendChild(parentId, box);
  return box;
};

const addText = (
  parentId: number,
  content: string,
  style?: Parameters<TextRenderable["setStyle"]>[0],
): TextRenderable => {
  const text = new TextRenderable({ text: content });
  if (style) {
    text.setStyle(style);
  }
  tree.appendChild(parentId, text);
  return text;
};

// ---------------------------------------------------------------------------
// Build the dashboard layout
// ---------------------------------------------------------------------------

// Root container — full screen, column layout
const root = BoxRenderable.create({
  flexDirection: "column",
  width: "100%",
  height: "100%",
});
tree.setRoot(root);

// -- Header ----------------------------------------------------------------

const header = addBox(root.nodeId, {
  backgroundColor: "#1e40af",
  flexDirection: "row",
  justifyContent: "center",
  alignItems: "center",
  width: "100%",
  height: 3,
  padding: [0, 2],
});

addText(header.nodeId, "KittyUI Dashboard", {
  fontWeight: "bold",
  color: "#ffffff",
});

// -- Body (sidebar + main) -------------------------------------------------

const body = addBox(root.nodeId, {
  flexDirection: "row",
  flexGrow: 1,
});

// Sidebar
const sidebar = addBox(body.nodeId, {
  backgroundColor: "#0f172a",
  flexDirection: "column",
  width: 24,
  padding: [1, 0],
  gap: 1,
});

addText(sidebar.nodeId, "NAVIGATION", {
  color: "#64748b",
  fontWeight: "bold",
  padding: [0, 1],
});

const navItems = [
  { label: "Overview", active: true },
  { label: "Metrics", active: false },
  { label: "Logs", active: false },
  { label: "Settings", active: false },
];

for (const item of navItems) {
  const navBox = addBox(sidebar.nodeId, {
    backgroundColor: item.active ? "#3b82f6" : undefined,
    padding: [0, 1],
    height: 1,
    width: "100%",
  });
  const prefix = item.active ? "> " : "  ";
  addText(navBox.nodeId, `${prefix}${item.label}`, {
    color: item.active ? "#ffffff" : "#94a3b8",
  });
}

// Main content area
const main = addBox(body.nodeId, {
  flexDirection: "column",
  flexGrow: 1,
  gap: 1,
});

// -- Stats row -------------------------------------------------------------

const statsRow = addBox(main.nodeId, {
  flexDirection: "row",
  width: "100%",
  gap: 2,
});

const stats = [
  { title: "Requests", value: "12,847", color: "#22c55e" },
  { title: "Errors", value: "23", color: "#ef4444" },
  { title: "Latency", value: "42ms", color: "#eab308" },
  { title: "Uptime", value: "99.9%", color: "#3b82f6" },
];

for (const stat of stats) {
  const card = addBox(statsRow.nodeId, {
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    flexGrow: 1,
    height: 5,
    padding: 1,
  });
  addText(card.nodeId, stat.value, {
    color: stat.color,
    fontWeight: "bold",
  });
  addText(card.nodeId, stat.title, { color: "#94a3b8" });
}

// -- Activity log ----------------------------------------------------------

const logPanel = addBox(main.nodeId, {
  flexDirection: "column",
  flexGrow: 1,
  padding: 1,
});

addText(logPanel.nodeId, "RECENT ACTIVITY", {
  color: "#64748b",
  fontWeight: "bold",
});

const logEntries = [
  { time: "14:32:01", msg: "Deployment completed successfully", level: "info" },
  { time: "14:31:58", msg: "Health check passed", level: "info" },
  { time: "14:31:45", msg: "High memory usage on node-3", level: "warn" },
  { time: "14:31:30", msg: "Connection timeout to db-replica", level: "error" },
  { time: "14:31:12", msg: "Cache invalidation triggered", level: "info" },
  { time: "14:30:55", msg: "Slow query detected (>500ms)", level: "warn" },
];

const logContainer = addBox(logPanel.nodeId, {
  flexDirection: "column",
  marginTop: 1,
});

for (const entry of logEntries) {
  const levelColor =
    entry.level === "error" ? "#ef4444" : entry.level === "warn" ? "#eab308" : "#64748b";
  const tag = entry.level === "error" ? "ERR" : entry.level === "warn" ? "WRN" : "INF";

  const row = addBox(logContainer.nodeId, {
    flexDirection: "row",
    gap: 1,
    height: 1,
  });
  addText(row.nodeId, entry.time, { color: "#475569" });
  addText(row.nodeId, `[${tag}]`, { color: levelColor, fontWeight: "bold" });
  addText(row.nodeId, entry.msg, { color: "#cbd5e1" });
}

// -- Footer ----------------------------------------------------------------

const footer = addBox(root.nodeId, {
  backgroundColor: "#1e293b",
  flexDirection: "row",
  justifyContent: "space-between",
  width: "100%",
  height: 1,
  padding: [0, 1],
});

addText(footer.nodeId, "KittyUI v0.1.0", { color: "#64748b" });
addText(footer.nodeId, "Connected", { color: "#22c55e" });

// ---------------------------------------------------------------------------
// Flush and render
// ---------------------------------------------------------------------------

console.log(`Tree built: ${tree.size} nodes`);

if (nativeReady) {
  tree.flushDirtyStyles();
  bridge.flushMutations();
  bridge.startRenderLoop();

  process.on("SIGINT", () => {
    bridge.stopRenderLoop();
    bridge.shutdown();
    process.exit(0);
  });

  console.log("Demo running — press Ctrl+C to exit.");
} else {
  console.log("Tree mounted successfully (no rendering without native engine).");
}
