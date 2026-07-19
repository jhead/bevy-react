/**
 * DevTools snapshot / fiber helpers (no live WebSocket).
 */
import { afterEach, describe, expect, it } from "vitest";
import {
  BEVY_RDT_BRIDGE_PROTOCOL,
  getDevToolsSnapshot,
  getFiberTreeSnapshot,
} from "../src/devtools";
import { ensureRoot, unmountRoot } from "../src/roots";

describe("devtools", () => {
  afterEach(() => {
    unmountRoot("devtools-test");
  });

  it("exposes RDT bridge protocol metadata", () => {
    expect(BEVY_RDT_BRIDGE_PROTOCOL.version).toBe(2);
    expect(BEVY_RDT_BRIDGE_PROTOCOL.note).toContain("not Int32");
  });

  it("dump returns empty roots when nothing mounted", () => {
    const snap = getDevToolsSnapshot();
    expect(snap.renderer).toBe("bevy-react");
    expect(snap.version).toBe(1);
    expect(Array.isArray(snap.roots)).toBe(true);
  });

  it("fiber dump includes a root after ensureRoot", () => {
    ensureRoot("devtools-test");
    const fibers = getFiberTreeSnapshot();
    expect(fibers.kind).toBe("bevy-fiber-snapshot");
    const root = fibers.roots.find((r) => r.rootId === "devtools-test");
    expect(root).toBeTruthy();
    expect(root!.rendererID).toBe(1);
    // HostRoot fiber at minimum once the container exists.
    expect(root!.fibers.length).toBeGreaterThanOrEqual(1);
    expect(root!.fibers.some((f) => f.name === "Root" || f.tag === 3)).toBe(
      true
    );
  });
});
