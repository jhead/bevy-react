import type { ReactNode } from "react";
import { injectBevyReactDevTools } from "./devtools";
import { reportErrorToHost, withErrorBoundary } from "./ErrorBoundary";
import {
  bindEventReconciler,
  createBevyReconciler,
  setInstanceLookup,
  type BevyInstanceMap,
  type BevyReconciler,
  type PublicInstance,
} from "./reconciler";

/** Host container: parent id 0 maps to the Bevy ReactRoot entity. */
export type HostContainer = { rootId: number };

export type BevyRootOptions = {
  /**
   * Batch commits into BRRP via `__react_commit_ops`.
   * When omitted: honors `__BEVY_REACT_BINARY_OPS`, else auto-detects
   * `typeof __react_commit_ops === "function"`. Pass `false` to force enum.
   */
  binaryOps?: boolean;
};

export type BevyRootState = {
  rootId: string;
  container: HostContainer;
  reconciler: BevyReconciler;
  fiberRoot: ReturnType<BevyReconciler["createContainer"]>;
  instanceMap: BevyInstanceMap;
  binaryOps?: boolean;
};

const roots = new Map<string, BevyRootState>();

/**
 * Get an existing root's state, if mounted.
 */
export function getRoot(rootId: string): BevyRootState | undefined {
  return roots.get(rootId);
}

/**
 * Look up a host instance by node id within a specific root.
 */
export function getInstance(
  rootId: string,
  nodeId: number
): PublicInstance | undefined {
  return roots.get(rootId)?.instanceMap.get(nodeId);
}

/** Snapshot of all mounted roots (DevTools / tests). */
export function listRoots(): BevyRootState[] {
  return Array.from(roots.values());
}

export { setInstanceLookup };

/**
 * Ensure a fiber container + instance map exist for `rootId`.
 * Creates them on first use; subsequent calls reuse the same state.
 */
export function ensureRoot(
  rootId: string,
  options?: BevyRootOptions
): BevyRootState {
  const existing = roots.get(rootId);
  if (existing) {
    return existing;
  }

  const instanceMap: BevyInstanceMap = new Map();
  const reconciler = createBevyReconciler({
    rootId,
    instanceMap,
    binaryOps: options?.binaryOps,
  });
  // Host container parent id is always 0 (the Bevy ReactRoot entity for this root).
  const container: HostContainer = { rootId: 0 };

  const fiberRoot = reconciler.createContainer(
    container,
    0,
    null, // hydration callbacks
    false, // isStrictMode
    null, // concurrentUpdatesByDefaultOverride
    "", // identifierPrefix
    (error) => {
      // Loud by default — silent recoverable errors made blank screens undebugable.
      reportErrorToHost(error);
    },
    null // transitionCallbacks
  );

  injectBevyReactDevTools(reconciler);
  // Keyboard/click setState must run as discrete + flushSyncWork.
  bindEventReconciler(reconciler);

  const state: BevyRootState = {
    rootId,
    container,
    reconciler,
    fiberRoot,
    instanceMap,
    binaryOps: options?.binaryOps,
  };
  roots.set(rootId, state);
  return state;
}

/**
 * Mount or update the React element tree for a root.
 */
export function renderRoot(
  element: ReactNode,
  rootId: string,
  options?: BevyRootOptions
): void {
  const root = ensureRoot(rootId, options);
  root.reconciler.updateContainer(
    withErrorBoundary(element),
    root.fiberRoot,
    null,
    () => {
      console.log("[bevy-react] Render complete for root", rootId);
    }
  );
}

/**
 * Unmount the React tree for `rootId` and drop fiber / instance state.
 * Safe to call if the root was never mounted.
 */
export function unmountRoot(rootId: string): void {
  const root = roots.get(rootId);
  if (!root) {
    return;
  }

  // Unmount through the reconciler so effects clean up. Host destroy RPCs may
  // no-op if the Bevy ReactRoot entity is already gone — that is intentional.
  root.reconciler.updateContainer(null, root.fiberRoot, null, () => {
    console.log("[bevy-react] Unmount complete for root", rootId);
  });

  // react-reconciler 0.32 exposes flushSyncWork at runtime; types omit it.
  const flushSyncWork = (
    root.reconciler as BevyReconciler & { flushSyncWork?: () => void }
  ).flushSyncWork;
  flushSyncWork?.();

  root.instanceMap.clear();
  roots.delete(rootId);
}

/**
 * Number of currently tracked roots (for tests / debugging).
 */
export function rootCount(): number {
  return roots.size;
}
