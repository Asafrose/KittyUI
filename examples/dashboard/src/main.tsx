/**
 * Entry point for the KittyUI shadcn Dashboard.
 */

import { createApp } from "@kittyui/react";
import { App } from "./app.js";

const debug = process.argv.includes("--debug");

const screenshotDir = process.argv.find(a => a.startsWith("--screenshot-dir="))?.split("=")[1]
    ?? (process.argv.includes("--screenshot-dir")
        ? process.argv[process.argv.indexOf("--screenshot-dir") + 1]
        : undefined);

createApp(<App />, { debug, screenshotDir });
