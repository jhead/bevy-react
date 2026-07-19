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
  | { op: "clear_container"; rootId: string };

export interface MockReactGlobals {
  calls: ReactCall[];
  nextId: number;
  reset: () => void;
  ops: () => string[];
}

export function installMockReactGlobals(): MockReactGlobals {
  const calls: ReactCall[] = [];
  let nextId = 1;

  const g = globalThis as typeof globalThis & Record<string, unknown>;

  g.__react_create_node = (rootId: string, type: string, propsJson: string) => {
    const nodeId = nextId++;
    calls.push({ op: "create_node", rootId, type, propsJson, nodeId });
    return nodeId;
  };

  g.__react_create_text = (rootId: string, content: string) => {
    const nodeId = nextId++;
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
    calls.push({ op: "destroy_node", rootId, nodeId });
  };

  g.__react_clear_container = (rootId: string) => {
    calls.push({ op: "clear_container", rootId });
  };

  return {
    calls,
    get nextId() {
      return nextId;
    },
    set nextId(value: number) {
      nextId = value;
    },
    reset() {
      calls.length = 0;
      nextId = 1;
    },
    ops() {
      return calls.map((c) => c.op);
    },
  };
}
