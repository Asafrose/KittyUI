/**
 * Dashboard App -- POC demonstrating KittyUI end-to-end:
 * - Flexbox layout (header, sidebar, content, footer)
 * - Keyboard navigation via useKeyboard
 * - Live counter via useState + useEffect (proves React re-rendering works)
 * - useTerminal for terminal dimensions
 */

import { useState, useEffect } from "react";
import { useTerminal, useKeyboard, KEY_UP, KEY_DOWN } from "@kittyui/react";

// -----------------------------------------------------------------------
// Navigation items
// -----------------------------------------------------------------------

const NAV_ITEMS = ["Overview", "Metrics", "Logs", "Settings"] as const;

// -----------------------------------------------------------------------
// Sub-components
// -----------------------------------------------------------------------

function Header() {
  return (
    <box style={{ width: "100%", height: 3, backgroundColor: "#1e40af" }}>
      <box style={{ padding: [0, 2] }}>
        <text style={{ color: "#ffffff", fontWeight: "bold" }}>
          {"  KittyUI Dashboard  "}
        </text>
      </box>
    </box>
  );
}

function Sidebar({
  activeIndex,
  items,
}: {
  activeIndex: number;
  items: readonly string[];
}) {
  return (
    <box
      style={{
        width: 24,
        flexDirection: "column",
        backgroundColor: "#1e293b",
        paddingTop: 1,
      }}
    >
      <box style={{ paddingLeft: 1, paddingBottom: 1 }}>
        <text style={{ color: "#94a3b8", fontWeight: "bold" }}>
          {"  NAVIGATION"}
        </text>
      </box>
      {items.map((item: string, i: number) => (
        <box
          key={item}
          style={{
            paddingLeft: 2,
            paddingRight: 2,
            height: 1,
            backgroundColor: i === activeIndex ? "#3b82f6" : "#1e293b",
          }}
        >
          <text
            style={{
              color: i === activeIndex ? "#ffffff" : "#cbd5e1",
              fontWeight: i === activeIndex ? "bold" : "normal",
            }}
          >
            {i === activeIndex ? "> " : "  "}
            {item}
          </text>
        </box>
      ))}
    </box>
  );
}

function StatsCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}) {
  return (
    <box
      style={{
        flexGrow: 1,
        height: 5,
        flexDirection: "column",
        backgroundColor: "#1e293b",
        padding: [1, 2],
        marginRight: 1,
      }}
    >
      <text style={{ color: "#94a3b8" }}>{label}</text>
      <text style={{ color, fontWeight: "bold" }}>{value}</text>
    </box>
  );
}

function OverviewContent({ uptime }: { uptime: number }) {
  return (
    <box style={{ flexDirection: "column", flexGrow: 1, padding: 1 }}>
      <box style={{ paddingBottom: 1 }}>
        <text style={{ color: "#e2e8f0", fontWeight: "bold" }}>
          {"Overview"}
        </text>
      </box>

      {/* Stats row */}
      <box style={{ flexDirection: "row", height: 5, paddingBottom: 1 }}>
        <StatsCard label="Requests" value="12,847" color="#22c55e" />
        <StatsCard label="Errors" value="23" color="#ef4444" />
        <StatsCard label="Latency" value="42ms" color="#eab308" />
        <StatsCard label="Uptime" value={`${uptime}s`} color="#3b82f6" />
      </box>

      {/* Live counter */}
      <box
        style={{
          height: 3,
          backgroundColor: "#1e293b",
          padding: [1, 2],
          marginBottom: 1,
        }}
      >
        <text style={{ color: "#22c55e" }}>
          {"Live uptime counter: " + String(uptime) + "s (updates every second)"}
        </text>
      </box>

      {/* Activity log */}
      <box style={{ paddingBottom: 1 }}>
        <text style={{ color: "#e2e8f0", fontWeight: "bold" }}>
          {"Recent Activity"}
        </text>
      </box>
      <box style={{ flexDirection: "column", backgroundColor: "#1e293b", padding: 1 }}>
        <text style={{ color: "#22c55e" }}>{"[OK]  GET /api/health        200  2ms"}</text>
        <text style={{ color: "#22c55e" }}>{"[OK]  POST /api/data         201  15ms"}</text>
        <text style={{ color: "#eab308" }}>{"[WARN] GET /api/metrics      200  89ms"}</text>
        <text style={{ color: "#ef4444" }}>{"[ERR]  GET /api/missing      404  3ms"}</text>
        <text style={{ color: "#22c55e" }}>{"[OK]  GET /api/users         200  12ms"}</text>
      </box>
    </box>
  );
}

function PlaceholderContent({ title }: { title: string }) {
  return (
    <box style={{ flexDirection: "column", flexGrow: 1, padding: 2 }}>
      <text style={{ color: "#e2e8f0", fontWeight: "bold" }}>{title}</text>
      <box style={{ paddingTop: 1 }}>
        <text style={{ color: "#94a3b8" }}>
          {title + " view coming soon..."}
        </text>
      </box>
    </box>
  );
}

function Footer({ cols, rows }: { cols: number; rows: number }) {
  return (
    <box
      style={{
        width: "100%",
        height: 1,
        backgroundColor: "#1e293b",
        flexDirection: "row",
        justifyContent: "space-between",
        paddingLeft: 1,
        paddingRight: 1,
      }}
    >
      <text style={{ color: "#94a3b8" }}>
        {"  \u2191/\u2193 navigate   q quit"}
      </text>
      <text style={{ color: "#22c55e" }}>
        {"Connected  " + cols + "x" + rows}
      </text>
    </box>
  );
}

// -----------------------------------------------------------------------
// Main App
// -----------------------------------------------------------------------

export function App() {
  const [activeIndex, setActiveIndex] = useState(0);
  const [uptime, setUptime] = useState(0);
  const { cols, rows } = useTerminal();

  // Live counter -- increments every second to prove re-rendering works
  useEffect(() => {
    const interval = setInterval(() => {
      setUptime((prev: number) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  // Keyboard navigation — createApp sends KEY_UP/KEY_DOWN as mapped keyCodes
  useKeyboard(
    (event) => {
      const raw = event as unknown as { keyCode: number };
      const keyCode = raw.keyCode ?? 0;

      if (keyCode === KEY_UP) {
        setActiveIndex((prev: number) =>
          prev > 0 ? prev - 1 : NAV_ITEMS.length - 1,
        );
      } else if (keyCode === KEY_DOWN) {
        setActiveIndex((prev: number) =>
          prev < NAV_ITEMS.length - 1 ? prev + 1 : 0,
        );
      }
    },
    { global: true },
  );

  // Determine which content to show based on sidebar selection
  const activePage = NAV_ITEMS[activeIndex];
  let content: React.ReactNode;
  switch (activePage) {
    case "Overview":
      content = <OverviewContent uptime={uptime} />;
      break;
    case "Metrics":
      content = <PlaceholderContent title="Metrics" />;
      break;
    case "Logs":
      content = <PlaceholderContent title="Logs" />;
      break;
    case "Settings":
      content = <PlaceholderContent title="Settings" />;
      break;
  }

  return (
    <box
      style={{
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0f172a",
      }}
    >
      {/* Header */}
      <Header />

      {/* Body: sidebar + content */}
      <box style={{ flexDirection: "row", flexGrow: 1 }}>
        <Sidebar activeIndex={activeIndex} items={NAV_ITEMS} />
        <box style={{ flexGrow: 1, flexDirection: "column" }}>
          {content}
        </box>
      </box>

      {/* Footer */}
      <Footer cols={cols} rows={rows} />
    </box>
  );
}
