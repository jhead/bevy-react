import { describe, it, expect } from "vitest";
import {
  BRRP_FLAG_STRING_TABLE,
  BRRP_MAGIC,
  BRRP_VERSION,
  decodeBatch,
  encodeBatch,
  type BinaryOp,
} from "../src/protocol";

/** Golden bytes from Rust `golden_inline_commit_frame` in codec.rs. */
const GOLDEN_INLINE = Uint8Array.of(
  // magic BRRP, version 1, flags 0
  0x42,
  0x52,
  0x52,
  0x50,
  0x01,
  0x00,
  0x00,
  0x00,
  // root_id "r"
  0x01,
  0x00,
  0x00,
  0x00,
  0x72,
  // op_count = 3
  0x03,
  0x00,
  0x00,
  0x00,
  // CreateNode id=1 type="view" props="{}"
  0x01,
  0x01,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x04,
  0x00,
  0x00,
  0x00,
  0x76,
  0x69,
  0x65,
  0x77,
  0x02,
  0x00,
  0x00,
  0x00,
  0x7b,
  0x7d,
  // AppendChild parent=0 child=1
  0x03,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x01,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  // Commit
  0x0a
);

const SAMPLE_OPS: BinaryOp[] = [
  {
    op: "CreateNode",
    nodeId: 1,
    nodeType: "view",
    propsJson: '{"style":{"flex":1}}',
  },
  { op: "CreateText", nodeId: 2, content: "hi" },
  { op: "AppendChild", parentId: 1, childId: 2 },
  { op: "InsertBefore", parentId: 1, childId: 3, beforeId: 2 },
  {
    op: "UpdateNode",
    nodeId: 1,
    propsJson: '{"style":{"opacity":0.5}}',
  },
  { op: "UpdateText", nodeId: 2, content: "hello" },
  { op: "RemoveChild", parentId: 1, childId: 3 },
  { op: "DestroyNode", nodeId: 3 },
  { op: "ClearContainer" },
  { op: "Commit" },
];

describe("BRRP protocol", () => {
  it("matches Rust MAGIC / VERSION constants", () => {
    expect(BRRP_MAGIC).toBe(0x50525242);
    expect(BRRP_VERSION).toBe(1);
    expect(BRRP_FLAG_STRING_TABLE).toBe(1);
  });

  it("encodes the Rust golden inline commit frame byte-for-byte", () => {
    const bytes = encodeBatch("r", [
      {
        op: "CreateNode",
        nodeId: 1,
        nodeType: "view",
        propsJson: "{}",
      },
      { op: "AppendChild", parentId: 0, childId: 1 },
      { op: "Commit" },
    ]);
    expect([...bytes]).toEqual([...GOLDEN_INLINE]);
  });

  it("round-trips common ops (inline strings)", () => {
    const bytes = encodeBatch("root-a", SAMPLE_OPS);
    expect(bytes[0]).toBe(0x42); // 'B'
    const { rootId, ops } = decodeBatch(bytes);
    expect(rootId).toBe("root-a");
    expect(ops).toEqual(SAMPLE_OPS);
  });

  it("round-trips with FLAG_STRING_TABLE", () => {
    const bytes = encodeBatch("root-a", SAMPLE_OPS, { stringTable: true });
    const flags = bytes[6]! | (bytes[7]! << 8);
    expect(flags).toBe(BRRP_FLAG_STRING_TABLE);
    const { rootId, ops } = decodeBatch(bytes);
    expect(rootId).toBe("root-a");
    expect(ops).toEqual(SAMPLE_OPS);
  });

  it("rejects bad magic and truncation", () => {
    expect(() => decodeBatch(Uint8Array.of(0x58, 0x58, 0x58, 0x58))).toThrow(
      /BadMagic|Truncated/
    );
    const bytes = encodeBatch("r", [{ op: "Commit" }]);
    expect(() => decodeBatch(bytes.subarray(0, 8))).toThrow(/Truncated/);
  });

  it("rejects empty root id on encode", () => {
    expect(() => encodeBatch("", [{ op: "Commit" }])).toThrow(/EmptyRootId/);
  });
});
