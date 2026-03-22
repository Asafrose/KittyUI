/**
 * Entry point for the KittyUI Dashboard POC.
 *
 * Bootstraps the app using createApp() which handles:
 * - Alt-screen entry, cursor hiding
 * - React reconciler setup
 * - Render loop (30 fps)
 * - stdin raw mode for keyboard input
 * - Graceful shutdown on q / Ctrl+C
 */

import { createApp } from "@kittyui/react";
import { App } from "./app.js";

const debug = process.argv.includes("--debug");

createApp(<App />, { debug });
