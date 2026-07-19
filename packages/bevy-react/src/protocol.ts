/**
 * BRRP (Bevy-React Render Protocol) — TypeScript stub.
 *
 * Documents the v1 binary frame layout for a future reconciler batch path.
 * Full encode/decode lives in Rust (`plugin/src/react/proto/`). See docs/PROTO.md.
 *
 * The default reconciler still calls per-op natives (`__react_create_node`, …).
 * With the host `binary_ops` feature, `__react_commit_ops(bytes)` accepts a
 * Uint8Array / ArrayBuffer built to this layout (declared in `global.d.ts`).
 */

/** Little-endian ASCII "BRRP". */
export const BRRP_MAGIC = 0x42525250;

export const BRRP_VERSION = 1;

/** Reserved: frame includes a string table (not implemented). */
export const BRRP_FLAG_STRING_TABLE = 1 << 0;

export const enum OpCode {
  CreateNode = 0x01,
  CreateText = 0x02,
  AppendChild = 0x03,
  InsertBefore = 0x04,
  RemoveChild = 0x05,
  UpdateNode = 0x06,
  UpdateText = 0x07,
  DestroyNode = 0x08,
  ClearContainer = 0x09,
  Commit = 0x0a,
}

/**
 * Frame sketch (not encoded by this stub):
 *
 * ```
 * magic:u32 | version:u16 | flags:u16 | root_id:String | op_count:u32 | ops…
 * String = len:u32 + utf8 bytes
 * ```
 *
 * @returns null — encoder not implemented yet; use Rust `encode_batch` in tests.
 */
export function encodeBatchStub(
  _rootId: string,
  _ops: ReadonlyArray<{ opcode: OpCode }>
): Uint8Array | null {
  return null;
}
