import type { ReactNode } from "react";

// Import types module for side-effects (registers JSX global augmentation)
import "./types";

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

export { installEventDispatcher, hostDispatchEvent } from "./events";
export type {
  HostEventPayload,
  KeyboardEventPayload,
  PointerEventPayload,
  ScrollEventPayload,
  WheelEventPayload,
} from "./events";

export {
  ensureRoot,
  getInstance,
  getRoot,
  renderRoot,
  rootCount,
  unmountRoot,
} from "./roots";

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
  renderRoot(element, rootId);
}

export function createBevyApp(element: ReactNode): BevyReactApp {
  installEventDispatcher();
  return {
    dispatchEvent,
    render: (rootId: string) => render(element, rootId),
    unmount: (rootId: string) => unmountRoot(rootId),
  };
}
