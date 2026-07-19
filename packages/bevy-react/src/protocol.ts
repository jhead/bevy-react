/**
 * BRRP (Bevy-React Render Protocol) — TypeScript encode/decode.
 *
 * Wire layout must match `plugin/src/react/proto/codec.rs` exactly.
 * See docs/PROTO.md.
 *
 * When `__react_commit_ops` is present (host `--features binary_ops`), the
 * reconciler defaults to BRRP batches. Force the enum path with
 * `binaryOps: false` or `__BEVY_REACT_BINARY_OPS = 0`.
 */

/** Little-endian ASCII "BRRP" (`B` at the low byte). Matches Rust `MAGIC`. */
export const BRRP_MAGIC = 0x50525242;

export const BRRP_VERSION = 1;

/** Frame includes a string table; string fields are StringRef (u32 index). */
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

/** One mutation inside a batch. `rootId` lives on the frame, not per op. */
export type BinaryOp =
  | {
      op: "CreateNode";
      nodeId: number;
      nodeType: string;
      propsJson: string;
    }
  | { op: "CreateText"; nodeId: number; content: string }
  | { op: "AppendChild"; parentId: number; childId: number }
  | {
      op: "InsertBefore";
      parentId: number;
      childId: number;
      beforeId: number;
    }
  | { op: "RemoveChild"; parentId: number; childId: number }
  | { op: "UpdateNode"; nodeId: number; propsJson: string }
  | { op: "UpdateText"; nodeId: number; content: string }
  | { op: "DestroyNode"; nodeId: number }
  | { op: "ClearContainer" }
  | { op: "Commit" };

export type EncodeBatchOptions = {
  /** When true, set FLAG_STRING_TABLE and intern repeated strings. */
  stringTable?: boolean;
};

export class EncodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "EncodeError";
  }
}

export class DecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DecodeError";
  }
}

/**
 * Resolve whether the reconciler should batch commits into BRRP.
 *
 * Priority:
 * 1. Explicit `binaryOps` option
 * 2. Explicit `globalThis.__BEVY_REACT_BINARY_OPS` when set
 *    (`true` / `1` / `"1"` → on; anything else → off)
 * 3. Auto-detect: `typeof __react_commit_ops === "function"`
 * 4. Otherwise false (enum natives)
 */
export function isBinaryOpsEnabled(option?: boolean): boolean {
  if (option === true) return true;
  if (option === false) return false;
  const g = globalThis as typeof globalThis & {
    __BEVY_REACT_BINARY_OPS?: unknown;
    __react_commit_ops?: unknown;
  };
  if (Object.prototype.hasOwnProperty.call(g, "__BEVY_REACT_BINARY_OPS")) {
    const v = g.__BEVY_REACT_BINARY_OPS;
    if (v === undefined) {
      // fall through to auto-detect
    } else {
      return v === true || v === 1 || v === "1";
    }
  }
  return typeof g.__react_commit_ops === "function";
}

/** @deprecated Use {@link encodeBatch}. Kept for older imports. */
export function encodeBatchStub(
  rootId: string,
  ops: ReadonlyArray<BinaryOp>
): Uint8Array {
  return encodeBatch(rootId, ops);
}

/**
 * Encode a BRRP v1 frame for `rootId` containing `ops`.
 */
export function encodeBatch(
  rootId: string,
  ops: ReadonlyArray<BinaryOp>,
  options?: EncodeBatchOptions
): Uint8Array {
  if (rootId.length === 0) {
    throw new EncodeError("EmptyRootId");
  }

  const useTable = options?.stringTable === true;
  if (useTable) {
    return encodeBatchWithStringTable(rootId, ops);
  }
  return encodeBatchInline(rootId, ops);
}

/**
 * Decode a BRRP v1 frame into `{ rootId, ops }`.
 */
export function decodeBatch(bytes: Uint8Array): {
  rootId: string;
  ops: BinaryOp[];
} {
  const cur = new Cursor(bytes);
  const magic = cur.readU32();
  if (magic !== BRRP_MAGIC) {
    throw new DecodeError(`BadMagic(${magic})`);
  }
  const version = cur.readU16();
  if (version !== BRRP_VERSION) {
    throw new DecodeError(`UnsupportedVersion(${version})`);
  }
  const flags = cur.readU16();
  const unknown = flags & ~BRRP_FLAG_STRING_TABLE;
  if (unknown !== 0) {
    throw new DecodeError(`UnsupportedFlags(${flags})`);
  }

  let table: string[] | null = null;
  if (flags & BRRP_FLAG_STRING_TABLE) {
    const count = cur.readU32();
    table = [];
    for (let i = 0; i < count; i++) {
      table.push(cur.readInlineString());
    }
    if (table.length === 0) {
      throw new DecodeError("EmptyStringTable");
    }
  }

  const rootId = readStringRef(cur, table);
  const opCount = cur.readU32();
  const ops: BinaryOp[] = [];
  for (let i = 0; i < opCount; i++) {
    ops.push(readOp(cur, table));
  }
  if (!cur.isEmpty()) {
    throw new DecodeError("TrailingBytes");
  }
  return { rootId, ops };
}

// ---------------------------------------------------------------------------
// Encode helpers
// ---------------------------------------------------------------------------

function encodeBatchInline(
  rootId: string,
  ops: ReadonlyArray<BinaryOp>
): Uint8Array {
  const w = new Writer();
  w.writeU32(BRRP_MAGIC);
  w.writeU16(BRRP_VERSION);
  w.writeU16(0); // flags
  w.writeInlineString(rootId);
  w.writeU32(ops.length);
  for (const op of ops) {
    writeOpInline(w, op);
  }
  return w.toUint8Array();
}

function encodeBatchWithStringTable(
  rootId: string,
  ops: ReadonlyArray<BinaryOp>
): Uint8Array {
  const intern = new StringInterner();
  // Pass 1: collect every string that will appear as a StringRef.
  intern.intern(rootId);
  for (const op of ops) {
    collectOpStrings(op, intern);
  }

  const w = new Writer();
  w.writeU32(BRRP_MAGIC);
  w.writeU16(BRRP_VERSION);
  w.writeU16(BRRP_FLAG_STRING_TABLE);
  w.writeU32(intern.table.length);
  for (const entry of intern.table) {
    w.writeInlineString(entry);
  }
  writeStringRef(w, rootId, intern);
  w.writeU32(ops.length);
  for (const op of ops) {
    writeOpWithTable(w, op, intern);
  }
  return w.toUint8Array();
}

function collectOpStrings(op: BinaryOp, intern: StringInterner): void {
  switch (op.op) {
    case "CreateNode":
      intern.intern(op.nodeType);
      intern.intern(op.propsJson);
      break;
    case "CreateText":
      intern.intern(op.content);
      break;
    case "UpdateNode":
      intern.intern(op.propsJson);
      break;
    case "UpdateText":
      intern.intern(op.content);
      break;
    default:
      break;
  }
}

function writeOpInline(w: Writer, op: BinaryOp): void {
  switch (op.op) {
    case "CreateNode":
      w.writeU8(OpCode.CreateNode);
      w.writeU64(op.nodeId);
      w.writeInlineString(op.nodeType);
      w.writeInlineString(op.propsJson);
      break;
    case "CreateText":
      w.writeU8(OpCode.CreateText);
      w.writeU64(op.nodeId);
      w.writeInlineString(op.content);
      break;
    case "AppendChild":
      w.writeU8(OpCode.AppendChild);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      break;
    case "InsertBefore":
      w.writeU8(OpCode.InsertBefore);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      w.writeU64(op.beforeId);
      break;
    case "RemoveChild":
      w.writeU8(OpCode.RemoveChild);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      break;
    case "UpdateNode":
      w.writeU8(OpCode.UpdateNode);
      w.writeU64(op.nodeId);
      w.writeInlineString(op.propsJson);
      break;
    case "UpdateText":
      w.writeU8(OpCode.UpdateText);
      w.writeU64(op.nodeId);
      w.writeInlineString(op.content);
      break;
    case "DestroyNode":
      w.writeU8(OpCode.DestroyNode);
      w.writeU64(op.nodeId);
      break;
    case "ClearContainer":
      w.writeU8(OpCode.ClearContainer);
      break;
    case "Commit":
      w.writeU8(OpCode.Commit);
      break;
  }
}

function writeOpWithTable(
  w: Writer,
  op: BinaryOp,
  intern: StringInterner
): void {
  switch (op.op) {
    case "CreateNode":
      w.writeU8(OpCode.CreateNode);
      w.writeU64(op.nodeId);
      writeStringRef(w, op.nodeType, intern);
      writeStringRef(w, op.propsJson, intern);
      break;
    case "CreateText":
      w.writeU8(OpCode.CreateText);
      w.writeU64(op.nodeId);
      writeStringRef(w, op.content, intern);
      break;
    case "AppendChild":
      w.writeU8(OpCode.AppendChild);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      break;
    case "InsertBefore":
      w.writeU8(OpCode.InsertBefore);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      w.writeU64(op.beforeId);
      break;
    case "RemoveChild":
      w.writeU8(OpCode.RemoveChild);
      w.writeU64(op.parentId);
      w.writeU64(op.childId);
      break;
    case "UpdateNode":
      w.writeU8(OpCode.UpdateNode);
      w.writeU64(op.nodeId);
      writeStringRef(w, op.propsJson, intern);
      break;
    case "UpdateText":
      w.writeU8(OpCode.UpdateText);
      w.writeU64(op.nodeId);
      writeStringRef(w, op.content, intern);
      break;
    case "DestroyNode":
      w.writeU8(OpCode.DestroyNode);
      w.writeU64(op.nodeId);
      break;
    case "ClearContainer":
      w.writeU8(OpCode.ClearContainer);
      break;
    case "Commit":
      w.writeU8(OpCode.Commit);
      break;
  }
}

function writeStringRef(w: Writer, s: string, intern: StringInterner): void {
  const idx = intern.indexOf(s);
  // Index 0 is reserved as the inline escape hatch.
  if (idx === 0) {
    w.writeU32(0);
    w.writeInlineString(s);
    return;
  }
  w.writeU32(idx);
}

// ---------------------------------------------------------------------------
// Decode helpers
// ---------------------------------------------------------------------------

function readStringRef(cur: Cursor, table: string[] | null): string {
  if (table === null) {
    return cur.readInlineString();
  }
  const idx = cur.readU32();
  if (idx === 0) {
    return cur.readInlineString();
  }
  if (idx >= table.length) {
    throw new DecodeError(`BadStringRef(${idx})`);
  }
  return table[idx]!;
}

function readOp(cur: Cursor, table: string[] | null): BinaryOp {
  const opcode = cur.readU8();
  switch (opcode) {
    case OpCode.CreateNode:
      return {
        op: "CreateNode",
        nodeId: cur.readU64(),
        nodeType: readStringRef(cur, table),
        propsJson: readStringRef(cur, table),
      };
    case OpCode.CreateText:
      return {
        op: "CreateText",
        nodeId: cur.readU64(),
        content: readStringRef(cur, table),
      };
    case OpCode.AppendChild:
      return {
        op: "AppendChild",
        parentId: cur.readU64(),
        childId: cur.readU64(),
      };
    case OpCode.InsertBefore:
      return {
        op: "InsertBefore",
        parentId: cur.readU64(),
        childId: cur.readU64(),
        beforeId: cur.readU64(),
      };
    case OpCode.RemoveChild:
      return {
        op: "RemoveChild",
        parentId: cur.readU64(),
        childId: cur.readU64(),
      };
    case OpCode.UpdateNode:
      return {
        op: "UpdateNode",
        nodeId: cur.readU64(),
        propsJson: readStringRef(cur, table),
      };
    case OpCode.UpdateText:
      return {
        op: "UpdateText",
        nodeId: cur.readU64(),
        content: readStringRef(cur, table),
      };
    case OpCode.DestroyNode:
      return { op: "DestroyNode", nodeId: cur.readU64() };
    case OpCode.ClearContainer:
      return { op: "ClearContainer" };
    case OpCode.Commit:
      return { op: "Commit" };
    default:
      throw new DecodeError(`UnknownOpcode(${opcode})`);
  }
}

// ---------------------------------------------------------------------------
// Byte I/O
// ---------------------------------------------------------------------------

class StringInterner {
  /** Index 0 reserved / unused (inline escape). */
  readonly table: string[] = [""];
  private readonly index = new Map<string, number>();

  intern(s: string): number {
    const existing = this.index.get(s);
    if (existing !== undefined) return existing;
    const idx = this.table.length;
    this.table.push(s);
    this.index.set(s, idx);
    return idx;
  }

  indexOf(s: string): number {
    const idx = this.index.get(s);
    if (idx === undefined) {
      throw new EncodeError(`StringNotInterned: ${s}`);
    }
    return idx;
  }
}

class Writer {
  private chunks: Uint8Array[] = [];
  private len = 0;

  writeU8(v: number): void {
    this.push(Uint8Array.of(v & 0xff));
  }

  writeU16(v: number): void {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setUint16(0, v >>> 0, true);
    this.push(b);
  }

  writeU32(v: number): void {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, v >>> 0, true);
    this.push(b);
  }

  /** Write a JS number as little-endian u64 (ids fit in 2^53). */
  writeU64(v: number): void {
    if (!Number.isFinite(v) || v < 0 || v > Number.MAX_SAFE_INTEGER) {
      throw new EncodeError(`InvalidU64(${v})`);
    }
    const lo = v >>> 0;
    const hi = Math.floor(v / 0x1_0000_0000) >>> 0;
    const b = new Uint8Array(8);
    const dv = new DataView(b.buffer);
    dv.setUint32(0, lo, true);
    dv.setUint32(4, hi, true);
    this.push(b);
  }

  writeInlineString(s: string): void {
    const utf8 = new TextEncoder().encode(s);
    this.writeU32(utf8.length);
    this.push(utf8);
  }

  toUint8Array(): Uint8Array {
    const out = new Uint8Array(this.len);
    let offset = 0;
    for (const chunk of this.chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  }

  private push(chunk: Uint8Array): void {
    this.chunks.push(chunk);
    this.len += chunk.length;
  }
}

class Cursor {
  private pos = 0;

  constructor(private readonly buf: Uint8Array) {}

  isEmpty(): boolean {
    return this.pos >= this.buf.length;
  }

  readU8(): number {
    return this.take(1)[0]!;
  }

  readU16(): number {
    const b = this.take(2);
    return new DataView(b.buffer, b.byteOffset, 2).getUint16(0, true);
  }

  readU32(): number {
    const b = this.take(4);
    return new DataView(b.buffer, b.byteOffset, 4).getUint32(0, true);
  }

  readU64(): number {
    const b = this.take(8);
    const dv = new DataView(b.buffer, b.byteOffset, 8);
    const lo = dv.getUint32(0, true);
    const hi = dv.getUint32(4, true);
    const v = hi * 0x1_0000_0000 + lo;
    if (v > Number.MAX_SAFE_INTEGER) {
      throw new DecodeError(`U64TooLarge(${v})`);
    }
    return v;
  }

  readInlineString(): string {
    const len = this.readU32();
    const bytes = this.take(len);
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      throw new DecodeError("InvalidUtf8");
    }
  }

  private take(n: number): Uint8Array {
    const end = this.pos + n;
    if (end > this.buf.length) {
      throw new DecodeError("Truncated");
    }
    const slice = this.buf.subarray(this.pos, end);
    this.pos = end;
    return slice;
  }
}
