/**
 * TerminalContext — React context providing terminal dimensions and bridge access.
 */

import { createContext, createElement, useState, useEffect, type ReactNode } from "react";
import type { Bridge } from "@kittyui/core";

// ---------------------------------------------------------------------------
// Context value
// ---------------------------------------------------------------------------

export interface TerminalContextValue {
  /** Number of terminal columns. */
  cols: number;
  /** Number of terminal rows. */
  rows: number;
  /** The KittyUI Bridge instance. */
  bridge: Bridge;
}

/**
 * React context that provides terminal dimensions and the Bridge instance.
 * Must be consumed within a `<TerminalProvider>`.
 */
// eslint-disable-next-line unicorn/no-null -- createContext requires a default value
export const TerminalContext = createContext<TerminalContextValue | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

interface TerminalProviderProps {
  bridge: Bridge;
  children?: ReactNode;
}

/**
 * Provider component that wraps the user's app and exposes terminal dimensions
 * and the Bridge via React context. Listens for resize events on stdout to
 * keep `cols` and `rows` up to date.
 */
export const TerminalProvider = ({ bridge, children }: TerminalProviderProps): ReactNode => {
  const [cols, setCols] = useState(process.stdout.columns || 80);
  const [rows, setRows] = useState(process.stdout.rows || 24);

  useEffect(() => {
    const onResize = (): void => {
      setCols(process.stdout.columns || 80);
      setRows(process.stdout.rows || 24);
    };
    process.stdout.on("resize", onResize);
    return () => {
      process.stdout.off("resize", onResize);
    };
  }, []);

  return createElement(
    TerminalContext.Provider,
    { value: { cols, rows, bridge } },
    children,
  );
};
