/**
 * shadcn/ui Dashboard Clone -- KittyUI showcase demo.
 *
 * Pixel-perfect reproduction of the shadcn/ui dark mode dashboard.
 * Uses the zinc dark palette with minimal borders and no shadows.
 */

import { useState } from "react";
import {
  useKeyboard,
  KEY_LEFT,
  KEY_RIGHT,
  Box,
  Text,
} from "@kittyui/react";
import type { CSSStyle, KeyboardEvent as KittyKeyboardEvent } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Design tokens (shadcn zinc dark -- exact palette)
// ---------------------------------------------------------------------------

const t = {
  bg: "#09090b",          // zinc-950
  card: "#09090b",        // same as bg per shadcn dark
  cardBorder: "#27272a",  // zinc-800
  muted: "#27272a",       // zinc-800
  mutedFg: "#a1a1aa",     // zinc-400
  foreground: "#fafafa",  // zinc-50
  primary: "#fafafa",     // zinc-50
  secondary: "#27272a",   // zinc-800
  accent: "#27272a",      // zinc-800
  destructive: "#7f1d1d", // red-900
  zinc500: "#71717a",     // zinc-500
} as const;

// ---------------------------------------------------------------------------
// Avatar colors per person
// ---------------------------------------------------------------------------

const AVATAR_COLORS: Record<string, string> = {
  OM: "#7c3aed", // violet
  JL: "#0ea5e9", // sky
  IN: "#f59e0b", // amber
  WK: "#22c55e", // green
  SD: "#ec4899", // pink
};

// ---------------------------------------------------------------------------
// Reusable card style (every card must use this)
// ---------------------------------------------------------------------------

const cardStyle: CSSStyle = {
  backgroundColor: "#09090b",
  borderRadius: 8,
  border: "round" as const,
  borderColor: "#27272a",
  padding: [1, 2] as [number, number],
  flexDirection: "column" as const,
};

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

const TABS = ["Overview", "Analytics", "Reports"] as const;
type Tab = (typeof TABS)[number];

function Header({ active }: { active: Tab }) {
  return (
    <Box
      style={{
        width: "100%",
        height: 3,
        flexDirection: "row",
        alignItems: "center",
        padding: [0, 2],
        gap: 3,
      }}
    >
      <Text style={{ color: t.foreground, fontWeight: "bold" }}>
        {"Dashboard"}
      </Text>
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
                  color: isActive ? t.foreground : t.mutedFg,
                  fontWeight: isActive ? "bold" : "normal",
                  textDecoration: isActive ? "underline" : "none",
                }}
              >
                {tab}
              </Text>
            </Box>
          );
        })}
      </Box>
      <Box style={{ flexGrow: 1 }} />
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Separator
// ---------------------------------------------------------------------------

function Separator() {
  return (
    <Box
      style={{
        width: "100%",
        height: 1,
        backgroundColor: t.cardBorder,
      }}
    />
  );
}

// ---------------------------------------------------------------------------
// Metric card
// ---------------------------------------------------------------------------

function MetricCard({
  title,
  value,
  description,
  icon,
}: {
  title: string;
  value: string;
  description: string;
  icon: string;
}) {
  return (
    <Box style={{ ...cardStyle, flexGrow: 1, minWidth: 18, height: 6, gap: 0 }}>
      <Box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <Text style={{ color: t.mutedFg }}>{title}</Text>
        <Text style={{ color: t.mutedFg }}>{icon}</Text>
      </Box>
      <Text style={{ color: t.foreground, fontWeight: "bold" }}>{value}</Text>
      <Text style={{ color: t.zinc500 }}>{description}</Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Revenue bar chart
// ---------------------------------------------------------------------------

const MONTHLY_REVENUE = [
  { month: "Jan", value: 4200 },
  { month: "Feb", value: 3800 },
  { month: "Mar", value: 5500 },
  { month: "Apr", value: 4700 },
  { month: "May", value: 6300 },
  { month: "Jun", value: 5800 },
  { month: "Jul", value: 7100 },
  { month: "Aug", value: 6500 },
  { month: "Sep", value: 8000 },
  { month: "Oct", value: 7400 },
  { month: "Nov", value: 6800 },
  { month: "Dec", value: 8500 },
];

function RevenueChartCard() {
  const maxVal = Math.max(...MONTHLY_REVENUE.map((m) => m.value));
  const barMaxHeight = 12;

  return (
    <Box style={{ ...cardStyle, flexGrow: 2, gap: 1 }}>
      <Text style={{ color: t.foreground, fontWeight: "bold" }}>{"Overview"}</Text>

      {/* Chart area -- bars rendered via backgroundColor + height */}
      <Box style={{ flexDirection: "row", alignItems: "end", gap: 1, height: barMaxHeight }}>
        {MONTHLY_REVENUE.map((m) => {
          const barHeight = Math.max(1, Math.round((m.value / maxVal) * barMaxHeight));
          return (
            <Box
              key={m.month}
              style={{
                width: 4,
                flexGrow: 1,
                flexDirection: "column",
                justifyContent: "end",
              }}
            >
              <Box
                style={{
                  height: barHeight,
                  backgroundColor: "#fafafa",
                  borderRadius: 2,
                }}
              />
            </Box>
          );
        })}
      </Box>

      {/* Month labels */}
      <Box style={{ flexDirection: "row", gap: 1 }}>
        {MONTHLY_REVENUE.map((m) => (
          <Box key={m.month} style={{ flexGrow: 1, width: 4 }}>
            <Text style={{ color: t.mutedFg }}>{m.month}</Text>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Avatar
// ---------------------------------------------------------------------------

function Avatar({ initials, color }: { initials: string; color: string }) {
  return (
    <Box
      style={{
        width: 4,
        height: 2,
        backgroundColor: color,
        borderRadius: 16,
        justifyContent: "center",
        alignItems: "center",
      }}
    >
      <Text style={{ color: "#fafafa", fontWeight: "bold" }}>{initials}</Text>
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
  initials: string;
}

const RECENT_SALES: Sale[] = [
  { name: "Olivia Martin", email: "olivia@email.com", amount: "+$1,999.00", initials: "OM" },
  { name: "Jackson Lee", email: "jackson@email.com", amount: "+$39.00", initials: "JL" },
  { name: "Isabella Nguyen", email: "isabella@email.com", amount: "+$299.00", initials: "IN" },
  { name: "William Kim", email: "will@email.com", amount: "+$99.00", initials: "WK" },
  { name: "Sofia Davis", email: "sofia@email.com", amount: "+$39.00", initials: "SD" },
];

function SaleRow({ sale }: { sale: Sale }) {
  const avatarColor = AVATAR_COLORS[sale.initials] ?? t.accent;
  return (
    <Box
      style={{
        flexDirection: "row",
        alignItems: "center",
        height: 2,
        gap: 2,
      }}
    >
      <Avatar initials={sale.initials} color={avatarColor} />
      <Box style={{ flexDirection: "column", flexGrow: 1 }}>
        <Text style={{ color: t.foreground }}>{sale.name}</Text>
        <Text style={{ color: t.mutedFg }}>{sale.email}</Text>
      </Box>
      <Text style={{ color: t.foreground, fontWeight: "bold" }}>{sale.amount}</Text>
    </Box>
  );
}

function RecentSalesCard() {
  return (
    <Box style={{ ...cardStyle, flexGrow: 1, gap: 1, padding: [2, 3] as [number, number] }}>
      <Text style={{ color: t.foreground, fontWeight: "bold" }}>{"Recent Sales"}</Text>
      <Text style={{ color: t.zinc500 }}>
        {"You made 265 sales this month."}
      </Text>
      {RECENT_SALES.map((sale) => (
        <SaleRow key={sale.name} sale={sale} />
      ))}
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Overview page
// ---------------------------------------------------------------------------

function OverviewPage() {
  return (
    <Box style={{ flexDirection: "column", flexGrow: 1, gap: 2, padding: [1, 2] }}>
      {/* Metric cards row */}
      <Box style={{ flexDirection: "row", gap: 2 }}>
        <MetricCard
          title="Total Revenue"
          value="$45,231.89"
          description="+20.1% from last month"
          icon="$"
        />
        <MetricCard
          title="Subscriptions"
          value="+2,350"
          description="+180.1% from last month"
          icon="Users"
        />
        <MetricCard
          title="Sales"
          value="+12,234"
          description="+19% from last month"
          icon="CCard"
        />
        <MetricCard
          title="Active Now"
          value="+573"
          description="+201 since last hour"
          icon="Act"
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
// Footer
// ---------------------------------------------------------------------------

function Footer() {
  return (
    <Box
      style={{
        width: "100%",
        height: 1,
        flexDirection: "row",
        padding: [0, 2],
        gap: 2,
      }}
    >
      <Text style={{ color: t.mutedFg }}>
        {"<-/-> navigate tabs   q quit"}
      </Text>
      <Box style={{ flexGrow: 1 }} />
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Main App
// ---------------------------------------------------------------------------

export function App() {
  const [activeTab, setActiveTab] = useState<Tab>("Overview");

  // Keyboard navigation: left/right arrows switch tabs
  useKeyboard(
    (event: KittyKeyboardEvent) => {
      const keyCode = (event as unknown as { keyCode: number }).keyCode;
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

  return (
    <Box
      style={{
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: t.bg,
      }}
    >
      <Header active={activeTab} />
      <Separator />
      <OverviewPage />
      <Footer />
    </Box>
  );
}
