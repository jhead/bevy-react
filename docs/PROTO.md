# Binary op protocol (BRRP)

Bevy-React Render Protocol — a compact little-endian batch format for the hot React → Bevy mutation path (`CreateNode`, `UpdateNode`, `AppendChild`, …).

Today the **default** path is still the in-process enum RPC (`ReactClientProto` over `mpsc`). BRRP sits **beside** that path: same semantics, denser framing. The TS reconciler can encode one commit into a BRRP frame when opted in.

## Status

| Piece | State |
|---|---|
| Schema + design notes | This doc |
| Rust encode/decode (`plugin/src/react/proto/`) | Landed; unit-tested (inline + string table) |
| Cargo feature `binary_ops` | Registers `__react_commit_ops` |
| TS encode/decode (`packages/bevy-react/src/protocol.ts`) | Landed; golden-byte + round-trip tests |
| TS reconciler binary commit path | Opt-in (`binaryOps` / `__BEVY_REACT_BINARY_OPS`) |
| Default reconciler switched to binary | **Not yet** — enum natives remain default |

## Enabling the binary hot path

**Host** (required for `__react_commit_ops`):

```bash
cargo build --manifest-path plugin/Cargo.toml --features binary_ops
```

**JS reconciler** (pick one):

```js
// Package option (preferred)
createBevyApp(<App />, { binaryOps: true });
// or
renderRoot(<App />, "root", { binaryOps: true });
createBevyReconciler({ rootId, instanceMap, binaryOps: true });
```

```js
// Global flag (Boa / Vite entry can set before createBevyApp)
globalThis.__BEVY_REACT_BINARY_OPS = 1; // also true / "1"
```

When enabled, mutation RPCs are queued during the React commit and flushed once via `__react_commit_ops(Uint8Array)` from `resetAfterCommit`. Node ids are allocated on the JS side; the host advances its id counter so a later enum-path alloc cannot collide.

## Why custom binary (not protobuf)

Ops are tiny, fixed-shape, and frame-scoped. A hand-rolled LE codec avoids prost/protobuf deps, keeps zero-copy-friendly layouts later, and matches Fabric-style “mutation instructions in a buffer” more closely than a general IDL.

## Frame layout (v1)

All multi-byte integers are **little-endian**.

```
Frame {
  magic:      u32   // ASCII "BRRP" on the wire (B at low byte) = 0x50525242
  version:    u16   // 1
  flags:      u16   // bit0 = FLAG_STRING_TABLE; other bits rejected
  [table: {         // only when FLAG_STRING_TABLE
    count: u32
    entries: String[count]   // index 0 reserved / unused
  }]
  root_id:    String | StringRef
  op_count:   u32
  ops:        Op[op_count]
}

String {
  len:   u32
  utf8:  u8[len]
}

StringRef = u32 index   // 0 ⇒ inline String follows; else table[index]
```

`root_id` is written **once per frame** and expanded onto each decoded `ReactClientProto` message.

### Opcodes

| Code | Name | Payload |
|---|---|---|
| `0x01` | CreateNode | `node_id:u64`, `node_type:String|StringRef`, `props_json:String|StringRef` |
| `0x02` | CreateText | `node_id:u64`, `content:String|StringRef` |
| `0x03` | AppendChild | `parent_id:u64`, `child_id:u64` |
| `0x04` | InsertBefore | `parent_id:u64`, `child_id:u64`, `before_id:u64` |
| `0x05` | RemoveChild | `parent_id:u64`, `child_id:u64` |
| `0x06` | UpdateNode | `node_id:u64`, `props_json:String|StringRef` |
| `0x07` | UpdateText | `node_id:u64`, `content:String|StringRef` |
| `0x08` | DestroyNode | `node_id:u64` |
| `0x09` | ClearContainer | _(none)_ |
| `0x0A` | Commit | _(none)_ → `ReactClientProto::Complete` |

Props remain JSON strings in v1 (style conversion still parses JSON). A later revision can intern style keys or send typed prop bags.

## Frame-aligned commits

One BRRP frame = one React commit batch for a single root:

1. Reconciler encodes all mutations for the commit into one buffer (plus trailing `Commit`).
2. Host decodes and enqueues `ReactClientProto` messages in order.
3. Trailing `Commit` (`0x0A`) maps to `Complete` (same marker the enum path already uses).

The Bevy render system still drains the channel per frame; BRRP does not yet introduce a separate apply barrier beyond `Complete`.

## String interning (`FLAG_STRING_TABLE`)

When `flags & FLAG_STRING_TABLE` is set, the frame includes a string table after `flags`, and every string field is a `StringRef` (`u32` index). Index `0` is reserved as an inline escape (`0` + following `String`).

```rust
encode_batch_with(root, &ops, EncodeOptions { string_table: true })?;
```

```ts
encodeBatch(rootId, ops, { stringTable: true });
```

The reconciler binary path currently uses **inline strings** (`flags == 0`) for soak simplicity; pass `{ stringTable: true }` to the encoder when you want interning. Decoders reject unknown flag bits so old hosts fail loudly.

## Dual path

| Path | How |
|---|---|
| **Enum (default)** | `__react_create_node`, `__react_append_child`, … → `ReactClient::send` |
| **Binary (opt-in)** | Queue ops → `encodeBatch` → `__react_commit_ops` → `decode_protos` → same channel |

## Rust API

```rust
use bevy_react::react::proto::{decode_batch, encode_batch, encode_batch_with, BinaryOp, EncodeOptions};

let bytes = encode_batch("root", &[
    BinaryOp::CreateNode {
        node_id: 1,
        node_type: "view".into(),
        props_json: "{}".into(),
    },
    BinaryOp::Commit,
])?;
let (root, ops) = decode_batch(&bytes)?;

let interned = encode_batch_with("root", &ops, EncodeOptions { string_table: true })?;
```

`encode_protos` / `decode_protos` convert to/from `ReactClientProto`.
