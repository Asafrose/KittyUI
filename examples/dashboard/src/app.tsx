/**
 * Dashboard App -- POC demonstrating KittyUI end-to-end:
 * - Flexbox layout (header, sidebar, content, footer)
 * - Keyboard navigation via useKeyboard
 * - Live counter via useState + useEffect (proves React re-rendering works)
 * - useTerminal for terminal dimensions
 */

import { useState, useEffect } from "react";
import { useTerminal, useKeyboard, KEY_UP, KEY_DOWN, Box, Text } from "@kittyui/react";

// -----------------------------------------------------------------------
// Navigation items
// -----------------------------------------------------------------------

const NAV_ITEMS = ["Overview", "Metrics", "Logs", "Settings"] as const;

// -----------------------------------------------------------------------
// Sub-components
// -----------------------------------------------------------------------

function Header() {
  return (
    <Box style={{ width: "100%", height: 3, backgroundColor: "#1e40af", padding: [1, 2] }}>
      <Text style={{ color: "#ffffff", fontWeight: "bold" }}>
        {"KittyUI Dashboard"}
      </Text>
    </Box>
  );
}

function SidebarItem({ label, active }: { label: string; active: boolean }) {
  return (
    <Box
      style={{
        height: 1,
        paddingLeft: 1,
        paddingRight: 1,
        backgroundColor: active ? "#3b82f6" : undefined,
      }}
    >
      <Text
        style={{
          color: active ? "#ffffff" : "#94a3b8",
          fontWeight: active ? "bold" : "normal",
        }}
      >
        {active ? " > " : "   "}
        {label}
      </Text>
    </Box>
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
    <Box
      style={{
        width: 22,
        flexDirection: "column",
        backgroundColor: "#1e293b",
        padding: [1, 0],
        gap: 1,
      }}
    >
      <Box style={{ paddingLeft: 1 }}>
        <Text style={{ color: "#64748b", fontWeight: "bold" }}>
          {"NAVIGATION"}
        </Text>
      </Box>
      {items.map((item: string, i: number) => (
        <SidebarItem key={item} label={item} active={i === activeIndex} />
      ))}
    </Box>
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
    <Box
      style={{
        flexGrow: 1,
        height: 5,
        flexDirection: "column",
        backgroundColor: "#1e293b",
        padding: [1, 2],
      }}
    >
      <Text style={{ color: "#94a3b8" }}>{label}</Text>
      <Text style={{ color, fontWeight: "bold" }}>{value}</Text>
    </Box>
  );
}

function OverviewContent({ uptime }: { uptime: number }) {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2] }}>
      <Text style={{ color: "#e2e8f0", fontWeight: "bold" }}>
        {"Overview"}
      </Text>

      {/* Stats row */}
      <Box style={{ flexDirection: "row", height: 5, marginTop: 1, gap: 2 }}>
        <StatsCard label="Requests" value="12,847" color="#22c55e" />
        <StatsCard label="Errors" value="23" color="#ef4444" />
        <StatsCard label="Latency" value="42ms" color="#eab308" />
        <StatsCard label="Uptime" value={`${uptime}s`} color="#3b82f6" />
      </Box>

      {/* Live counter */}
      <Box
        style={{
          height: 3,
          backgroundColor: "#1e293b",
          padding: [1, 2],
          marginTop: 1,
        }}
      >
        <Text style={{ color: "#22c55e" }}>
          {"Live uptime counter: " + String(uptime) + "s (updates every second)"}
        </Text>
      </Box>

      {/* Activity log */}
      <Box style={{ marginTop: 1 }}>
        <Text style={{ color: "#e2e8f0", fontWeight: "bold" }}>
          {"Recent Activity"}
        </Text>
      </Box>
      <Box style={{ flexDirection: "column", backgroundColor: "#1e293b", padding: 1, marginTop: 1 }}>
        <Text style={{ color: "#22c55e" }}>{"[OK]   GET  /api/health      200   2ms"}</Text>
        <Text style={{ color: "#22c55e" }}>{"[OK]   POST /api/data        201  15ms"}</Text>
        <Text style={{ color: "#eab308" }}>{"[WARN] GET  /api/metrics     200  89ms"}</Text>
        <Text style={{ color: "#ef4444" }}>{"[ERR]  GET  /api/missing     404   3ms"}</Text>
        <Text style={{ color: "#22c55e" }}>{"[OK]   GET  /api/users       200  12ms"}</Text>
      </Box>
    </Box>
  );
}

function PlaceholderContent({ title }: { title: string }) {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [2, 2] }}>
      <Text style={{ color: "#e2e8f0", fontWeight: "bold" }}>{title}</Text>
      <Box style={{ marginTop: 1 }}>
        <Text style={{ color: "#64748b" }}>
          {title + " view coming soon..."}
        </Text>
      </Box>
    </Box>
  );
}

function Footer({ cols, rows }: { cols: number; rows: number }) {
  return (
    <Box
      style={{
        width: "100%",
        height: 1,
        backgroundColor: "#1e293b",
        flexDirection: "row",
        padding: [0, 1],
      }}
    >
      <Text style={{ color: "#64748b" }}>
        {" \u2191/\u2193 navigate   q quit"}
      </Text>
      <Box style={{ flexGrow: 1 }} />
      <Text style={{ color: "#22c55e" }}>
        {"Connected " + cols + "x" + rows + " "}
      </Text>
    </Box>
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

  // Keyboard navigation
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
    <Box
      style={{
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0f172a",
      }}
    >
      <Header />
      <Box style={{ flexDirection: "row", flexGrow: 1 }}>
        <Sidebar activeIndex={activeIndex} items={NAV_ITEMS} />
        <Box style={{ flexGrow: 1, flexDirection: "column" }}>
          {content}
        </Box>
      </Box>
      <Footer cols={cols} rows={rows} />
    </Box>
  );
}
