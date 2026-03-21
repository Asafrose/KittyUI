/**
 * KittyUI Demo — a mini dashboard layout showcasing boxes, text, colors,
 * borders, and nested flexbox layouts with keyboard navigation.
 */

import React, { useState } from "react";

// ---------------------------------------------------------------------------
// Header bar
// ---------------------------------------------------------------------------

const Header = (): React.JSX.Element => (
  <box
    style={{
      backgroundColor: "#1e40af",
      color: "#ffffff",
      flexDirection: "row",
      justifyContent: "center",
      alignItems: "center",
      width: "100%",
      height: 3,
      padding: [0, 2],
    }}
  >
    <text style={{ fontWeight: "bold", color: "#ffffff" }}>
      KittyUI Dashboard
    </text>
  </box>
);

// ---------------------------------------------------------------------------
// Sidebar navigation
// ---------------------------------------------------------------------------

const SIDEBAR_ITEMS = ["Overview", "Metrics", "Logs", "Settings"];

const SidebarItem = ({ label, active }: { label: string; active?: boolean }): React.JSX.Element => (
  <box
    style={{
      backgroundColor: active ? "#3b82f6" : undefined,
      padding: [0, 1],
      height: 1,
      width: "100%",
    }}
  >
    <text style={{ color: active ? "#ffffff" : "#94a3b8" }}>
      {active ? "> " : "  "}{label}
    </text>
  </box>
);

const Sidebar = ({ activeIndex }: { activeIndex: number }): React.JSX.Element => (
  <box
    style={{
      backgroundColor: "#0f172a",
      flexDirection: "column",
      width: 24,
      padding: [1, 0],
      gap: 1,
    }}
  >
    <text style={{ color: "#64748b", fontWeight: "bold", padding: [0, 1] }}>
      NAVIGATION
    </text>
    {SIDEBAR_ITEMS.map((label, i) => (
      <SidebarItem key={label} label={label} active={i === activeIndex} />
    ))}
  </box>
);

// ---------------------------------------------------------------------------
// Stat cards
// ---------------------------------------------------------------------------

const StatCard = ({
  title,
  value,
  color,
}: {
  title: string;
  value: string;
  color: string;
}): React.JSX.Element => (
  <box
    style={{
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      flexGrow: 1,
      height: 5,
      padding: 1,
    }}
  >
    <text style={{ color, fontWeight: "bold" }}>{value}</text>
    <text style={{ color: "#94a3b8" }}>{title}</text>
  </box>
);

const StatsRow = (): React.JSX.Element => (
  <box style={{ flexDirection: "row", width: "100%", gap: 2 }}>
    <StatCard title="Requests" value="12,847" color="#22c55e" />
    <StatCard title="Errors" value="23" color="#ef4444" />
    <StatCard title="Latency" value="42ms" color="#eab308" />
    <StatCard title="Uptime" value="99.9%" color="#3b82f6" />
  </box>
);

// ---------------------------------------------------------------------------
// Activity log
// ---------------------------------------------------------------------------

const LogEntry = ({
  time,
  message,
  level,
}: {
  time: string;
  message: string;
  level: "info" | "warn" | "error";
}): React.JSX.Element => {
  const levelColor = level === "error" ? "#ef4444" : level === "warn" ? "#eab308" : "#64748b";
  const tag = level === "error" ? "ERR" : level === "warn" ? "WRN" : "INF";

  return (
    <box style={{ flexDirection: "row", gap: 1, height: 1 }}>
      <text style={{ color: "#475569" }}>{time}</text>
      <text style={{ color: levelColor, fontWeight: "bold" }}>[{tag}]</text>
      <text style={{ color: "#cbd5e1" }}>{message}</text>
    </box>
  );
};

const ActivityLog = (): React.JSX.Element => (
  <box style={{ flexDirection: "column", flexGrow: 1, padding: 1 }}>
    <text style={{ color: "#64748b", fontWeight: "bold" }}>RECENT ACTIVITY</text>
    <box style={{ flexDirection: "column", marginTop: 1 }}>
      <LogEntry time="14:32:01" message="Deployment completed successfully" level="info" />
      <LogEntry time="14:31:58" message="Health check passed" level="info" />
      <LogEntry time="14:31:45" message="High memory usage on node-3" level="warn" />
      <LogEntry time="14:31:30" message="Connection timeout to db-replica" level="error" />
      <LogEntry time="14:31:12" message="Cache invalidation triggered" level="info" />
      <LogEntry time="14:30:55" message="Slow query detected (>500ms)" level="warn" />
    </box>
  </box>
);

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

const Footer = ({ hint }: { hint: string }): React.JSX.Element => (
  <box
    style={{
      backgroundColor: "#1e293b",
      flexDirection: "row",
      justifyContent: "space-between",
      width: "100%",
      height: 1,
      padding: [0, 1],
    }}
  >
    <text style={{ color: "#64748b" }}>{hint}</text>
    <text style={{ color: "#22c55e" }}>Connected</text>
  </box>
);

// ---------------------------------------------------------------------------
// Main content
// ---------------------------------------------------------------------------

const MainContent = (): React.JSX.Element => (
  <box style={{ flexDirection: "column", flexGrow: 1, gap: 1 }}>
    <StatsRow />
    <ActivityLog />
  </box>
);

// ---------------------------------------------------------------------------
// App root — stateful, receives keyboard events via props
// ---------------------------------------------------------------------------

export interface AppProps {
  activeIndex?: number;
}

export const App = ({ activeIndex = 0 }: AppProps): React.JSX.Element => (
  <box style={{ flexDirection: "column", width: "100%", height: "100%" }}>
    <Header />
    <box style={{ flexDirection: "row", flexGrow: 1 }}>
      <Sidebar activeIndex={activeIndex} />
      <MainContent />
    </box>
    <Footer hint="↑/↓ navigate  q quit" />
  </box>
);
