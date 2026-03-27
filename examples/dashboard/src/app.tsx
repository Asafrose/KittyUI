/**
 * shadcn/ui-inspired Dashboard -- KittyUI showcase demo.
 *
 * Features:
 * - Pixel-rendered rounded corners, box shadows, gradients
 * - Keyboard tab navigation (left/right arrows)
 * - Live uptime counter
 * - shadcn dark theme (zinc palette)
 * - Unicode sparkline charts
 * - Responsive flexbox layout
 */

import { useState, useEffect } from "react";
import {
  useTerminal,
  useKeyboard,
  KEY_LEFT,
  KEY_RIGHT,
  Box,
  Text,
} from "@kittyui/react";
import type { CSSStyle } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Design tokens (shadcn dark / zinc)
// ---------------------------------------------------------------------------

const t = {
  bg: "#09090b",
  card: "#18181b",
  cardFg: "#fafafa",
  muted: "#27272a",
  mutedFg: "#a1a1aa",
  border: "#27272a",
  primary: "#fafafa",
  primaryFg: "#18181b",
  accent: "#27272a",
  destructive: "#ef4444",
  success: "#22c55e",
  warning: "#eab308",
  blue: "#3b82f6",
  violet: "#8b5cf6",
} as const;

// ---------------------------------------------------------------------------
// Reusable card style
// ---------------------------------------------------------------------------

const cardStyle: CSSStyle = {
  backgroundColor: t.card,
  borderRadius: 8,
  boxShadow: "0 1px 3px rgba(0,0,0,0.3)",
  border: "round",
  borderColor: t.border,
  padding: [1, 2],
  flexDirection: "column",
};

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

const TABS = ["Overview", "Analytics", "Reports", "Settings"] as const;
type Tab = (typeof TABS)[number];

function HeaderTabs({
  active,
  onSelect,
}: {
  active: Tab;
  onSelect: (tab: Tab) => void;
}) {
  return (
    <Box style={{ flexDirection: "row", gap: 1 }}>
      {TABS.map((tab) => {
        const isActive = tab === active;
        return (
          <Box
            key={tab}
            style={{
              paddingLeft: 1,
              paddingRight: 1,
              height: 1,
              backgroundColor: isActive ? t.accent : undefined,
              borderRadius: isActive ? 4 : 0,
            }}
          >
            <Text
              style={{
                color: isActive ? t.primary : t.mutedFg,
                fontWeight: isActive ? "bold" : "normal",
              }}
            >
              {tab}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}

function Header({ active, onSelect }: { active: Tab; onSelect: (t: Tab) => void }) {
  return (
    <Box
      style={{
        width: "100%",
        height: 3,
        background: "linear-gradient(90deg, #18181b, #27272a)",
        flexDirection: "row",
        alignItems: "center",
        padding: [0, 2],
        gap: 3,
      }}
    >
      <Text style={{ color: t.primary, fontWeight: "bold" }}>
        {"KittyUI"}
      </Text>
      <HeaderTabs active={active} onSelect={onSelect} />
      <Box style={{ flexGrow: 1 }} />
      <Text style={{ color: t.mutedFg, dim: true }}>
        {"Press q to quit"}
      </Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Metric card
// ---------------------------------------------------------------------------

function MetricCard({
  title,
  value,
  subtitle,
  icon,
}: {
  title: string;
  value: string;
  subtitle: string;
  icon: string;
}) {
  return (
    <Box style={{ ...cardStyle, flexGrow: 1, minWidth: 18, height: 6, gap: 0 }}>
      <Box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <Text style={{ color: t.mutedFg, dim: true }}>{title}</Text>
        <Text style={{ color: t.mutedFg }}>{icon}</Text>
      </Box>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{value}</Text>
      <Text style={{ color: t.mutedFg, dim: true }}>{subtitle}</Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Recent sales
// ---------------------------------------------------------------------------

interface Sale {
  name: string;
  email: string;
  amount: string;
  indicator: string;
}

const RECENT_SALES: Sale[] = [
  { name: "Olivia Martin", email: "olivia@email.com", amount: "+$1,999.00", indicator: "OM" },
  { name: "Jackson Lee", email: "jackson@email.com", amount: "+$39.00", indicator: "JL" },
  { name: "Isabella Nguyen", email: "isabella@email.com", amount: "+$299.00", indicator: "IN" },
  { name: "William Kim", email: "will@email.com", amount: "+$99.00", indicator: "WK" },
  { name: "Sofia Davis", email: "sofia@email.com", amount: "+$39.00", indicator: "SD" },
];

function SaleRow({ sale }: { sale: Sale }) {
  return (
    <Box
      style={{
        flexDirection: "row",
        alignItems: "center",
        height: 2,
        gap: 2,
        paddingLeft: 1,
        paddingRight: 1,
      }}
    >
      <Box
        style={{
          width: 4,
          height: 1,
          backgroundColor: t.accent,
          borderRadius: 4,
          justifyContent: "center",
          alignItems: "center",
        }}
      >
        <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{sale.indicator}</Text>
      </Box>
      <Box style={{ flexDirection: "column", flexGrow: 1 }}>
        <Text style={{ color: t.cardFg }}>{sale.name}</Text>
        <Text style={{ color: t.mutedFg, dim: true }}>{sale.email}</Text>
      </Box>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{sale.amount}</Text>
    </Box>
  );
}

function RecentSalesCard() {
  return (
    <Box style={{ ...cardStyle, flexGrow: 1, gap: 1 }}>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{"Recent Sales"}</Text>
      <Text style={{ color: t.mutedFg, dim: true }}>
        {"You made 265 sales this month."}
      </Text>
      {RECENT_SALES.map((sale) => (
        <SaleRow key={sale.name} sale={sale} />
      ))}
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Sparkline chart helpers
// ---------------------------------------------------------------------------

const SPARK_BLOCKS = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

function sparkline(data: number[]): string {
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;
  return data
    .map((v) => {
      const idx = Math.round(((v - min) / range) * (SPARK_BLOCKS.length - 1));
      return SPARK_BLOCKS[idx];
    })
    .join("");
}

// ---------------------------------------------------------------------------
// Overview revenue chart (ASCII bar-style)
// ---------------------------------------------------------------------------

const MONTHLY_REVENUE = [
  { month: "Jan", value: 4200 },
  { month: "Feb", value: 3800 },
  { month: "Mar", value: 5100 },
  { month: "Apr", value: 4600 },
  { month: "May", value: 5800 },
  { month: "Jun", value: 6200 },
  { month: "Jul", value: 5400 },
  { month: "Aug", value: 4900 },
  { month: "Sep", value: 6800 },
  { month: "Oct", value: 7200 },
  { month: "Nov", value: 6100 },
  { month: "Dec", value: 7800 },
];

function RevenueChartCard() {
  const maxVal = Math.max(...MONTHLY_REVENUE.map((m) => m.value));
  const barMaxHeight = 8;

  return (
    <Box style={{ ...cardStyle, flexGrow: 2, gap: 1 }}>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{"Overview"}</Text>

      {/* Chart area */}
      <Box style={{ flexDirection: "row", alignItems: "end", gap: 1, height: barMaxHeight }}>
        {MONTHLY_REVENUE.map((m) => {
          const height = Math.max(1, Math.round((m.value / maxVal) * barMaxHeight));
          return (
            <Box
              key={m.month}
              style={{
                flexGrow: 1,
                height,
                backgroundColor: t.cardFg,
                borderRadius: 2,
              }}
            />
          );
        })}
      </Box>

      {/* Month labels */}
      <Box style={{ flexDirection: "row", gap: 1 }}>
        {MONTHLY_REVENUE.map((m) => (
          <Box key={m.month} style={{ flexGrow: 1 }}>
            <Text style={{ color: t.mutedFg, dim: true }}>{m.month}</Text>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Overview page
// ---------------------------------------------------------------------------

function OverviewPage({ uptime }: { uptime: number }) {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, gap: 2, padding: [1, 2] }}>
      {/* Gradient accent strip */}
      <Box
        style={{
          width: "100%",
          height: 1,
          background: "linear-gradient(90deg, #667eea 0%, #764ba2 50%, #f093fb 100%)",
          borderRadius: 4,
        }}
      />

      {/* Metric cards row */}
      <Box style={{ flexDirection: "row", gap: 2 }}>
        <MetricCard
          title="Total Revenue"
          value="$45,231.89"
          subtitle="+20.1% from last month"
          icon="$"
        />
        <MetricCard
          title="Subscriptions"
          value="+2,350"
          subtitle="+180.1% from last month"
          icon="+"
        />
        <MetricCard
          title="Sales"
          value="+12,234"
          subtitle="+19% from last month"
          icon="$"
        />
        <MetricCard
          title="Active Now"
          value={"+573"}
          subtitle={`Uptime ${uptime}s`}
          icon="@"
        />
      </Box>

      {/* Chart + Recent Sales */}
      <Box style={{ flexDirection: "row", gap: 2, flexGrow: 1 }}>
        <RevenueChartCard />
        <RecentSalesCard />
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Analytics page
// ---------------------------------------------------------------------------

const VISITOR_DATA = [120, 180, 150, 210, 280, 320, 290, 340, 380, 350, 420, 460, 430, 510, 480, 520, 560, 540, 610, 590];
const PAGEVIEW_DATA = [340, 420, 380, 510, 620, 710, 680, 740, 820, 780, 890, 930, 910, 1020, 980, 1050, 1120, 1080, 1210, 1180];
const BOUNCE_DATA = [65, 58, 62, 55, 48, 42, 45, 38, 35, 40, 32, 28, 30, 25, 28, 22, 20, 24, 18, 15];

function SparklineCard({
  title,
  value,
  change,
  changeColor,
  data,
  sparkColor,
}: {
  title: string;
  value: string;
  change: string;
  changeColor: string;
  data: number[];
  sparkColor: string;
}) {
  return (
    <Box style={{ ...cardStyle, flexGrow: 1, gap: 1 }}>
      <Box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <Text style={{ color: t.mutedFg }}>{title}</Text>
        <Text style={{ color: changeColor, fontWeight: "bold" }}>{change}</Text>
      </Box>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{value}</Text>
      <Text style={{ color: sparkColor }}>{sparkline(data)}</Text>
    </Box>
  );
}

function AnalyticsTable() {
  const rows = [
    { page: "/", views: "4,231", uniques: "2,108", bounce: "32%", time: "2m 34s" },
    { page: "/pricing", views: "3,847", uniques: "1,923", bounce: "28%", time: "3m 12s" },
    { page: "/docs", views: "2,156", uniques: "1,432", bounce: "45%", time: "4m 56s" },
    { page: "/blog", views: "1,893", uniques: "987", bounce: "52%", time: "1m 48s" },
    { page: "/about", views: "1,245", uniques: "876", bounce: "38%", time: "2m 02s" },
    { page: "/changelog", views: "987", uniques: "654", bounce: "41%", time: "1m 23s" },
  ];

  return (
    <Box style={{ ...cardStyle, flexGrow: 1, gap: 1 }}>
      <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{"Top Pages"}</Text>
      <Text style={{ color: t.mutedFg, dim: true }}>{"This Month vs Last Month"}</Text>

      {/* Table header */}
      <Box style={{ flexDirection: "row", paddingTop: 1, gap: 2 }}>
        <Box style={{ width: 16 }}>
          <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Page"}</Text>
        </Box>
        <Box style={{ width: 10 }}>
          <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Views"}</Text>
        </Box>
        <Box style={{ width: 10 }}>
          <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Uniques"}</Text>
        </Box>
        <Box style={{ width: 10 }}>
          <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Bounce"}</Text>
        </Box>
        <Box style={{ flexGrow: 1 }}>
          <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Avg. Time"}</Text>
        </Box>
      </Box>

      {/* Separator */}
      <Box style={{ height: 1, width: "100%", backgroundColor: t.border }} />

      {/* Table rows */}
      {rows.map((row) => (
        <Box key={row.page} style={{ flexDirection: "row", height: 1, gap: 2 }}>
          <Box style={{ width: 16 }}>
            <Text style={{ color: t.cardFg }}>{row.page}</Text>
          </Box>
          <Box style={{ width: 10 }}>
            <Text style={{ color: t.cardFg }}>{row.views}</Text>
          </Box>
          <Box style={{ width: 10 }}>
            <Text style={{ color: t.cardFg }}>{row.uniques}</Text>
          </Box>
          <Box style={{ width: 10 }}>
            <Text style={{ color: t.mutedFg }}>{row.bounce}</Text>
          </Box>
          <Box style={{ flexGrow: 1 }}>
            <Text style={{ color: t.mutedFg }}>{row.time}</Text>
          </Box>
        </Box>
      ))}
    </Box>
  );
}

function AnalyticsPage() {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, gap: 2, padding: [1, 2] }}>
      {/* Sparkline row */}
      <Box style={{ flexDirection: "row", gap: 2 }}>
        <SparklineCard
          title="Total Visitors"
          value="12,480"
          change="+24.5%"
          changeColor={t.success}
          data={VISITOR_DATA}
          sparkColor={t.success}
        />
        <SparklineCard
          title="Page Views"
          value="34,720"
          change="+18.2%"
          changeColor={t.success}
          data={PAGEVIEW_DATA}
          sparkColor={t.blue}
        />
        <SparklineCard
          title="Bounce Rate"
          value="24.3%"
          change="-12.1%"
          changeColor={t.success}
          data={BOUNCE_DATA}
          sparkColor={t.warning}
        />
      </Box>

      {/* Table */}
      <AnalyticsTable />
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Reports page
// ---------------------------------------------------------------------------

function ReportsPage() {
  const reports = [
    { name: "Q4 Revenue Report", status: "Complete", date: "Dec 31, 2025", type: "Financial" },
    { name: "User Growth Analysis", status: "Complete", date: "Jan 15, 2026", type: "Growth" },
    { name: "Churn Analysis", status: "In Progress", date: "Mar 01, 2026", type: "Retention" },
    { name: "Marketing ROI", status: "Complete", date: "Feb 28, 2026", type: "Marketing" },
    { name: "Infrastructure Costs", status: "Pending", date: "Mar 15, 2026", type: "Operations" },
    { name: "Customer Satisfaction", status: "In Progress", date: "Mar 10, 2026", type: "Support" },
  ];

  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, gap: 2, padding: [1, 2] }}>
      <Box style={{ ...cardStyle, gap: 1 }}>
        <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{"Reports"}</Text>
        <Text style={{ color: t.mutedFg, dim: true }}>
          {"Manage and download generated reports."}
        </Text>

        {/* Header */}
        <Box style={{ flexDirection: "row", paddingTop: 1, gap: 2 }}>
          <Box style={{ flexGrow: 1 }}>
            <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Report Name"}</Text>
          </Box>
          <Box style={{ width: 14 }}>
            <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Type"}</Text>
          </Box>
          <Box style={{ width: 16 }}>
            <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Date"}</Text>
          </Box>
          <Box style={{ width: 14 }}>
            <Text style={{ color: t.mutedFg, fontWeight: "bold" }}>{"Status"}</Text>
          </Box>
        </Box>

        <Box style={{ height: 1, width: "100%", backgroundColor: t.border }} />

        {reports.map((r) => {
          const statusColor =
            r.status === "Complete"
              ? t.success
              : r.status === "In Progress"
                ? t.warning
                : t.mutedFg;
          const statusIcon =
            r.status === "Complete"
              ? "✓"
              : r.status === "In Progress"
                ? "●"
                : "○";
          return (
            <Box key={r.name} style={{ flexDirection: "row", height: 1, gap: 2 }}>
              <Box style={{ flexGrow: 1 }}>
                <Text style={{ color: t.cardFg }}>{r.name}</Text>
              </Box>
              <Box style={{ width: 14 }}>
                <Text style={{ color: t.mutedFg }}>{r.type}</Text>
              </Box>
              <Box style={{ width: 16 }}>
                <Text style={{ color: t.mutedFg }}>{r.date}</Text>
              </Box>
              <Box style={{ width: 14 }}>
                <Text style={{ color: statusColor }}>
                  {statusIcon + " " + r.status}
                </Text>
              </Box>
            </Box>
          );
        })}
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

function SettingsPage() {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, gap: 2, padding: [1, 2] }}>
      <Box style={{ ...cardStyle, gap: 1 }}>
        <Text style={{ color: t.cardFg, fontWeight: "bold" }}>{"General Settings"}</Text>
        <Text style={{ color: t.mutedFg, dim: true }}>
          {"Manage your account settings and preferences."}
        </Text>

        <Box style={{ height: 1, width: "100%", backgroundColor: t.border, marginTop: 1 }} />

        {/* Settings rows */}
        {[
          { label: "Theme", value: "Dark", desc: "Choose your preferred color scheme" },
          { label: "Language", value: "English", desc: "Select display language" },
          { label: "Timezone", value: "UTC-8 (PST)", desc: "Set your local timezone" },
          { label: "Notifications", value: "Enabled", desc: "Email and push notification preferences" },
          { label: "Two-Factor Auth", value: "Active", desc: "Additional security for your account" },
        ].map((setting) => (
          <Box
            key={setting.label}
            style={{
              flexDirection: "row",
              alignItems: "center",
              height: 3,
              paddingLeft: 1,
              paddingRight: 1,
            }}
          >
            <Box style={{ flexDirection: "column", flexGrow: 1 }}>
              <Text style={{ color: t.cardFg }}>{setting.label}</Text>
              <Text style={{ color: t.mutedFg, dim: true }}>{setting.desc}</Text>
            </Box>
            <Box
              style={{
                backgroundColor: t.accent,
                borderRadius: 4,
                paddingLeft: 1,
                paddingRight: 1,
                height: 1,
              }}
            >
              <Text style={{ color: t.cardFg }}>{setting.value}</Text>
            </Box>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

function Footer({ cols, rows, uptime }: { cols: number; rows: number; uptime: number }) {
  return (
    <Box
      style={{
        width: "100%",
        height: 1,
        backgroundColor: t.card,
        flexDirection: "row",
        padding: [0, 2],
        gap: 2,
      }}
    >
      <Text style={{ color: t.mutedFg, dim: true }}>
        {"←/→ navigate tabs"}
      </Text>
      <Box style={{ flexGrow: 1 }} />
      <Text style={{ color: t.mutedFg, dim: true }}>
        {cols + "x" + rows}
      </Text>
      <Text style={{ color: t.success }}>
        {"● " + uptime + "s uptime"}
      </Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Main App
// ---------------------------------------------------------------------------

export function App() {
  const [activeTab, setActiveTab] = useState<Tab>("Overview");
  const [uptime, setUptime] = useState(0);
  const { cols, rows } = useTerminal();

  // Live uptime counter
  useEffect(() => {
    const ONE_SECOND = 1000;
    const timer = setInterval(() => {
      setUptime((prev: number) => prev + 1);
    }, ONE_SECOND);
    return () => clearInterval(timer);
  }, []);

  // Keyboard navigation: left/right arrows switch tabs
  useKeyboard(
    (event) => {
      const keyCode = event.keyCode;
      const tabIndex = TABS.indexOf(activeTab);
      if (keyCode === KEY_LEFT) {
        const next = tabIndex > 0 ? tabIndex - 1 : TABS.length - 1;
        setActiveTab(TABS[next]);
      } else if (keyCode === KEY_RIGHT) {
        const next = tabIndex < TABS.length - 1 ? tabIndex + 1 : 0;
        setActiveTab(TABS[next]);
      }
    },
    { global: true },
  );

  let content: React.ReactNode;
  switch (activeTab) {
    case "Overview":
      content = <OverviewPage uptime={uptime} />;
      break;
    case "Analytics":
      content = <AnalyticsPage />;
      break;
    case "Reports":
      content = <ReportsPage />;
      break;
    case "Settings":
      content = <SettingsPage />;
      break;
  }

  return (
    <Box
      style={{
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: t.bg,
      }}
    >
      <Header active={activeTab} onSelect={setActiveTab} />
      {content}
      <Footer cols={cols} rows={rows} uptime={uptime} />
    </Box>
  );
}
