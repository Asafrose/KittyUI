/**
 * Dashboard App -- POC demonstrating KittyUI end-to-end:
 * - Flexbox layout (header, sidebar, content, footer)
 * - Keyboard navigation via useKeyboard
 * - Live counter via useState + useEffect (proves React re-rendering works)
 * - useTerminal for terminal dimensions
 */

import { useState, useEffect, useRef } from "react";
import { useTerminal, useKeyboard } from "@kittyui/react";

// -----------------------------------------------------------------------
// Navigation items
// -----------------------------------------------------------------------

const NAV_ITEMS = ["Overview", "Metrics", "Logs", "Settings"] as const;
type NavItem = (typeof NAV_ITEMS)[number];

// -----------------------------------------------------------------------
// Escape sequence detection for arrow keys
// Arrow up   = ESC [ A  (keyCodes: 27, 91, 65)
// Arrow down = ESC [ B  (keyCodes: 27, 91, 66)
// -----------------------------------------------------------------------

const ESC = 27;
const BRACKET = 91;
const ARROW_UP_CHAR = 65; // 'A'
const ARROW_DOWN_CHAR = 66; // 'B'

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

  // Escape sequence buffer for detecting arrow keys.
  // Arrow keys arrive as 3 sequential keyboard events: ESC, [, A/B
  const escBuf = useRef<number[]>([]);
  const escTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Live counter -- increments every second to prove re-rendering works
  useEffect(() => {
    const interval = setInterval(() => {
      setUptime((prev: number) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  // Keyboard navigation
  useKeyboard(
    (event) => {
      // The useKeyboard hook passes the raw core KeyboardEvent (with keyCode
      // directly) even though it is typed as KittyKeyboardEvent. We cast to
      // access the runtime shape.
      const raw = event as unknown as { keyCode: number };
      const keyCode = raw.keyCode ?? 0;

      const buf = escBuf.current;

      // Clear any pending escape timeout
      if (escTimer.current) {
        clearTimeout(escTimer.current);
        escTimer.current = null;
      }

      // Build up escape sequences: ESC [ A/B
      if (keyCode === ESC) {
        buf.length = 0;
        buf.push(ESC);
        escTimer.current = setTimeout(() => {
          buf.length = 0;
        }, 100);
        return;
      }

      if (buf.length === 1 && buf[0] === ESC && keyCode === BRACKET) {
        buf.push(BRACKET);
        escTimer.current = setTimeout(() => {
          buf.length = 0;
        }, 100);
        return;
      }

      if (buf.length === 2 && buf[0] === ESC && buf[1] === BRACKET) {
        buf.length = 0;
        if (keyCode === ARROW_UP_CHAR) {
          setActiveIndex((prev: number) =>
            prev > 0 ? prev - 1 : NAV_ITEMS.length - 1,
          );
          return;
        }
        if (keyCode === ARROW_DOWN_CHAR) {
          setActiveIndex((prev: number) =>
            prev < NAV_ITEMS.length - 1 ? prev + 1 : 0,
          );
          return;
        }
      }

      // Reset buffer for any non-sequence key
      buf.length = 0;
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
