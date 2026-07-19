/**
 * DevTools integration for bevy-react.
 *
 * MVP: register the reconciler with React's DevTools hook via
 * `injectIntoDevTools`, expose a JSON tree dump for the Rust WebSocket
 * bridge (`devtools` Cargo feature), and optionally push snapshots to
 * `ws://127.0.0.1:8098` (bevy-react debug server — not the full RDT protocol).
 *
 * Full standalone React DevTools (`npx react-devtools` on :8097) needs
 * `react-devtools-core` speaking the official backend protocol. Boa + our
 * WebSocket shim may support that later; this module ships a practical
 * inspector bridge today.
 */

import type { Fiber } from "react-reconciler";
import type { BevyReconciler, PublicInstance } from "./reconciler";
import type { BevyInstance, BevyTextInstance } from "./types";
import { listRoots, rootCount } from "./roots";

export const BEVY_REACT_DEVTOOLS_PORT = 8098;
export const BEVY_REACT_DEVTOOLS_DEFAULT_URL = `ws://127.0.0.1:${BEVY_REACT_DEVTOOLS_PORT}`;

export type DevToolsNodeSnapshot = {
  nodeId: number;
  kind: "host" | "text";
  type?: string;
  text?: string;
  parentId?: number;
  childIds: number[];
  props?: Record<string, unknown>;
};

export type DevToolsRootSnapshot = {
  rootId: string;
  nodes: DevToolsNodeSnapshot[];
};

export type DevToolsSnapshot = {
  version: 1;
  renderer: "bevy-react";
  rootCount: number;
  roots: DevToolsRootSnapshot[];
  capturedAt: number;
};

type DevToolsApi = {
  dump: () => DevToolsSnapshot;
  dumpJson: () => string;
  connect: (url?: string) => void;
  disconnect: () => void;
};

declare global {
  // eslint-disable-next-line no-var
  var __bevyReactDevTools: DevToolsApi | undefined;
}

let attached = false;
let bridgeSocket: WebSocket | null = null;
let bridgeTimer: ReturnType<typeof setInterval> | number | null = null;

function isHostInstance(inst: PublicInstance): inst is BevyInstance {
  return "type" in inst;
}

function isTextInstance(inst: PublicInstance): inst is BevyTextInstance {
  return !("type" in inst);
}

/** Serialize props for the dump (drop functions). */
function serializeProps(
  props: Record<string, unknown> | undefined
): Record<string, unknown> | undefined {
  if (!props) return undefined;
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(props)) {
    if (key === "children") continue;
    if (typeof value === "function") {
      out[key] = "[Function]";
      continue;
    }
    if (value === undefined) continue;
    try {
      JSON.stringify(value);
      out[key] = value;
    } catch {
      out[key] = String(value);
    }
  }
  return out;
}

/**
 * Build a snapshot of all mounted bevy-react host trees.
 */
export function getDevToolsSnapshot(): DevToolsSnapshot {
  return {
    version: 1,
    renderer: "bevy-react",
    rootCount: rootCount(),
    roots: listRootSnapshots(),
    capturedAt: Date.now(),
  };
}

export function getDevToolsSnapshotJson(): string {
  return JSON.stringify(getDevToolsSnapshot());
}

function listRootSnapshots(): DevToolsRootSnapshot[] {
  const out: DevToolsRootSnapshot[] = [];
  for (const state of listRoots()) {
    const nodes: DevToolsNodeSnapshot[] = [];
    for (const [, inst] of state.instanceMap) {
      if (isHostInstance(inst)) {
        nodes.push({
          nodeId: inst.nodeId,
          kind: "host",
          type: inst.type,
          parentId: inst.parentId,
          childIds: inst.children.map((c) => c.nodeId),
          props: serializeProps(inst.props),
        });
      } else if (isTextInstance(inst)) {
        nodes.push({
          nodeId: inst.nodeId,
          kind: "text",
          text: inst.text,
          childIds: [],
        });
      }
    }
    nodes.sort((a, b) => a.nodeId - b.nodeId);
    out.push({ rootId: state.rootId, nodes });
  }
  return out;
}

/**
 * Call once per reconciler (from `ensureRoot`) so DevTools / the hook can see us.
 */
export function injectBevyReactDevTools(reconciler: BevyReconciler): void {
  reconciler.injectIntoDevTools({
    bundleType: 1, // development
    version: "0.1.0",
    rendererPackageName: "bevy-react",
    // Host hit-testing → fiber is not wired yet (no DOM). Null is correct for MVP.
    findFiberByHostInstance: (_instance: unknown) => null as Fiber | null,
    rendererConfig: {
      supportsMutation: true,
      supportsPersistence: false,
      supportsHydration: false,
    },
  });

  ensureGlobalApi();
}

function ensureGlobalApi(): void {
  if (attached) return;
  attached = true;

  const api: DevToolsApi = {
    dump: getDevToolsSnapshot,
    dumpJson: getDevToolsSnapshotJson,
    connect: connectDevToolsBridge,
    disconnect: disconnectDevToolsBridge,
  };

  globalThis.__bevyReactDevTools = api;

  // Auto-connect to the Rust debug bridge when the WebSocket shim is present.
  // Failures are silent — production / missing feature just skips the bridge.
  try {
    if (typeof WebSocket === "function") {
      connectDevToolsBridge();
    }
  } catch (err) {
    console.warn("[bevy-react] DevTools bridge auto-connect failed:", err);
  }

  console.log(
    "[bevy-react] DevTools ready — dump via __bevyReactDevTools.dump(), " +
      `bridge ws://127.0.0.1:${BEVY_REACT_DEVTOOLS_PORT}`
  );
}

/**
 * Connect to the bevy-react Rust debug WebSocket and push tree snapshots.
 */
export function connectDevToolsBridge(
  url: string = BEVY_REACT_DEVTOOLS_DEFAULT_URL
): void {
  disconnectDevToolsBridge();

  if (typeof WebSocket !== "function") {
    console.warn("[bevy-react] WebSocket unavailable; DevTools bridge disabled");
    return;
  }

  const ws = new WebSocket(url);
  bridgeSocket = ws;

  ws.onopen = () => {
    console.log("[bevy-react] DevTools bridge connected:", url);
    pushSnapshot(ws);
  };

  ws.onmessage = (event: { data?: unknown }) => {
    let raw = "";
    if (typeof event.data === "string") {
      raw = event.data;
    } else if (event.data != null) {
      raw = String(event.data);
    }
    try {
      const msg = JSON.parse(raw) as { type?: string };
      if (msg.type === "request_dump" || msg.type === "ping") {
        pushSnapshot(ws);
      }
    } catch {
      // ignore non-JSON
    }
  };

  ws.onerror = () => {
    // Expected when the `devtools` Cargo feature is off — keep quiet after first try.
  };

  ws.onclose = () => {
    if (bridgeSocket === ws) {
      bridgeSocket = null;
    }
  };

  if (bridgeTimer != null) {
    clearInterval(bridgeTimer);
  }
  bridgeTimer = setInterval(() => {
    if (bridgeSocket && bridgeSocket.readyState === WebSocket.OPEN) {
      pushSnapshot(bridgeSocket);
    }
  }, 2000);
}

export function disconnectDevToolsBridge(): void {
  if (bridgeTimer != null) {
    clearInterval(bridgeTimer);
    bridgeTimer = null;
  }
  if (bridgeSocket) {
    try {
      bridgeSocket.close();
    } catch {
      // ignore
    }
    bridgeSocket = null;
  }
}

function pushSnapshot(ws: WebSocket): void {
  if (ws.readyState !== WebSocket.OPEN) return;
  const payload = {
    type: "tree",
    ...getDevToolsSnapshot(),
  };
  try {
    ws.send(JSON.stringify(payload));
  } catch (err) {
    console.warn("[bevy-react] Failed to push DevTools snapshot:", err);
  }
}
