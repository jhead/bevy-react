# Binary op protocol (BRRP)

Bevy-React Render Protocol — a compact little-endian batch format for the hot React → Bevy mutation path (`CreateNode`, `UpdateNode`, `AppendChild`, …).

Today the default path is still the in-process enum RPC (`ReactClientProto` over `mpsc`). BRRP sits **beside** that path: same semantics, denser framing for a future JS→native batch commit.

## Status

| Piece | State |
|---|---|
| Schema + design notes | This doc |
| Rust encode/decode (`plugin/src/react/proto/`) | Landed; unit-tested |
| Cargo feature `binary_ops` | Registers `__react_commit_ops` |
| TS reconciler encoding | Stub only (`packages/bevy-react/src/protocol.ts`) |
| Default reconciler switched to binary | **Not wired** — enum natives remain |

Enable the host binding:

```bash
cargo build --manifest-path plugin/Cargo.toml --features binary_ops
```

## Why custom binary (not protobuf)

Ops are tiny, fixed-shape, and frame-scoped. A hand-rolled LE codec avoids prost/protobuf deps, keeps zero-copy-friendly layouts later, and matches Fabric-style “mutation instructions in a buffer” more closely than a general IDL.

## Frame layout (v1)

All multi-byte integers are **little-endian**.

```
Frame {
  magic:      u32   // 0x42525250 ASCII "BRRP" (B at low byte)
  version:    u16   // 1
  flags:      u16   // MVP: must be 0; bit0 reserved for string table
  root_id:    String
  op_count:   u32
  ops:        Op[op_count]
}

String {
  len:   u32
  utf8:  u8[len]
}
```

`root_id` is written **once per frame** and expanded onto each decoded `ReactClientProto` message.

### Opcodes

| Code | Name | Payload |
|---|---|---|
| `0x01` | CreateNode | `node_id:u64`, `node_type:String`, `props_json:String` |
| `0x02` | CreateText | `node_id:u64`, `content:String` |
| `0x03` | AppendChild | `parent_id:u64`, `child_id:u64` |
| `0x04` | InsertBefore | `parent_id:u64`, `child_id:u64`, `before_id:u64` |
| `0x05` | RemoveChild | `parent_id:u64`, `child_id:u64` |
| `0x06` | UpdateNode | `node_id:u64`, `props_json:String` |
| `0x07` | UpdateText | `node_id:u64`, `content:String` |
| `0x08` | DestroyNode | `node_id:u64` |
| `0x09` | ClearContainer | _(none)_ |
| `0x0A` | Commit | _(none)_ → `ReactClientProto::Complete` |

Props remain JSON strings in v1 (style conversion still parses JSON). A later revision can intern style keys or send typed prop bags.

## Frame-aligned commits

One BRRP frame = one React commit batch for a single root:

1. Reconciler (or test harness) encodes all mutations for the commit into one buffer.
2. Host decodes and enqueues `ReactClientProto` messages in order.
3. Trailing `Commit` (`0x0A`) maps to `Complete` (same marker the enum path already uses).

The Bevy render system still drains the channel per frame; BRRP does not yet introduce a separate apply barrier beyond `Complete`.

## String interning (design notes — not in MVP)

Repeated `node_type`, style keys, and `root_id`-adjacent strings dominate payload size once props stop being JSON blobs.

Planned shape when `flags & FLAG_STRING_TABLE` is set:

```
Frame {
  …
  flags:      u16   // bit0 = string table present
  table: {
    count: u32
    entries: String[count]   // index 0 reserved / unused
  }
  root_id:    StringRef      // u32 index into table, or inline escape
  …
}

StringRef = u32 index   // 0 means “inline String follows” escape hatch (TBD)
```

MVP keeps **inline strings only** (`flags == 0`). Decoders must reject unknown flags so old hosts fail loudly.

## Dual path

| Path | How |
|---|---|
| **Enum (default)** | `__react_create_node`, `__react_append_child`, … → `ReactClient::send` |
| **Binary (`binary_ops`)** | `__react_commit_ops(Uint8Array \| ArrayBuffer)` → `decode_protos` → same channel |

The reconciler does not call `__react_commit_ops` yet. Use the Rust codec (or the TS stub constants) to build buffers for tests and future wiring.

## Rust API

```rust
use bevy_react::react::proto::{decode_batch, encode_batch, BinaryOp};

let bytes = encode_batch("root", &[
    BinaryOp::CreateNode {
        node_id: 1,
        node_type: "view".into(),
        props_json: "{}".into(),
    },
    BinaryOp::Commit,
])?;
let (root, ops) = decode_batch(&bytes)?;
```

`encode_protos` / `decode_protos` convert to/from `ReactClientProto`.
