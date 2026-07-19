import { describe, it, expect, beforeEach } from "vitest";
import React from "react";
import { createBevyReconciler } from "../src/reconciler";
import {
  installMockReactGlobals,
  type MockReactGlobals,
} from "./mockReactGlobals";

const ROOT_ID = "test-root";
const CONTAINER = { rootId: 0 };

function renderTree(
  element: React.ReactNode,
  mock: MockReactGlobals
): { unmount: () => void } {
  mock.reset();
  const reconciler = createBevyReconciler({ rootId: ROOT_ID });
  // tag 0 historically meant LegacyRoot; react-reconciler 0.32 always creates a
  // concurrent root, so use updateContainerSync + flushSyncWork for tests.
  const fiberRoot = reconciler.createContainer(
    CONTAINER,
    0,
    null,
    false,
    null,
    "",
    () => {},
    null
  );

  reconciler.updateContainerSync(element, fiberRoot, null, null);
  reconciler.flushSyncWork();

  return {
    unmount() {
      mock.reset();
      reconciler.updateContainerSync(null, fiberRoot, null, null);
      reconciler.flushSyncWork();
    },
  };
}

describe("reconciler host config RPC sequences", () => {
  let mock: MockReactGlobals;

  beforeEach(() => {
    mock = installMockReactGlobals();
  });

  it("mounts a node tree and appends children via __react_* calls", () => {
    renderTree(
      React.createElement(
        "bevy-node",
        { style: { width: 100 } },
        React.createElement("bevy-text", { children: "Hello" })
      ),
      mock
    );

    const ops = mock.ops();
    expect(ops).toEqual([
      "create_node",
      "create_node",
      "append_child",
      "clear_container",
      "append_child",
    ]);

    const createCalls = mock.calls.filter((c) => c.op === "create_node");
    expect(createCalls).toHaveLength(2);

    const textCreate = createCalls.find((c) => c.type === "bevy-text");
    const nodeCreate = createCalls.find((c) => c.type === "bevy-node");
    expect(textCreate).toBeDefined();
    expect(nodeCreate).toBeDefined();
    expect(textCreate).toMatchObject({ rootId: ROOT_ID, type: "bevy-text" });
    expect(nodeCreate).toMatchObject({ rootId: ROOT_ID, type: "bevy-node" });

    if (textCreate && textCreate.op === "create_node") {
      const props = JSON.parse(textCreate.propsJson) as { content?: string };
      expect(props.content).toBe("Hello");
    }

    const appends = mock.calls.filter((c) => c.op === "append_child");
    expect(appends).toHaveLength(2);
    // Child text appended under parent node, then parent appended to container rootId 0
    expect(appends[0]).toMatchObject({
      op: "append_child",
      rootId: ROOT_ID,
      parentId: nodeCreate!.nodeId,
      childId: textCreate!.nodeId,
    });
    expect(appends[1]).toMatchObject({
      op: "append_child",
      rootId: ROOT_ID,
      parentId: 0,
      childId: nodeCreate!.nodeId,
    });
  });

  it("unmounts by issuing remove_child for the container child", () => {
    const { unmount } = renderTree(
      React.createElement(
        "bevy-button",
        { onClick: () => {} },
        React.createElement("bevy-text", { children: "Click" })
      ),
      mock
    );

    expect(mock.ops()).toContain("create_node");
    expect(mock.ops()).toContain("append_child");

    const mountedRootChild = mock.calls.find(
      (c) => c.op === "append_child" && c.parentId === 0
    );
    expect(mountedRootChild).toBeDefined();
    if (!mountedRootChild || mountedRootChild.op !== "append_child") {
      throw new Error("expected root append_child");
    }

    unmount();

    const unmountOps = mock.ops();
    expect(unmountOps[0]).toBe("remove_child");
    expect(unmountOps).toContain("destroy_node");
    expect(unmountOps.filter((op) => op === "destroy_node").length).toBeGreaterThanOrEqual(1);

    expect(mock.calls[0]).toMatchObject({
      op: "remove_child",
      rootId: ROOT_ID,
      parentId: 0,
      childId: mountedRootChild.childId,
    });

    const destroys = mock.calls.filter((c) => c.op === "destroy_node");
    expect(destroys.some((c) => c.nodeId === mountedRootChild.childId)).toBe(
      true
    );
  });
});
