/**
 * Test harness — re-exports for headless testing of KittyUI.
 */

export { VirtualScreen } from "./virtual-screen.js";
export { TestBridge } from "./test-bridge.js";
export type { NodeLayout } from "./test-bridge.js";

// Side-effect import to register custom matchers.
import "./assertions.js";
