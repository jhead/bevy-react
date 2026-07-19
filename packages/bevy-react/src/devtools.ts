/**
 * DevTools integration for bevy-react.
 *
 * ## What works today
 * - `injectIntoDevTools` so `__REACT_DEVTOOLS_GLOBAL_HOOK__` (if present) sees us
 * - Host tree dump via `__bevyReactDevTools.dump()`
 * - Fiber tree walk from reconciler roots
 * - WebSocket bridge on `:8098` emitting both legacy `type:` messages and
 *   React DevTools–shaped `{ event, payload }` envelopes
 *
 * ## What does *not* work yet (Boa constraints)
 * Full standalone `npx react-devtools` / Chrome extension needs
 * `react-devtools-core` (`initialize` + `connectToDevTools`) **before** React
 * loads, plus a faithful Int32 `operations` codec and browser-ish APIs.
 * Bundling that into Vite→Boa is unreliable (init order, missing DOM/eval
 * surfaces, heavy deps). Until that lands, this bridge speaks RDT *message
 * shapes* with a JSON fiber snapshot payload — not the binary operations
 * stream standalone DevTools expects on `:8097`.
 *
 * See docs/DEVTOOLS.md.
 */

import type { Fiber } from "react-reconciler";
import type { BevyReconciler, PublicInstance } from "./reconciler";
import type { BevyInstance, BevyTextInstance } from "./types";
import { listRoots, rootCount, type BevyRootState } from "./roots";

export const BEVY_REACT_DEVTOOLS_PORT = 8098;
export const BEVY_REACT_DEVTOOLS_DEFAULT_URL = `ws://127.0.0.1:${BEVY_REACT_DEVTOOLS_PORT}`;

/** RDT bridge protocol version we advertise (informational; not full RDT). */
export const BEVY_RDT_BRIDGE_PROTOCOL = {
  version: 2,
  minSupportedVersion: 2,
  maxSupportedVersion: 2,
  note: "bevy-react subset — JSON fiber snapshots, not Int32 operations",
} as const;

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

/** Fiber node as walked from the reconciler (RDT-inspired, JSON-friendly). */
export type FiberNodeSnapshot = {
  id: number;
  name: string;
  tag: number;
  key: string | null;
  typeName: string | null;
  childIds: number[];
  hooks?: unknown;
  props?: Record<string, unknown>;
};

export type FiberRootSnapshot = {
  rootId: string;
  rendererID: number;
  fibers: FiberNodeSnapshot[];
};

export type FiberTreeSnapshot = {
  version: 1;
  kind: "bevy-fiber-snapshot";
  roots: FiberRootSnapshot[];
  capturedAt: number;
};

/** RDT-shaped bridge envelope (`event` + `payload`). */
export type RdtBridgeEnvelope = {
  event: string;
  payload: unknown;
};

type DevToolsApi = {
  dump: () => DevToolsSnapshot;
  dumpJson: () => string;
  dumpFibers: () => FiberTreeSnapshot;
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
let nextFiberId = 1;
const RENDERER_ID = 1;

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

type FiberLike = {
  tag: number;
  key: null | string;
  elementType: unknown;
  type: unknown;
  memoizedProps: unknown;
  memoizedState: unknown;
  child: FiberLike | null;
  sibling: FiberLike | null;
  return: FiberLike | null;
};

type FiberRootLike = {
  current: FiberLike | null;
};

function typeDisplayName(type: unknown): string | null {
  if (type == null) return null;
  if (typeof type === "string") return type;
  if (typeof type === "function") {
    const fn = type as { displayName?: string; name?: string };
    return fn.displayName || fn.name || "Anonymous";
  }
  if (typeof type === "object") {
    const obj = type as { displayName?: string; name?: string; $$typeof?: symbol };
    if (obj.displayName) return obj.displayName;
    if (obj.name) return obj.name;
  }
  return String(type);
}

function fiberName(fiber: FiberLike): string {
  switch (fiber.tag) {
    case 3: // HostRoot
      return "Root";
    case 6: // HostText
      return "#text";
    case 7: // Fragment
      return "Fragment";
    default:
      return (
        typeDisplayName(fiber.type) ||
        typeDisplayName(fiber.elementType) ||
        `tag:${fiber.tag}`
      );
  }
}

/**
 * Walk reconciler fiber trees into a JSON snapshot (RDT-inspired).
 */
export function getFiberTreeSnapshot(): FiberTreeSnapshot {
  const roots: FiberRootSnapshot[] = [];
  for (const state of listRoots()) {
    roots.push(walkRootFibers(state));
  }
  return {
    version: 1,
    kind: "bevy-fiber-snapshot",
    roots,
    capturedAt: Date.now(),
  };
}

function walkRootFibers(state: BevyRootState): FiberRootSnapshot {
  const fibers: FiberNodeSnapshot[] = [];
  const idByFiber = new Map<FiberLike, number>();

  const fiberRoot = state.fiberRoot as unknown as FiberRootLike | null;
  const current = fiberRoot?.current ?? null;
  if (!current) {
    return { rootId: state.rootId, rendererID: RENDERER_ID, fibers };
  }

  const assignId = (fiber: FiberLike): number => {
    const existing = idByFiber.get(fiber);
    if (existing != null) return existing;
    const id = nextFiberId++;
    idByFiber.set(fiber, id);
    return id;
  };

  const visit = (fiber: FiberLike | null): number[] => {
    const ids: number[] = [];
    let node = fiber;
    while (node) {
      const id = assignId(node);
      ids.push(id);
      const childIds = visit(node.child);
      const props =
        node.memoizedProps && typeof node.memoizedProps === "object"
          ? serializeProps(node.memoizedProps as Record<string, unknown>)
          : undefined;
      fibers.push({
        id,
        name: fiberName(node),
        tag: node.tag,
        key: node.key,
        typeName: typeDisplayName(node.type),
        childIds,
        props,
      });
      node = node.sibling;
    }
    return ids;
  };

  visit(current);
  fibers.sort((a, b) => a.id - b.id);
  return { rootId: state.rootId, rendererID: RENDERER_ID, fibers };
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
    dumpFibers: getFiberTreeSnapshot,
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
    "[bevy-react] DevTools ready — dump via __bevyReactDevTools.dump() / dumpFibers(), " +
      `bridge ws://127.0.0.1:${BEVY_REACT_DEVTOOLS_PORT} (RDT-shaped events + legacy tree)`
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
    pushHandshake(ws);
    pushSnapshots(ws);
  };

  ws.onmessage = (event: { data?: unknown }) => {
    let raw = "";
    if (typeof event.data === "string") {
      raw = event.data;
    } else if (event.data != null) {
      raw = String(event.data);
    }
    try {
      const msg = JSON.parse(raw) as {
        type?: string;
        event?: string;
      };
      if (
        msg.type === "request_dump" ||
        msg.type === "ping" ||
        msg.event === "request_dump" ||
        msg.event === "ping"
      ) {
        pushSnapshots(ws);
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
      pushSnapshots(bridgeSocket);
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

function sendJson(ws: WebSocket, value: unknown): void {
  if (ws.readyState !== WebSocket.OPEN) return;
  try {
    ws.send(JSON.stringify(value));
  } catch (err) {
    console.warn("[bevy-react] Failed to push DevTools message:", err);
  }
}

function rdt(event: string, payload: unknown): RdtBridgeEnvelope {
  return { event, payload };
}

function pushHandshake(ws: WebSocket): void {
  sendJson(ws, rdt("backendVersion", "bevy-react@0.1.0"));
  sendJson(ws, rdt("bridgeProtocol", BEVY_RDT_BRIDGE_PROTOCOL));
  sendJson(
    ws,
    rdt("rendererAttached", {
      id: RENDERER_ID,
      rendererPackageName: "bevy-react",
      rendererVersion: "0.1.0",
      reactVersion: "19",
      bundleType: 1,
    })
  );
}

function pushSnapshots(ws: WebSocket): void {
  const hostTree = getDevToolsSnapshot();
  const fibers = getFiberTreeSnapshot();

  // Legacy bevy-react messages (type: …)
  sendJson(ws, { type: "tree", ...hostTree });
  sendJson(ws, { type: "fiber_tree", ...fibers });

  // RDT-shaped envelopes — payload is JSON fiber snapshot, not Int32 ops.
  sendJson(
    ws,
    rdt("operations", {
      compatibleWith: "react-devtools-bridge-event-shape-only",
      rendererID: RENDERER_ID,
      ...fibers,
    })
  );
}
