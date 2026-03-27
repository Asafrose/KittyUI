/**
 * KittyUI Dashboard — a polished terminal UI that looks like a web app.
 *
 * Showcases: borders, text-overflow/ellipsis, text-decoration, dim,
 * overflow:hidden, colored inline spans, TextInput, flexbox layout,
 * keyboard navigation, live state updates, simulated box-shadow.
 */

import { useState, useEffect } from "react";
import {
  useTerminal,
  useKeyboard,
  KEY_UP,
  KEY_DOWN,
  Box,
  Text,
  TextInput,
} from "@kittyui/react";

// ── Data ──────────────────────────────────────────────────────────────

const NAV = [
  { label: "Overview", icon: "\u25C8" },   // ◈
  { label: "Services", icon: "\u25A0" },   // ■
  { label: "Logs",     icon: "\u25B6" },   // ▶
  { label: "Settings", icon: "\u2699" },   // ⚙
] as const;

const SERVICES = [
  { name: "api-gateway",      status: "healthy",  latency: 12,  uptime: 99.98 },
  { name: "auth-service",     status: "healthy",  latency: 8,   uptime: 99.99 },
  { name: "payment-worker",   status: "degraded", latency: 142, uptime: 98.40 },
  { name: "search-index",     status: "healthy",  latency: 34,  uptime: 99.95 },
  { name: "notification-svc", status: "down",     latency: 0,   uptime: 94.20 },
  { name: "cdn-proxy",        status: "healthy",  latency: 3,   uptime: 100.0 },
] as const;

const LOG_ENTRIES = [
  { ts: "12:04:31", level: "INFO",  svc: "api-gateway",      msg: "GET /api/v2/users 200 in 12ms" },
  { ts: "12:04:30", level: "INFO",  svc: "auth-service",     msg: "Token refresh for user_8a3f completed" },
  { ts: "12:04:29", level: "WARN",  svc: "payment-worker",   msg: "Stripe webhook retry #3 for evt_1N2x \u2014 upstream 503" },
  { ts: "12:04:28", level: "ERROR", svc: "notification-svc", msg: "Connection refused: smtp.provider.io:587 \u2014 circuit open" },
  { ts: "12:04:27", level: "INFO",  svc: "search-index",     msg: "Reindex batch 847/1200 committed (34ms)" },
  { ts: "12:04:26", level: "INFO",  svc: "cdn-proxy",        msg: "Cache HIT ratio 97.3% \u2014 12.4k req/s" },
  { ts: "12:04:25", level: "WARN",  svc: "payment-worker",   msg: "Queue depth 847 \u2014 approaching backpressure threshold" },
  { ts: "12:04:24", level: "INFO",  svc: "api-gateway",      msg: "POST /api/v2/orders 201 in 89ms" },
  { ts: "12:04:23", level: "ERROR", svc: "notification-svc", msg: "Failed to deliver email batch #4201 \u2014 23 bounced" },
  { ts: "12:04:22", level: "INFO",  svc: "auth-service",     msg: "OAuth2 callback processed for provider github" },
] as const;

// ── Palette ───────────────────────────────────────────────────────────

const C = {
  // Background layers (darkest → lightest)
  bg:          "#0f172a",  // page background
  shadow:      "#0a0f1e",  // card shadow (darker than bg)
  surface:     "#1e293b",  // card surface
  surfaceHi:   "#283548",  // card hover / raised surface
  borderCard:  "#3b4a63",  // card border (visible against surface)
  borderDim:   "#2d3a4f",  // subtle border
  // Text
  text:        "#f1f5f9",
  textSec:     "#cbd5e1",
  textDim:     "#94a3b8",
  textMuted:   "#64748b",
  // Accent
  accent:      "#3b82f6",
  accentLight: "#60a5fa",
  accentBg:    "#1e3a6e",
  // Semantic
  green:       "#4ade80",
  greenDim:    "#166534",
  yellow:      "#facc15",
  yellowDim:   "#854d0e",
  red:         "#f87171",
  redDim:      "#991b1b",
  cyan:        "#22d3ee",
  purple:      "#c4b5fd",
} as const;

// ── Helpers ───────────────────────────────────────────────────────────

function bar(pct: number, width: number): string {
  const filled = Math.round((pct / 100) * width);
  return "\u2588".repeat(filled) + "\u2591".repeat(width - filled);
}

function sparkline(values: number[]): string {
  const ticks = ["\u2581","\u2582","\u2583","\u2584","\u2585","\u2586","\u2587","\u2588"];
  const mn = Math.min(...values);
  const mx = Math.max(...values) || 1;
  return values.map(v => ticks[Math.round(((v - mn) / (mx - mn)) * 7)]).join("");
}

function levelColor(level: string): string {
  if (level === "ERROR") return C.red;
  if (level === "WARN")  return C.yellow;
  return C.textDim;
}

function statusBadge(s: string): { label: string; fg: string; bg: string } {
  if (s === "healthy")  return { label: " \u2714 healthy  ", fg: "#dcfce7", bg: C.greenDim };
  if (s === "degraded") return { label: " \u26A0 degraded ", fg: "#fef9c3", bg: C.yellowDim };
  return                       { label: " \u2716 down     ", fg: "#fecaca", bg: C.redDim };
}

// ── Card with shadow ──────────────────────────────────────────────────
// Simulates a CSS box-shadow by rendering a darker box offset by 1 char
// behind the main card. The card uses round borders for that modern feel.

function Card({ children, height, style }: {
  children: React.ReactNode;
  height?: number;
  style?: Record<string, any>;
}) {
  const cardHeight = height ?? 0;
  return (
    <Box style={{
      flexDirection: "column", flexGrow: style?.flexGrow ?? 0,
      ...(style?.flexGrow ? {} : {}),
    }}>
      {/* Main card */}
      <Box style={{
        flexDirection: "column",
        backgroundColor: C.surface,
        border: "round",
        borderColor: C.borderCard,
        padding: [1, 2],
        ...(cardHeight ? { height: cardHeight } : {}),
        ...style,
        // Override border to always be round
        ...(style?.border ? {} : { border: "round" as const }),
      } as any}>
        {children}
      </Box>
      {/* Shadow: a thin dark bar below the card, offset right by 1 */}
      <Box style={{
        height: 1,
        marginLeft: 1,
        marginRight: 0,
        backgroundColor: C.shadow,
      }} />
    </Box>
  );
}

// ── Components ────────────────────────────────────────────────────────

function Header() {
  return (
    <Box style={{
      width: "100%", height: 3, backgroundColor: C.accentBg,
      flexDirection: "row", padding: [1, 2],
      border: "single", borderColor: C.accent,
    }}>
      <Text style={{ color: C.accentLight, fontWeight: "bold" }}>
        {"\u25C6 "}
      </Text>
      <Text style={{ color: "#ffffff", fontWeight: "bold" }}>
        {"KittyUI"}
      </Text>
      <Text style={{ color: C.accentLight }}>
        {"  Dashboard"}
      </Text>
      <Box style={{ flexGrow: 1 }} />
      <Text style={{ color: C.accentLight, dim: true } as any}>
        {"\u25CF connected"}
      </Text>
    </Box>
  );
}

function SidebarItem({ icon, label, active }: { icon: string; label: string; active: boolean }) {
  return (
    <Box style={{
      height: 1, paddingLeft: 1, paddingRight: 1,
      backgroundColor: active ? C.accent : undefined,
    }}>
      <Text style={{
        color: active ? "#ffffff" : C.textMuted,
        fontWeight: active ? "bold" : "normal",
      }}>
        {active ? "\u2590 " : "  "}
        {icon + "  " + label}
      </Text>
    </Box>
  );
}

function Sidebar({ activeIndex }: { activeIndex: number }) {
  return (
    <Box style={{
      width: 24, flexDirection: "column", backgroundColor: C.surface,
      paddingTop: 1, paddingBottom: 1,
      border: "round", borderColor: C.borderCard,
    }}>
      <Box style={{ paddingLeft: 2, paddingBottom: 1 }}>
        <Text style={{ color: C.textMuted, fontWeight: "bold" }}>
          {"MENU"}
        </Text>
      </Box>
      {NAV.map((item, i) => (
        <SidebarItem key={item.label} icon={item.icon} label={item.label} active={i === activeIndex} />
      ))}
      <Box style={{ flexGrow: 1 }} />
      <Box style={{ paddingLeft: 2 }}>
        <Text style={{ color: C.textMuted, dim: true } as any}>
          {"\u2191\u2193 navigate"}
        </Text>
      </Box>
    </Box>
  );
}

// ── Stat Card ─────────────────────────────────────────────────────────

function StatCard({
  label, value, sub, color, spark,
}: {
  label: string; value: string; sub: string; color: string; spark?: number[];
}) {
  return (
    <Card height={8} style={{ flexGrow: 1 }}>
      <Text style={{ color: C.textMuted, dim: true } as any}>{label}</Text>
      <Box style={{ flexDirection: "row", marginTop: 1 }}>
        <Text style={{ color, fontWeight: "bold" }}>{value}</Text>
        <Text style={{ color: C.textDim, dim: true } as any}>{"  " + sub}</Text>
      </Box>
      {spark ? (
        <Box style={{ marginTop: 1 }}>
          <Text style={{ color } as any}>
            {sparkline(spark)}
          </Text>
        </Box>
      ) : null}
    </Card>
  );
}

// ── Activity Log ──────────────────────────────────────────────────────

function LogLine({ ts, level, svc, msg }: { ts: string; level: string; svc: string; msg: string }) {
  return (
    <Box style={{ flexDirection: "row", height: 1 }}>
      <Text style={{ color: C.textMuted, dim: true } as any}>
        {ts + "  "}
      </Text>
      <Text style={{ color: levelColor(level), fontWeight: level !== "INFO" ? "bold" : "normal" }}>
        {level.padEnd(6)}
      </Text>
      <Text style={{ color: C.purple }}>
        {svc.padEnd(18)}
      </Text>
      <Box style={{ flexGrow: 1, flexShrink: 1 }}>
        <Text style={{ color: C.textSec, textOverflow: "ellipsis" }}>
          {msg}
        </Text>
      </Box>
    </Box>
  );
}

// ── Overview Page ─────────────────────────────────────────────────────

function OverviewPage({ uptime }: { uptime: number }) {
  const reqSpark = [42, 38, 55, 47, 63, 58, 71, 65, 80, 74, 68, 85];
  const errSpark = [1, 0, 2, 1, 3, 0, 1, 2, 0, 1, 0, 2];
  const latSpark = [12, 15, 11, 18, 14, 13, 22, 16, 12, 19, 14, 12];

  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2], gap: 1 }}>
      <Text style={{ color: C.text, fontWeight: "bold" }}>
        {"\u25C8  Overview"}
      </Text>

      {/* Stats row */}
      <Box style={{ flexDirection: "row", gap: 1 }}>
        <StatCard label="REQUESTS/S" value="12,847" sub="\u2191 8.3%" color={C.green} spark={reqSpark} />
        <StatCard label="ERRORS" value="23" sub="\u2193 0.18%" color={C.red} spark={errSpark} />
        <StatCard label="P95 LATENCY" value="42ms" sub="\u2193 12%" color={C.yellow} spark={latSpark} />
        <StatCard label="UPTIME" value={`${uptime}s`} sub="\u25CF live" color={C.cyan} />
      </Box>

      {/* Activity log */}
      <Box style={{ flexDirection: "row", marginTop: 1 }}>
        <Text style={{ color: C.text, fontWeight: "bold" }}>
          {"\u25B6  Recent Activity"}
        </Text>
        <Box style={{ flexGrow: 1 }} />
        <Text style={{ color: C.textMuted, dim: true } as any}>
          {"8 of 1,204"}
        </Text>
      </Box>

      <Card style={{ flexGrow: 1, overflow: "hidden" }}>
        {/* Column headers */}
        <Box style={{ flexDirection: "row", height: 1, marginBottom: 1 }}>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"TIME      LEVEL "}
          </Text>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"SERVICE           "}
          </Text>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"MESSAGE"}
          </Text>
        </Box>
        {LOG_ENTRIES.slice(0, 8).map((e, i) => (
          <LogLine key={i} ts={e.ts} level={e.level} svc={e.svc} msg={e.msg} />
        ))}
      </Card>
    </Box>
  );
}

// ── Services Page ─────────────────────────────────────────────────────

function ServiceRow({ name, status, latency, uptimePct }: {
  name: string; status: string; latency: number; uptimePct: number;
}) {
  const badge = statusBadge(status);
  const barColor = uptimePct >= 99.9 ? C.green : uptimePct >= 98 ? C.yellow : C.red;

  return (
    <Box style={{ flexDirection: "row", height: 1 }}>
      <Box style={{ width: 20 }}>
        <Text style={{ color: C.text }}>{name}</Text>
      </Box>
      <Box style={{ width: 14 }}>
        <Text style={{ color: badge.fg, backgroundColor: badge.bg, fontWeight: "bold" }}>
          {badge.label}
        </Text>
      </Box>
      <Box style={{ width: 10 }}>
        <Text style={{ color: latency > 100 ? C.yellow : C.textDim }}>
          {latency > 0 ? `${latency}ms` : "  \u2014"}
        </Text>
      </Box>
      <Box style={{ width: 18 }}>
        <Text style={{ color: barColor }}>
          {bar(uptimePct, 8)}
        </Text>
        <Text style={{ color: C.textDim, dim: true } as any}>
          {` ${uptimePct.toFixed(1)}%`}
        </Text>
      </Box>
    </Box>
  );
}

function ServicesPage() {
  const healthy = SERVICES.filter(s => s.status === "healthy").length;
  const total = SERVICES.length;

  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2], gap: 1 }}>
      <Box style={{ flexDirection: "row" }}>
        <Text style={{ color: C.text, fontWeight: "bold" }}>
          {"\u25A0  Services"}
        </Text>
        <Box style={{ flexGrow: 1 }} />
        <Text style={{ color: C.green, fontWeight: "bold" }}>{`${healthy}`}</Text>
        <Text style={{ color: C.textMuted }}>{"/" + total + " healthy"}</Text>
      </Box>

      <Card>
        {/* Column headers */}
        <Box style={{ flexDirection: "row", height: 1, marginBottom: 1 }}>
          <Box style={{ width: 20 }}>
            <Text style={{ color: C.textMuted, textDecoration: "underline" }}>{"SERVICE"}</Text>
          </Box>
          <Box style={{ width: 14 }}>
            <Text style={{ color: C.textMuted, textDecoration: "underline" }}>{"STATUS"}</Text>
          </Box>
          <Box style={{ width: 10 }}>
            <Text style={{ color: C.textMuted, textDecoration: "underline" }}>{"LATENCY"}</Text>
          </Box>
          <Box style={{ width: 18 }}>
            <Text style={{ color: C.textMuted, textDecoration: "underline" }}>{"UPTIME"}</Text>
          </Box>
        </Box>
        {SERVICES.map(s => (
          <ServiceRow
            key={s.name}
            name={s.name}
            status={s.status}
            latency={s.latency}
            uptimePct={s.uptime}
          />
        ))}
      </Card>
    </Box>
  );
}

// ── Logs Page ─────────────────────────────────────────────────────────

function LogsPage() {
  const [filter, setFilter] = useState("");
  const filtered = filter
    ? LOG_ENTRIES.filter(e =>
        e.msg.toLowerCase().includes(filter.toLowerCase()) ||
        e.svc.includes(filter) ||
        e.level.toLowerCase().includes(filter.toLowerCase()))
    : LOG_ENTRIES;

  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2], gap: 1 }}>
      <Text style={{ color: C.text, fontWeight: "bold" }}>
        {"\u25B6  Logs"}
      </Text>

      {/* Search bar */}
      <Card height={3}>
        <Box style={{ flexDirection: "row" }}>
          <Text style={{ color: C.textMuted }}>
            {"\u2315 "}
          </Text>
          <TextInput
            value={filter}
            onChange={setFilter}
            placeholder="Filter by service, level, or message..."
            style={{ flexGrow: 1, backgroundColor: C.surface, color: C.text }}
          />
        </Box>
      </Card>

      <Card style={{ flexGrow: 1, overflow: "hidden" }}>
        <Box style={{ flexDirection: "row", height: 1, marginBottom: 1 }}>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"TIME      LEVEL "}
          </Text>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"SERVICE           "}
          </Text>
          <Text style={{ color: C.textMuted, textDecoration: "underline" }}>
            {"MESSAGE"}
          </Text>
        </Box>
        {filtered.map((e, i) => (
          <LogLine key={i} ts={e.ts} level={e.level} svc={e.svc} msg={e.msg} />
        ))}
        {filtered.length === 0 ? (
          <Box style={{ padding: [1, 0] }}>
            <Text style={{ color: C.textMuted, dim: true } as any}>
              {"No logs match your filter."}
            </Text>
          </Box>
        ) : null}
      </Card>
    </Box>
  );
}

// ── Settings Page ─────────────────────────────────────────────────────

function SettingsRow({ label, value }: { label: string; value: string }) {
  return (
    <Box style={{ flexDirection: "row", height: 1 }}>
      <Box style={{ width: 24 }}>
        <Text style={{ color: C.textDim }}>{label}</Text>
      </Box>
      <Text style={{ color: C.text }}>{value}</Text>
    </Box>
  );
}

function SettingsPage() {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, padding: [1, 2], gap: 1 }}>
      <Text style={{ color: C.text, fontWeight: "bold" }}>
        {"\u2699  Settings"}
      </Text>

      <Card>
        <Text style={{ color: C.textMuted, textDecoration: "underline", marginBottom: 1 } as any}>
          {"GENERAL"}
        </Text>
        <SettingsRow label="Refresh interval" value="30s" />
        <SettingsRow label="Log retention" value="7 days" />
        <SettingsRow label="Alert threshold" value="95% uptime" />
        <SettingsRow label="Timezone" value="UTC" />
        <SettingsRow label="Theme" value="Dark" />
      </Card>

      <Card>
        <Text style={{ color: C.textMuted, textDecoration: "underline", marginBottom: 1 } as any}>
          {"NOTIFICATIONS"}
        </Text>
        <SettingsRow label="Email alerts" value="\u2714 Enabled" />
        <SettingsRow label="Slack webhook" value="\u2714 Connected" />
        <SettingsRow label="PagerDuty" value="\u2716 Not configured" />
      </Card>
    </Box>
  );
}

// ── Footer ────────────────────────────────────────────────────────────

function Footer({ cols, rows }: { cols: number; rows: number }) {
  return (
    <Box style={{
      width: "100%", height: 1, backgroundColor: C.surface,
      flexDirection: "row", padding: [0, 2],
    }}>
      <Text style={{ color: C.textMuted }}>
        {"\u2191\u2193 navigate"}
      </Text>
      <Text style={{ color: C.borderDim }}>{"  \u2502  "}</Text>
      <Text style={{ color: C.textMuted }}>
        {"q quit"}
      </Text>
      <Box style={{ flexGrow: 1 }} />
      <Text style={{ color: C.green }}>{"\u25CF "}</Text>
      <Text style={{ color: C.textDim, dim: true } as any}>
        {cols + "\u00D7" + rows}
      </Text>
    </Box>
  );
}

// ── App ───────────────────────────────────────────────────────────────

export function App() {
  const [page, setPage] = useState(0);
  const [uptime, setUptime] = useState(0);
  const { cols, rows } = useTerminal();

  useEffect(() => {
    const id = setInterval(() => setUptime((p: number) => p + 1), 1000);
    return () => clearInterval(id);
  }, []);

  useKeyboard(
    (event) => {
      const kc = (event as unknown as { keyCode: number }).keyCode ?? 0;
      if (kc === KEY_UP)   setPage((p: number) => (p > 0 ? p - 1 : NAV.length - 1));
      if (kc === KEY_DOWN) setPage((p: number) => (p < NAV.length - 1 ? p + 1 : 0));
    },
    { global: true },
  );

  let content: React.ReactNode;
  switch (page) {
    case 0:  content = <OverviewPage uptime={uptime} />; break;
    case 1:  content = <ServicesPage />; break;
    case 2:  content = <LogsPage />; break;
    default: content = <SettingsPage />; break;
  }

  return (
    <Box style={{
      flexDirection: "column", width: "100%", height: "100%",
      backgroundColor: C.bg,
    }}>
      <Header />
      <Box style={{ flexDirection: "row", flexGrow: 1 }}>
        <Sidebar activeIndex={page} />
        <Box style={{ flexGrow: 1, flexDirection: "column" }}>
          {content}
        </Box>
      </Box>
      <Footer cols={cols} rows={rows} />
    </Box>
  );
}
