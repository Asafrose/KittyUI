/**
 * Entry point for the KittyUI shadcn Dashboard.
 */

import { createApp } from "@kittyui/react";
import { App } from "./app.js";

const debug = process.argv.includes("--debug");

createApp(<App />, { debug });
