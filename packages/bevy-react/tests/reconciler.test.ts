import { describe, it, expect, beforeEach } from "vitest";
import React from "react";
import {
  createBevyReconciler,
  type BevyInstanceMap,
} from "../src/reconciler";
import {
  installMockReactGlobals,
  type MockReactGlobals,
} from "./mockReactGlobals";

const ROOT_ID = "test-root";
const CONTAINER = { rootId: 0 };

type Renderer = {
  render: (element: React.ReactNode) => void;
  unmount: () => void;
};

function createRenderer(mock: MockReactGlobals): Renderer {
  mock.reset();
  const instanceMap: BevyInstanceMap = new Map();
  const reconciler = createBevyReconciler({ rootId: ROOT_ID, instanceMap });
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

  return {
    render(element: React.ReactNode) {
      reconciler.updateContainerSync(element, fiberRoot, null, null);
      reconciler.flushSyncWork();
    },
    unmount() {
      reconciler.updateContainerSync(null, fiberRoot, null, null);
      reconciler.flushSyncWork();
    },
  };
}

function listTree(items: { id: string; label: string }[]) {
  return React.createElement(
    "bevy-node",
    { style: { flexDirection: "column" } },
    ...items.map((item) =>
      React.createElement(
        "bevy-node",
        { key: item.id, style: { width: 20 } },
        React.createElement("bevy-text", { children: item.label })
      )
    )
  );
}

describe("reconciler host config RPC sequences", () => {
  let mock: MockReactGlobals;

  beforeEach(() => {
    mock = installMockReactGlobals();
  });

  it("mounts a node tree and appends children via __react_* calls", () => {
    const { render } = createRenderer(mock);
    render(
      React.createElement(
        "bevy-node",
        { style: { width: 100 } },
        React.createElement("bevy-text", { children: "Hello" })
      )
    );

    const ops = mock.ops();
    // Concurrent root used to emit clear_container mid-mount (which despawned the
    // tree). clearContainer is now a no-op host method — no RPC.
    expect(ops).toEqual([
      "create_node",
      "create_node",
      "append_child",
      "append_child",
    ]);
    expect(ops).not.toContain("clear_container");

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

  it("unmounts by issuing remove_child + destroy_node", () => {
    const { render, unmount } = createRenderer(mock);
    render(
      React.createElement(
        "bevy-button",
        { onClick: () => {} },
        React.createElement("bevy-text", { children: "Click" })
      )
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

    mock.reset();
    unmount();

    const unmountOps = mock.ops();
    expect(unmountOps[0]).toBe("remove_child");
    expect(unmountOps).toContain("destroy_node");
    expect(
      unmountOps.filter((op) => op === "destroy_node").length
    ).toBeGreaterThanOrEqual(1);

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

  it("updates props via __react_update_node and skips unchanged props", () => {
    const { render } = createRenderer(mock);
    render(
      React.createElement("bevy-node", {
        style: { width: 100, backgroundColor: "#111111" },
      })
    );

    const created = mock.calls.find(
      (c) => c.op === "create_node" && c.type === "bevy-node"
    );
    expect(created).toBeDefined();
    if (!created || created.op !== "create_node") {
      throw new Error("expected create_node");
    }

    mock.reset();
    render(
      React.createElement("bevy-node", {
        style: { width: 200, backgroundColor: "#111111" },
      })
    );

    expect(mock.ops()).toEqual(["update_node"]);
    expect(mock.calls[0]).toMatchObject({
      op: "update_node",
      rootId: ROOT_ID,
      nodeId: created.nodeId,
    });
    if (mock.calls[0].op === "update_node") {
      const props = JSON.parse(mock.calls[0].propsJson) as {
        style?: { width?: number };
      };
      expect(props.style?.width).toBe(200);
    }

    mock.reset();
    render(
      React.createElement("bevy-node", {
        style: { width: 200, backgroundColor: "#111111" },
      })
    );
    expect(mock.ops()).toEqual([]);
  });

  it("updates bevy-text content via __react_update_node", () => {
    const { render } = createRenderer(mock);
    render(React.createElement("bevy-text", { children: "one" }));

    const created = mock.calls.find(
      (c) => c.op === "create_node" && c.type === "bevy-text"
    );
    expect(created).toBeDefined();
    if (!created || created.op !== "create_node") {
      throw new Error("expected create_node");
    }

    mock.reset();
    render(React.createElement("bevy-text", { children: "two" }));

    expect(mock.ops()).toEqual(["update_node"]);
    if (mock.calls[0].op === "update_node") {
      const props = JSON.parse(mock.calls[0].propsJson) as {
        content?: string;
      };
      expect(props.content).toBe("two");
      expect(mock.calls[0].nodeId).toBe(created.nodeId);
    }
  });

  it("updates numeric text children (HUD clock shape)", () => {
    const { render } = createRenderer(mock);
    render(React.createElement("bevy-text", { children: 1444 }));

    const created = mock.calls.find(
      (c) => c.op === "create_node" && c.type === "bevy-text"
    );
    expect(created).toBeDefined();
    if (!created || created.op !== "create_node") {
      throw new Error("expected create_node");
    }
    const initial = JSON.parse(created.propsJson) as { content?: string };
    expect(initial.content).toBe("1444");

    mock.reset();
    render(React.createElement("bevy-text", { children: 1445 }));

    // Must update the bevy-text host via update_node — not a sibling update_text.
    expect(mock.ops()).toEqual(["update_node"]);
    expect(mock.ops()).not.toContain("update_text");
    if (mock.calls[0].op === "update_node") {
      const props = JSON.parse(mock.calls[0].propsJson) as {
        content?: string;
      };
      expect(props.content).toBe("1445");
      expect(mock.calls[0].nodeId).toBe(created.nodeId);
    }
  });

  it("updates single-expression string text on the same bevy-text node", () => {
    const { render } = createRenderer(mock);
    render(React.createElement("bevy-text", null, String(10)));

    const created = mock.calls.find(
      (c) => c.op === "create_node" && c.type === "bevy-text"
    );
    expect(created).toBeDefined();
    if (!created || created.op !== "create_node") {
      throw new Error("expected create_node");
    }

    mock.reset();
    render(React.createElement("bevy-text", null, String(11)));

    expect(mock.ops()).toContain("update_node");
    expect(mock.ops()).not.toContain("update_text");
    const update = mock.calls.find((c) => c.op === "update_node");
    expect(update).toMatchObject({ nodeId: created.nodeId });
    if (update && update.op === "update_node") {
      expect(JSON.parse(update.propsJson)).toMatchObject({ content: "11" });
    }
  });

  it("reorders keyed children via append_child (move to end)", () => {
    const { render } = createRenderer(mock);
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "a", children: "A" }),
        React.createElement("bevy-text", { key: "b", children: "B" })
      )
    );

    const creates = mock.calls.filter((c) => c.op === "create_node");
    const parent = creates.find((c) => c.type === "bevy-node");
    const textCreates = creates.filter((c) => c.type === "bevy-text");
    expect(parent).toBeDefined();
    expect(textCreates).toHaveLength(2);
    if (!parent || parent.op !== "create_node") {
      throw new Error("expected parent create_node");
    }

    const idA = textCreates[0]!.nodeId;

    mock.reset();
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "b", children: "B" }),
        React.createElement("bevy-text", { key: "a", children: "A" })
      )
    );

    expect(mock.ops()).toEqual(["append_child"]);
    expect(mock.calls[0]).toMatchObject({
      op: "append_child",
      rootId: ROOT_ID,
      parentId: parent.nodeId,
      childId: idA,
    });
  });

  it("inserts a new keyed sibling with insert_before", () => {
    const { render } = createRenderer(mock);
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "a", children: "A" }),
        React.createElement("bevy-text", { key: "c", children: "C" })
      )
    );

    const creates = mock.calls.filter((c) => c.op === "create_node");
    const parent = creates.find((c) => c.type === "bevy-node");
    const textCreates = creates.filter((c) => c.type === "bevy-text");
    expect(parent).toBeDefined();
    expect(textCreates).toHaveLength(2);
    if (!parent || parent.op !== "create_node") {
      throw new Error("expected parent create_node");
    }

    const idC = textCreates.find((c) => {
      if (c.op !== "create_node") return false;
      const props = JSON.parse(c.propsJson) as { content?: string };
      return props.content === "C";
    })!.nodeId;

    mock.reset();
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "a", children: "A" }),
        React.createElement("bevy-text", { key: "b", children: "B" }),
        React.createElement("bevy-text", { key: "c", children: "C" })
      )
    );

    expect(mock.ops()).toContain("create_node");
    expect(mock.ops()).toContain("insert_before");
    const insert = mock.calls.find((c) => c.op === "insert_before");
    const createdB = mock.calls.find(
      (c) => c.op === "create_node" && c.type === "bevy-text"
    );
    expect(createdB).toBeDefined();
    expect(insert).toMatchObject({
      op: "insert_before",
      rootId: ROOT_ID,
      parentId: parent.nodeId,
      childId: createdB!.nodeId,
      beforeId: idC,
    });
  });

  it("destroys removed subtrees on update (remove + destroy)", () => {
    const { render } = createRenderer(mock);
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "keep", children: "keep" }),
        React.createElement("bevy-text", { key: "gone", children: "gone" })
      )
    );

    const textCreates = mock.calls.filter(
      (c) => c.op === "create_node" && c.type === "bevy-text"
    );
    expect(textCreates).toHaveLength(2);
    const gone = textCreates.find((c) => {
      if (c.op !== "create_node") return false;
      const props = JSON.parse(c.propsJson) as { content?: string };
      return props.content === "gone";
    });
    expect(gone).toBeDefined();
    if (!gone || gone.op !== "create_node") {
      throw new Error("expected gone create_node");
    }

    mock.reset();
    render(
      React.createElement(
        "bevy-node",
        null,
        React.createElement("bevy-text", { key: "keep", children: "keep" })
      )
    );

    expect(mock.ops()).toContain("remove_child");
    expect(mock.ops()).toContain("destroy_node");
    expect(
      mock.calls.some(
        (c) => c.op === "destroy_node" && c.nodeId === gone.nodeId
      )
    ).toBe(true);
  });

  it("list shuffle + delete triggers removeChild, reorder, and duplicate destroy", () => {
    const { render } = createRenderer(mock);

    const initial = [
      { id: "a", label: "A" },
      { id: "b", label: "B" },
      { id: "c", label: "C" },
      { id: "d", label: "D" },
    ];
    render(listTree(initial));

    const createsBefore = mock.calls.filter(
      (c) => c.op === "create_node" && c.type === "bevy-node"
    );
    expect(createsBefore.length).toBeGreaterThanOrEqual(5);

    mock.reset();

    const next = [
      { id: "d", label: "D" },
      { id: "c", label: "C" },
      { id: "a", label: "A" },
      { id: "e", label: "E" },
    ];
    expect(() => render(listTree(next))).not.toThrow();

    const ops = mock.ops();
    expect(ops).toContain("remove_child");
    expect(ops).toContain("destroy_node");
    expect(
      ops.includes("insert_before") || ops.includes("append_child")
    ).toBe(true);

    const duplicateIds = mock.duplicateDestroys();
    expect(duplicateIds.length).toBeGreaterThanOrEqual(1);

    for (const id of duplicateIds) {
      expect(mock.destroyCounts.get(id)).toBeGreaterThanOrEqual(2);
      expect(mock.liveIds.has(id)).toBe(false);
    }

    const destroyCalls = mock.calls.filter((c) => c.op === "destroy_node");
    expect(destroyCalls.length).toBeGreaterThanOrEqual(2);
  });
});
