/**
 * Entry point for the KittyUI shadcn Dashboard.
 */

import { createApp } from "@kittyui/react";
import { App } from "./app.js";

const debug = process.argv.includes("--debug");
const pixel = process.argv.includes("--pixel");
const cell = process.argv.includes("--cell");
const renderMode = pixel ? "pixel" as const : cell ? "cell" as const : "auto" as const;

const screenshotDir = process.argv.find(a => a.startsWith("--screenshot-dir="))?.split("=")[1]
    ?? (process.argv.includes("--screenshot-dir")
        ? process.argv[process.argv.indexOf("--screenshot-dir") + 1]
        : undefined);

createApp(<App />, { debug, renderMode, screenshotDir });
