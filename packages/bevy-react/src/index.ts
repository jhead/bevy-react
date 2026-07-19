import type { ReactNode } from "react";
import { createBevyReconciler, dispatchEvent, type BevyReconciler } from "./reconciler";
import { installEventDispatcher } from "./events";

// Import types module for side-effects (registers JSX global augmentation)
import "./types";
import React from "react";
import { Fiber } from "react-reconciler";

// Re-export types for consumers
export * from "./types";

// Export Bevy UI components
export *from "./components";

export { installEventDispatcher, hostDispatchEvent } from "./events";
export type {
  HostEventPayload,
  KeyboardEventPayload,
  PointerEventPayload,
} from "./events";

export interface BevyReactApp {
  render: (rootId: string) => void;
  dispatchEvent: (nodeId: number, event: string) => void;
}

/**
 * Root container for the React tree.
 * Uses rootId = 0 to represent the Bevy UI root entity.
 */
const container = { rootId: 0 };

/**
 * The fiber root, lazily initialized
 */
let fiberRoot: ReturnType<BevyReconciler["createContainer"]> | null = null;

/**
 * Render a React element tree to Bevy UI.
 */
function render(element: ReactNode, rootId: string): void {
  // Ensure the host→JS callback is registered (natives exist by the time render runs).
  installEventDispatcher();

  const reconciler = createBevyReconciler({ rootId });
  log("Created reconciler with rootId", rootId);

  if (!fiberRoot) {
    fiberRoot = reconciler.createContainer(
      container,
      0,
      null, // hydration callbacks
      false, // isStrictMode
      null, // concurrentUpdatesByDefaultOverride
      "", // identifierPrefix
      () => {}, // onRecoverableError
      null // transitionCallbacks
    );
    console.log('Created fiber root');
  }

  reconciler.updateContainer(element, fiberRoot, null, () => {
    log("Initial render complete");
  });

  reconciler.injectIntoDevTools({
    bundleType: 1,
    version: "1",
    rendererPackageName: "bevy-react",
    findFiberByHostInstance: (instance: unknown) => instance as unknown as Fiber | null,
    rendererConfig: {
      supportsMutation: true,
      supportsPersistence: false,
      supportsHydration: false,
    },
  });
}

export function createBevyApp(element: ReactNode): BevyReactApp {
  installEventDispatcher();
  return {
    dispatchEvent,
    render: (rootId: string) => render(element, rootId),
   };
}

/**
 * Log helper
 */
function log(...args: unknown[]): void {
  console.log("[bevy-react]", ...args);
}
