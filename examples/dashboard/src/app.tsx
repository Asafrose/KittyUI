/**
 * Dashboard App -- showcasing KittyUI features:
 * - Flexbox layout (header, sidebar, content, footer)
 * - Keyboard navigation via useKeyboard
 * - Live counter via useState + useEffect
 * - Borders (round, single, double)
 * - Text overflow with ellipsis
 * - Text decoration (underline, dim)
 * - Overflow clipping
 * - Colored inline spans
 * - TextInput component
 */

import { useState, useEffect } from "react";
import { useTerminal, useKeyboard, KEY_UP, KEY_DOWN, Box, Text, TextInput } from "@kittyui/react";

// -----------------------------------------------------------------------
// Navigation items
// -----------------------------------------------------------------------

const NAV_ITEMS = ["Overview", "Metrics", "Logs", "Settings"] as const;

// -----------------------------------------------------------------------
// Activity log data
// -----------------------------------------------------------------------

const ACTIVITY_LOG = [
  { status: "OK", method: "GET", path: "/api/health", code: 200, time: "2ms" },
  { status: "OK", method: "POST", path: "/api/data", code: 201, time: "15ms" },
  { status: "WARN", method: "GET", path: "/api/metrics/dashboard/extended-report", code: 200, time: "89ms" },
  { status: "ERR", method: "GET", path: "/api/missing", code: 404, time: "3ms" },
  { status: "OK", method: "GET", path: "/api/users", code: 200, time: "12ms" },
  { status: "OK", method: "PUT", path: "/api/settings/preferences/notifications/email", code: 200, time: "34ms" },
  { status: "OK", method: "DELETE", path: "/api/cache/invalidate", code: 204, time: "8ms" },
] as const;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

function statusColor(status: string): string {
  switch (status) {
    case "OK": return "#22c55e";
    case "WARN": return "#eab308";
    case "ERR": return "#ef4444";
    default: return "#94a3b8";
  }
}

function codeColor(code: number): string {
  if (code >= 200 && code < 300) return "#22c55e";
  if (code >= 300 && code < 400) return "#eab308";
  return "#ef4444";
}

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
        {(active ? " > " : "   ") + label}
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
        width: 24,
        flexDirection: "column",
        backgroundColor: "#1e293b",
        padding: [1, 0],
        gap: 1,
        border: "single",
        borderColor: "#334155",
      }}
    >
      <Box style={{ paddingLeft: 1 }}>
        <Text style={{ color: "#64748b", fontWeight: "bold", textDecoration: "underline" }}>
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
        border: "round",
        borderColor: "#334155",
      }}
    >
      <Text style={{ color: "#94a3b8", dim: true }}>{label}</Text>
      <Text style={{ color, fontWeight: "bold" }}>{value}</Text>
    </Box>
  );
}

function ActivityLogEntry({
  status,
  method,
  path,
  code,
  time,
}: {
  status: string;
  method: string;
  path: string;
  code: number;
  time: string;
}) {
  return (
    <Box style={{ flexDirection: "row", height: 1 }}>
      <Text style={{ color: statusColor(status), fontWeight: "bold" }}>
        {`[${status}]`.padEnd(7)}
      </Text>
      <Text style={{ color: "#e2e8f0" }}>
        {` ${method.padEnd(7)}`}
      </Text>
      <Box style={{ flexGrow: 1, flexShrink: 1 }}>
        <Text style={{ color: "#94a3b8", textOverflow: "ellipsis" }}>
          {path}
        </Text>
      </Box>
      <Text style={{ color: codeColor(code), fontWeight: "bold" }}>
        {`  ${code}`}
      </Text>
      <Text style={{ color: "#64748b", dim: true }}>
        {`  ${time.padStart(5)}`}
      </Text>
    </Box>
  );
}

function OverviewContent({ uptime }: { uptime: number }) {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2] }}>
      <Text style={{ color: "#e2e8f0", fontWeight: "bold", textDecoration: "underline" }}>
        {"Overview"}
      </Text>

      {/* Stats row */}
      <Box style={{ flexDirection: "row", height: 7, marginTop: 1, gap: 1 }}>
        <StatsCard label="Requests" value="12,847" color="#22c55e" />
        <StatsCard label="Errors" value="23" color="#ef4444" />
        <StatsCard label="Latency" value="42ms" color="#eab308" />
        <StatsCard label="Uptime" value={`${uptime}s`} color="#3b82f6" />
      </Box>

      {/* Activity log */}
      <Box style={{ marginTop: 1 }}>
        <Text style={{ color: "#e2e8f0", fontWeight: "bold", textDecoration: "underline" }}>
          {"Recent Activity"}
        </Text>
      </Box>
      <Box
        style={{
          flexDirection: "column",
          backgroundColor: "#1e293b",
          padding: 1,
          marginTop: 1,
          border: "double",
          borderColor: "#334155",
          height: 9,
          overflow: "hidden",
        } as any}
      >
        {ACTIVITY_LOG.map((entry, i) => (
          <ActivityLogEntry
            key={i}
            status={entry.status}
            method={entry.method}
            path={entry.path}
            code={entry.code}
            time={entry.time}
          />
        ))}
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
          {"Live uptime: "}
        </Text>
        <Text style={{ color: "#22c55e", fontWeight: "bold" }}>
          {String(uptime) + "s"}
        </Text>
        <Text style={{ color: "#64748b", dim: true }}>
          {" (updates every second)"}
        </Text>
      </Box>
    </Box>
  );
}

function SettingsContent() {
  const [search, setSearch] = useState("");

  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [2, 2] }}>
      <Text style={{ color: "#e2e8f0", fontWeight: "bold", textDecoration: "underline" }}>
        {"Settings"}
      </Text>
      <Box style={{ marginTop: 1, flexDirection: "column", gap: 1 }}>
        <Text style={{ color: "#94a3b8" }}>{"Search settings:"}</Text>
        <TextInput
          value={search}
          onChange={setSearch}
          placeholder="Search settings..."
          style={{ backgroundColor: "#1e293b", color: "#e2e8f0", padding: [0, 1] }}
        />
      </Box>
      <Box style={{ marginTop: 1 }}>
        <Text style={{ color: "#64748b", dim: true }}>
          {search ? `Filtering for: "${search}"` : "Type to filter settings..."}
        </Text>
      </Box>
    </Box>
  );
}

function PlaceholderContent({ title }: { title: string }) {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [2, 2] }}>
      <Text style={{ color: "#e2e8f0", fontWeight: "bold", textDecoration: "underline" }}>{title}</Text>
      <Box style={{ marginTop: 1 }}>
        <Text style={{ color: "#64748b", dim: true }}>
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
        {"Connected "}
      </Text>
      <Text style={{ color: "#22c55e", dim: true }}>
        {cols + "x" + rows + " "}
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
      content = <SettingsContent />;
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
