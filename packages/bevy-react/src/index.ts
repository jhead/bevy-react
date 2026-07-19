import type { ReactNode } from "react";

// Import types module for side-effects (registers JSX global augmentation)
import "./types";

import {
  installBridgeDispatcher,
} from "./bridge";
import { installEventDispatcher, hostDispatchEvent } from "./events";
import { dispatchEvent } from "./reconciler";
import {
  ensureRoot,
  getInstance,
  getRoot,
  renderRoot,
  rootCount,
  setInstanceLookup,
  unmountRoot,
} from "./roots";

// Re-export types for consumers
export * from "./types";

// Export Bevy UI components
export * from "./components";

export { installEventDispatcher, hostDispatchEvent, requestFocus, requestBlur } from "./events";
export type {
  HostEventPayload,
  KeyboardEventPayload,
  PointerEventPayload,
  ScrollEventPayload,
  WheelEventPayload,
} from "./events";

export {
  callNative,
  getBridgeState,
  hostDispatchBridge,
  hostResolveBridgeCall,
  installBridgeDispatcher,
  subscribeBridge,
  useBridgeState,
  useQuery,
  useResource,
} from "./bridge";

export {
  ensureRoot,
  getInstance,
  getRoot,
  listRoots,
  renderRoot,
  rootCount,
  unmountRoot,
} from "./roots";

export {
  BevyErrorBoundary,
  reportErrorToHost,
  withErrorBoundary,
} from "./ErrorBoundary";
export type { ReportErrorOptions } from "./ErrorBoundary";

export {
  entityFromBits,
  resolveEntity,
  useEntity,
  useEntityRef,
} from "./entity";
export type { BevyHostInstance, EntityId } from "./entity";

export {
  BEVY_REACT_DEVTOOLS_DEFAULT_URL,
  BEVY_REACT_DEVTOOLS_PORT,
  BEVY_RDT_BRIDGE_PROTOCOL,
  connectDevToolsBridge,
  disconnectDevToolsBridge,
  getDevToolsSnapshot,
  getDevToolsSnapshotJson,
  getFiberTreeSnapshot,
  injectBevyReactDevTools,
} from "./devtools";
export type {
  DevToolsNodeSnapshot,
  DevToolsRootSnapshot,
  DevToolsSnapshot,
  FiberNodeSnapshot,
  FiberRootSnapshot,
  FiberTreeSnapshot,
  RdtBridgeEnvelope,
} from "./devtools";

export {
  BRRP_FLAG_STRING_TABLE,
  BRRP_MAGIC,
  BRRP_VERSION,
  encodeBatchStub,
  OpCode,
} from "./protocol";

export interface BevyReactApp {
  render: (rootId: string) => void;
  dispatchEvent: typeof dispatchEvent;
  unmount: (rootId: string) => void;
}

// Wire root-scoped instance lookup once at module load.
setInstanceLookup(getInstance);

/**
 * Render a React element tree to Bevy UI for `rootId`.
 */
function render(element: ReactNode, rootId: string): void {
  installEventDispatcher();
  installBridgeDispatcher();
  renderRoot(element, rootId);
}

export function createBevyApp(element: ReactNode): BevyReactApp {
  installEventDispatcher();
  installBridgeDispatcher();
  return {
    dispatchEvent,
    render: (rootId: string) => render(element, rootId),
    unmount: (rootId: string) => unmountRoot(rootId),
  };
}
