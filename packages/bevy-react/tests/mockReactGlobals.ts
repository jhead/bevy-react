/**
 * Mock harness for the Rust-injected `__react_*` globals used by the reconciler.
 * Call `installMockReactGlobals()` in `beforeEach` before exercising the reconciler.
 */

export type ReactCall =
  | {
      op: "create_node";
      rootId: string;
      type: string;
      propsJson: string;
      nodeId: number;
    }
  | { op: "create_text"; rootId: string; content: string; nodeId: number }
  | { op: "append_child"; rootId: string; parentId: number; childId: number }
  | {
      op: "insert_before";
      rootId: string;
      parentId: number;
      childId: number;
      beforeId: number;
    }
  | { op: "remove_child"; rootId: string; parentId: number; childId: number }
  | { op: "update_node"; rootId: string; nodeId: number; propsJson: string }
  | { op: "update_text"; rootId: string; nodeId: number; content: string }
  | { op: "destroy_node"; rootId: string; nodeId: number }
  | { op: "clear_container"; rootId: string }
  | { op: "commit_ops"; bytes: Uint8Array };

export interface MockReactGlobals {
  calls: ReactCall[];
  nextId: number;
  /** Node IDs still considered live (create − successful destroy). */
  liveIds: Set<number>;
  /** nodeId → how many times destroy was invoked (including no-ops). */
  destroyCounts: Map<number, number>;
  /** Decoded BRRP frames from `__react_commit_ops` (when present). */
  commitFrames: Uint8Array[];
  reset: () => void;
  ops: () => string[];
  /** Destroy calls where the node was already gone (idempotent second path). */
  duplicateDestroys: () => number[];
}

export function installMockReactGlobals(): MockReactGlobals {
  const calls: ReactCall[] = [];
  const liveIds = new Set<number>();
  const destroyCounts = new Map<number, number>();
  const commitFrames: Uint8Array[] = [];
  let nextId = 1;

  const g = globalThis as typeof globalThis & Record<string, unknown>;

  g.__react_create_node = (rootId: string, type: string, propsJson: string) => {
    const nodeId = nextId++;
    liveIds.add(nodeId);
    calls.push({ op: "create_node", rootId, type, propsJson, nodeId });
    return nodeId;
  };

  g.__react_create_text = (rootId: string, content: string) => {
    const nodeId = nextId++;
    liveIds.add(nodeId);
    calls.push({ op: "create_text", rootId, content, nodeId });
    return nodeId;
  };

  g.__react_append_child = (
    rootId: string,
    parentId: number,
    childId: number
  ) => {
    calls.push({ op: "append_child", rootId, parentId, childId });
  };

  g.__react_insert_before = (
    rootId: string,
    parentId: number,
    childId: number,
    beforeId: number
  ) => {
    calls.push({ op: "insert_before", rootId, parentId, childId, beforeId });
  };

  g.__react_remove_child = (
    rootId: string,
    parentId: number,
    childId: number
  ) => {
    calls.push({ op: "remove_child", rootId, parentId, childId });
  };

  g.__react_update_node = (
    rootId: string,
    nodeId: number,
    propsJson: string
  ) => {
    calls.push({ op: "update_node", rootId, nodeId, propsJson });
  };

  g.__react_update_text = (
    rootId: string,
    nodeId: number,
    content: string
  ) => {
    calls.push({ op: "update_text", rootId, nodeId, content });
  };

  g.__react_destroy_node = (rootId: string, nodeId: number) => {
    // Idempotent: removeChild and detachDeletedInstance both call destroy.
    const count = (destroyCounts.get(nodeId) ?? 0) + 1;
    destroyCounts.set(nodeId, count);
    liveIds.delete(nodeId);
    calls.push({ op: "destroy_node", rootId, nodeId });
  };

  g.__react_clear_container = (rootId: string) => {
    calls.push({ op: "clear_container", rootId });
  };

  g.__react_commit_ops = (bytes: Uint8Array | ArrayBuffer) => {
    const view =
      bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const copy = new Uint8Array(view);
    commitFrames.push(copy);
    calls.push({ op: "commit_ops", bytes: copy });
  };

  return {
    calls,
    liveIds,
    destroyCounts,
    commitFrames,
    get nextId() {
      return nextId;
    },
    set nextId(value: number) {
      nextId = value;
    },
    reset() {
      // Keep nextId monotonic so mid-session updates don't collide with
      // still-mounted fiber node IDs after clearing the call log.
      calls.length = 0;
      liveIds.clear();
      destroyCounts.clear();
      commitFrames.length = 0;
    },
    ops() {
      return calls.map((c) => c.op);
    },
    duplicateDestroys() {
      return [...destroyCounts.entries()]
        .filter(([, n]) => n > 1)
        .map(([id]) => id);
    },
  };
}
