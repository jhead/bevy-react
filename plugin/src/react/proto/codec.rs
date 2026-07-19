//! Encode / decode BRRP v1 batches.
//!
//! Wire layout (little-endian):
//!
//! ```text
//! Frame {
//!   magic:      u32   // MAGIC = 0x4252_5250 ("BRRP")
//!   version:    u16   // VERSION = 1
//!   flags:      u16   // bit0 = FLAG_STRING_TABLE
//!   [table: { count:u32, entries:String[count] }]  // when FLAG_STRING_TABLE
//!   root_id:    String | StringRef
//!   op_count:   u32
//!   ops:        Op[op_count]
//! }
//!
//! String { len: u32, utf8: u8[len] }
//! StringRef = u32 index  // 0 ⇒ inline String follows; else table[index]
//!
//! Op { opcode: u8, payload… }
//! ```

use crate::react::client::ReactClientProto;
use std::collections::HashMap;

/// ASCII `BRRP` as little-endian u32 (`B` at the low byte).
pub const MAGIC: u32 = u32::from_le_bytes(*b"BRRP");

/// Protocol version encoded in every frame.
pub const VERSION: u16 = 1;

/// Frame carries a string table; string fields are StringRef indices.
pub const FLAG_STRING_TABLE: u16 = 1 << 0;

const OP_CREATE_NODE: u8 = 0x01;
const OP_CREATE_TEXT: u8 = 0x02;
const OP_APPEND_CHILD: u8 = 0x03;
const OP_INSERT_BEFORE: u8 = 0x04;
const OP_REMOVE_CHILD: u8 = 0x05;
const OP_UPDATE_NODE: u8 = 0x06;
const OP_UPDATE_TEXT: u8 = 0x07;
const OP_DESTROY_NODE: u8 = 0x08;
const OP_CLEAR_CONTAINER: u8 = 0x09;
const OP_COMMIT: u8 = 0x0A;

/// One mutation inside a batch. `root_id` lives on the frame, not per op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    CreateNode {
        node_id: u64,
        node_type: String,
        props_json: String,
    },
    CreateText {
        node_id: u64,
        content: String,
    },
    AppendChild {
        parent_id: u64,
        child_id: u64,
    },
    InsertBefore {
        parent_id: u64,
        child_id: u64,
        before_id: u64,
    },
    RemoveChild {
        parent_id: u64,
        child_id: u64,
    },
    UpdateNode {
        node_id: u64,
        props_json: String,
    },
    UpdateText {
        node_id: u64,
        content: String,
    },
    DestroyNode {
        node_id: u64,
    },
    ClearContainer,
    /// Frame-aligned commit marker (maps to [`ReactClientProto::Complete`]).
    Commit,
}

/// Encoding failure (currently only empty root id is rejected up-front).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    EmptyRootId,
}

/// Decoding failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    BadMagic(u32),
    UnsupportedVersion(u16),
    UnsupportedFlags(u16),
    InvalidUtf8,
    UnknownOpcode(u8),
    TrailingBytes,
    EmptyStringTable,
    BadStringRef(u32),
}

/// Options for [`encode_batch`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodeOptions {
    /// When true, set [`FLAG_STRING_TABLE`] and intern repeated strings.
    pub string_table: bool,
}

impl BinaryOp {
    /// Convert a host RPC message into a frame op. `Complete` → [`BinaryOp::Commit`].
    pub fn from_proto(msg: &ReactClientProto) -> Option<Self> {
        Some(match msg {
            ReactClientProto::CreateNode {
                node_id,
                node_type,
                props_json,
                ..
            } => Self::CreateNode {
                node_id: *node_id,
                node_type: node_type.clone(),
                props_json: props_json.clone(),
            },
            ReactClientProto::CreateText {
                node_id, content, ..
            } => Self::CreateText {
                node_id: *node_id,
                content: content.clone(),
            },
            ReactClientProto::AppendChild {
                parent_id,
                child_id,
                ..
            } => Self::AppendChild {
                parent_id: *parent_id,
                child_id: *child_id,
            },
            ReactClientProto::InsertBefore {
                parent_id,
                child_id,
                before_id,
                ..
            } => Self::InsertBefore {
                parent_id: *parent_id,
                child_id: *child_id,
                before_id: *before_id,
            },
            ReactClientProto::RemoveChild {
                parent_id,
                child_id,
                ..
            } => Self::RemoveChild {
                parent_id: *parent_id,
                child_id: *child_id,
            },
            ReactClientProto::UpdateNode {
                node_id,
                props_json,
                ..
            } => Self::UpdateNode {
                node_id: *node_id,
                props_json: props_json.clone(),
            },
            ReactClientProto::UpdateText {
                node_id, content, ..
            } => Self::UpdateText {
                node_id: *node_id,
                content: content.clone(),
            },
            ReactClientProto::DestroyNode { node_id, .. } => Self::DestroyNode {
                node_id: *node_id,
            },
            ReactClientProto::ClearContainer { .. } => Self::ClearContainer,
            ReactClientProto::Complete => Self::Commit,
        })
    }

    /// Expand a frame op into a host RPC message using the batch `root_id`.
    pub fn into_proto(self, root_id: &str) -> ReactClientProto {
        match self {
            Self::CreateNode {
                node_id,
                node_type,
                props_json,
            } => ReactClientProto::CreateNode {
                root_id: root_id.to_owned(),
                node_id,
                node_type,
                props_json,
            },
            Self::CreateText { node_id, content } => ReactClientProto::CreateText {
                root_id: root_id.to_owned(),
                node_id,
                content,
            },
            Self::AppendChild {
                parent_id,
                child_id,
            } => ReactClientProto::AppendChild {
                root_id: root_id.to_owned(),
                parent_id,
                child_id,
            },
            Self::InsertBefore {
                parent_id,
                child_id,
                before_id,
            } => ReactClientProto::InsertBefore {
                root_id: root_id.to_owned(),
                parent_id,
                child_id,
                before_id,
            },
            Self::RemoveChild {
                parent_id,
                child_id,
            } => ReactClientProto::RemoveChild {
                root_id: root_id.to_owned(),
                parent_id,
                child_id,
            },
            Self::UpdateNode {
                node_id,
                props_json,
            } => ReactClientProto::UpdateNode {
                root_id: root_id.to_owned(),
                node_id,
                props_json,
            },
            Self::UpdateText { node_id, content } => ReactClientProto::UpdateText {
                root_id: root_id.to_owned(),
                node_id,
                content,
            },
            Self::DestroyNode { node_id } => ReactClientProto::DestroyNode {
                root_id: root_id.to_owned(),
                node_id,
            },
            Self::ClearContainer => ReactClientProto::ClearContainer {
                root_id: root_id.to_owned(),
            },
            Self::Commit => ReactClientProto::Complete,
        }
    }
}

/// Encode a frame for `root_id` containing `ops` (inline strings, flags = 0).
pub fn encode_batch(root_id: &str, ops: &[BinaryOp]) -> Result<Vec<u8>, EncodeError> {
    encode_batch_with(root_id, ops, EncodeOptions::default())
}

/// Encode a frame with optional string-table interning.
pub fn encode_batch_with(
    root_id: &str,
    ops: &[BinaryOp],
    options: EncodeOptions,
) -> Result<Vec<u8>, EncodeError> {
    if root_id.is_empty() {
        return Err(EncodeError::EmptyRootId);
    }

    if options.string_table {
        encode_batch_string_table(root_id, ops)
    } else {
        encode_batch_inline(root_id, ops)
    }
}

fn encode_batch_inline(root_id: &str, ops: &[BinaryOp]) -> Result<Vec<u8>, EncodeError> {
    let mut out = Vec::with_capacity(64 + ops.len() * 24);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    write_inline_string(&mut out, root_id);
    out.extend_from_slice(&(ops.len() as u32).to_le_bytes());
    for op in ops {
        write_op_inline(&mut out, op);
    }
    Ok(out)
}

fn encode_batch_string_table(root_id: &str, ops: &[BinaryOp]) -> Result<Vec<u8>, EncodeError> {
    let mut intern = StringInterner::new();
    intern.intern(root_id);
    for op in ops {
        collect_op_strings(op, &mut intern);
    }

    let mut out = Vec::with_capacity(64 + ops.len() * 24 + intern.table.len() * 8);
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&FLAG_STRING_TABLE.to_le_bytes());
    out.extend_from_slice(&(intern.table.len() as u32).to_le_bytes());
    for entry in &intern.table {
        write_inline_string(&mut out, entry);
    }
    write_string_ref(&mut out, root_id, &intern);
    out.extend_from_slice(&(ops.len() as u32).to_le_bytes());
    for op in ops {
        write_op_with_table(&mut out, op, &intern);
    }
    Ok(out)
}

/// Decode a BRRP frame into `(root_id, ops)`.
pub fn decode_batch(bytes: &[u8]) -> Result<(String, Vec<BinaryOp>), DecodeError> {
    let mut cur = Cursor::new(bytes);
    let magic = cur.read_u32()?;
    if magic != MAGIC {
        return Err(DecodeError::BadMagic(magic));
    }
    let version = cur.read_u16()?;
    if version != VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let flags = cur.read_u16()?;
    let unknown = flags & !FLAG_STRING_TABLE;
    if unknown != 0 {
        return Err(DecodeError::UnsupportedFlags(flags));
    }

    let table = if flags & FLAG_STRING_TABLE != 0 {
        let count = cur.read_u32()? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(cur.read_inline_string()?);
        }
        if entries.is_empty() {
            return Err(DecodeError::EmptyStringTable);
        }
        Some(entries)
    } else {
        None
    };

    let root_id = read_string_ref(&mut cur, table.as_deref())?;
    let op_count = cur.read_u32()? as usize;
    let mut ops = Vec::with_capacity(op_count);
    for _ in 0..op_count {
        ops.push(read_op(&mut cur, table.as_deref())?);
    }
    if !cur.is_empty() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok((root_id, ops))
}

/// Encode `ReactClientProto` messages that share one `root_id` (plus optional `Complete`).
///
/// Messages with a mismatched `root_id` are skipped. Empty input yields an empty-op frame
/// only when `root_id` is provided explicitly via [`encode_batch`].
pub fn encode_protos(
    root_id: &str,
    msgs: &[ReactClientProto],
) -> Result<Vec<u8>, EncodeError> {
    let mut ops = Vec::with_capacity(msgs.len());
    for msg in msgs {
        if let Some(op) = BinaryOp::from_proto(msg) {
            // Skip messages for a different root (Complete has no root).
            if let Some(msg_root) = proto_root_id(msg) {
                if msg_root != root_id {
                    continue;
                }
            }
            ops.push(op);
        }
    }
    encode_batch(root_id, &ops)
}

/// Decode a frame into host RPC messages (root expanded onto each op).
pub fn decode_protos(bytes: &[u8]) -> Result<Vec<ReactClientProto>, DecodeError> {
    let (root_id, ops) = decode_batch(bytes)?;
    Ok(ops
        .into_iter()
        .map(|op| op.into_proto(&root_id))
        .collect())
}

fn proto_root_id(msg: &ReactClientProto) -> Option<&str> {
    match msg {
        ReactClientProto::CreateNode { root_id, .. }
        | ReactClientProto::CreateText { root_id, .. }
        | ReactClientProto::AppendChild { root_id, .. }
        | ReactClientProto::InsertBefore { root_id, .. }
        | ReactClientProto::RemoveChild { root_id, .. }
        | ReactClientProto::UpdateNode { root_id, .. }
        | ReactClientProto::UpdateText { root_id, .. }
        | ReactClientProto::DestroyNode { root_id, .. }
        | ReactClientProto::ClearContainer { root_id } => Some(root_id.as_str()),
        ReactClientProto::Complete => None,
    }
}

fn write_inline_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn write_string_ref(out: &mut Vec<u8>, s: &str, intern: &StringInterner) {
    let idx = intern.index_of(s);
    // Index 0 is reserved as the inline escape hatch.
    if idx == 0 {
        out.extend_from_slice(&0u32.to_le_bytes());
        write_inline_string(out, s);
        return;
    }
    out.extend_from_slice(&idx.to_le_bytes());
}

fn collect_op_strings(op: &BinaryOp, intern: &mut StringInterner) {
    match op {
        BinaryOp::CreateNode {
            node_type,
            props_json,
            ..
        } => {
            intern.intern(node_type);
            intern.intern(props_json);
        }
        BinaryOp::CreateText { content, .. } | BinaryOp::UpdateText { content, .. } => {
            intern.intern(content);
        }
        BinaryOp::UpdateNode { props_json, .. } => {
            intern.intern(props_json);
        }
        _ => {}
    }
}

fn write_op_inline(out: &mut Vec<u8>, op: &BinaryOp) {
    match op {
        BinaryOp::CreateNode {
            node_id,
            node_type,
            props_json,
        } => {
            out.push(OP_CREATE_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_inline_string(out, node_type);
            write_inline_string(out, props_json);
        }
        BinaryOp::CreateText { node_id, content } => {
            out.push(OP_CREATE_TEXT);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_inline_string(out, content);
        }
        BinaryOp::AppendChild {
            parent_id,
            child_id,
        } => {
            out.push(OP_APPEND_CHILD);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
        }
        BinaryOp::InsertBefore {
            parent_id,
            child_id,
            before_id,
        } => {
            out.push(OP_INSERT_BEFORE);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
            out.extend_from_slice(&before_id.to_le_bytes());
        }
        BinaryOp::RemoveChild {
            parent_id,
            child_id,
        } => {
            out.push(OP_REMOVE_CHILD);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
        }
        BinaryOp::UpdateNode {
            node_id,
            props_json,
        } => {
            out.push(OP_UPDATE_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_inline_string(out, props_json);
        }
        BinaryOp::UpdateText { node_id, content } => {
            out.push(OP_UPDATE_TEXT);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_inline_string(out, content);
        }
        BinaryOp::DestroyNode { node_id } => {
            out.push(OP_DESTROY_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
        }
        BinaryOp::ClearContainer => {
            out.push(OP_CLEAR_CONTAINER);
        }
        BinaryOp::Commit => {
            out.push(OP_COMMIT);
        }
    }
}

fn write_op_with_table(out: &mut Vec<u8>, op: &BinaryOp, intern: &StringInterner) {
    match op {
        BinaryOp::CreateNode {
            node_id,
            node_type,
            props_json,
        } => {
            out.push(OP_CREATE_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_string_ref(out, node_type, intern);
            write_string_ref(out, props_json, intern);
        }
        BinaryOp::CreateText { node_id, content } => {
            out.push(OP_CREATE_TEXT);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_string_ref(out, content, intern);
        }
        BinaryOp::AppendChild {
            parent_id,
            child_id,
        } => {
            out.push(OP_APPEND_CHILD);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
        }
        BinaryOp::InsertBefore {
            parent_id,
            child_id,
            before_id,
        } => {
            out.push(OP_INSERT_BEFORE);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
            out.extend_from_slice(&before_id.to_le_bytes());
        }
        BinaryOp::RemoveChild {
            parent_id,
            child_id,
        } => {
            out.push(OP_REMOVE_CHILD);
            out.extend_from_slice(&parent_id.to_le_bytes());
            out.extend_from_slice(&child_id.to_le_bytes());
        }
        BinaryOp::UpdateNode {
            node_id,
            props_json,
        } => {
            out.push(OP_UPDATE_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_string_ref(out, props_json, intern);
        }
        BinaryOp::UpdateText { node_id, content } => {
            out.push(OP_UPDATE_TEXT);
            out.extend_from_slice(&node_id.to_le_bytes());
            write_string_ref(out, content, intern);
        }
        BinaryOp::DestroyNode { node_id } => {
            out.push(OP_DESTROY_NODE);
            out.extend_from_slice(&node_id.to_le_bytes());
        }
        BinaryOp::ClearContainer => {
            out.push(OP_CLEAR_CONTAINER);
        }
        BinaryOp::Commit => {
            out.push(OP_COMMIT);
        }
    }
}

fn read_string_ref(cur: &mut Cursor<'_>, table: Option<&[String]>) -> Result<String, DecodeError> {
    match table {
        None => cur.read_inline_string(),
        Some(entries) => {
            let idx = cur.read_u32()?;
            if idx == 0 {
                return cur.read_inline_string();
            }
            let i = idx as usize;
            entries
                .get(i)
                .cloned()
                .ok_or(DecodeError::BadStringRef(idx))
        }
    }
}

fn read_op(cur: &mut Cursor<'_>, table: Option<&[String]>) -> Result<BinaryOp, DecodeError> {
    let opcode = cur.read_u8()?;
    Ok(match opcode {
        OP_CREATE_NODE => BinaryOp::CreateNode {
            node_id: cur.read_u64()?,
            node_type: read_string_ref(cur, table)?,
            props_json: read_string_ref(cur, table)?,
        },
        OP_CREATE_TEXT => BinaryOp::CreateText {
            node_id: cur.read_u64()?,
            content: read_string_ref(cur, table)?,
        },
        OP_APPEND_CHILD => BinaryOp::AppendChild {
            parent_id: cur.read_u64()?,
            child_id: cur.read_u64()?,
        },
        OP_INSERT_BEFORE => BinaryOp::InsertBefore {
            parent_id: cur.read_u64()?,
            child_id: cur.read_u64()?,
            before_id: cur.read_u64()?,
        },
        OP_REMOVE_CHILD => BinaryOp::RemoveChild {
            parent_id: cur.read_u64()?,
            child_id: cur.read_u64()?,
        },
        OP_UPDATE_NODE => BinaryOp::UpdateNode {
            node_id: cur.read_u64()?,
            props_json: read_string_ref(cur, table)?,
        },
        OP_UPDATE_TEXT => BinaryOp::UpdateText {
            node_id: cur.read_u64()?,
            content: read_string_ref(cur, table)?,
        },
        OP_DESTROY_NODE => BinaryOp::DestroyNode {
            node_id: cur.read_u64()?,
        },
        OP_CLEAR_CONTAINER => BinaryOp::ClearContainer,
        OP_COMMIT => BinaryOp::Commit,
        other => return Err(DecodeError::UnknownOpcode(other)),
    })
}

struct StringInterner {
    /// Index 0 reserved / unused (inline escape).
    table: Vec<String>,
    index: HashMap<String, u32>,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            table: vec![String::new()],
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.index.get(s) {
            return idx;
        }
        let idx = self.table.len() as u32;
        self.table.push(s.to_owned());
        self.index.insert(s.to_owned(), idx);
        idx
    }

    fn index_of(&self, s: &str) -> u32 {
        *self
            .index
            .get(s)
            .expect("string must be interned before encode")
    }
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.buf.len())
            .ok_or(DecodeError::Truncated)?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, DecodeError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_inline_string(&mut self) -> Result<String, DecodeError> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError::InvalidUtf8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ops() -> Vec<BinaryOp> {
        vec![
            BinaryOp::CreateNode {
                node_id: 1,
                node_type: "view".into(),
                props_json: r#"{"style":{"flex":1}}"#.into(),
            },
            BinaryOp::CreateText {
                node_id: 2,
                content: "hi".into(),
            },
            BinaryOp::AppendChild {
                parent_id: 1,
                child_id: 2,
            },
            BinaryOp::InsertBefore {
                parent_id: 1,
                child_id: 3,
                before_id: 2,
            },
            BinaryOp::UpdateNode {
                node_id: 1,
                props_json: r#"{"style":{"opacity":0.5}}"#.into(),
            },
            BinaryOp::UpdateText {
                node_id: 2,
                content: "hello".into(),
            },
            BinaryOp::RemoveChild {
                parent_id: 1,
                child_id: 3,
            },
            BinaryOp::DestroyNode { node_id: 3 },
            BinaryOp::ClearContainer,
            BinaryOp::Commit,
        ]
    }

    #[test]
    fn round_trip_common_ops() {
        let ops = sample_ops();
        let bytes = encode_batch("root-a", &ops).unwrap();
        assert_eq!(&bytes[..4], b"BRRP");
        let (root, decoded) = decode_batch(&bytes).unwrap();
        assert_eq!(root, "root-a");
        assert_eq!(decoded, ops);
    }

    #[test]
    fn round_trip_with_string_table() {
        let ops = sample_ops();
        let bytes = encode_batch_with(
            "root-a",
            &ops,
            EncodeOptions {
                string_table: true,
            },
        )
        .unwrap();
        assert_eq!(&bytes[..4], b"BRRP");
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        assert_eq!(flags, FLAG_STRING_TABLE);
        let (root, decoded) = decode_batch(&bytes).unwrap();
        assert_eq!(root, "root-a");
        assert_eq!(decoded, ops);
    }

    /// Stable golden bytes for the TS vitest harness (`packages/bevy-react/tests/protocol.test.ts`).
    #[test]
    fn golden_inline_commit_frame() {
        let ops = [
            BinaryOp::CreateNode {
                node_id: 1,
                node_type: "view".into(),
                props_json: "{}".into(),
            },
            BinaryOp::AppendChild {
                parent_id: 0,
                child_id: 1,
            },
            BinaryOp::Commit,
        ];
        let bytes = encode_batch("r", &ops).unwrap();
        let expected: &[u8] = &[
            // magic BRRP, version 1, flags 0
            b'B', b'R', b'R', b'P', 0x01, 0x00, 0x00, 0x00, //
            // root_id "r"
            0x01, 0x00, 0x00, 0x00, b'r', //
            // op_count = 3
            0x03, 0x00, 0x00, 0x00, //
            // CreateNode id=1 type="view" props="{}"
            0x01, //
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            0x04, 0x00, 0x00, 0x00, b'v', b'i', b'e', b'w', //
            0x02, 0x00, 0x00, 0x00, b'{', b'}', //
            // AppendChild parent=0 child=1
            0x03, //
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
            // Commit
            0x0a,
        ];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn round_trip_via_react_client_proto() {
        let root = "hud";
        let msgs = vec![
            ReactClientProto::CreateNode {
                root_id: root.into(),
                node_id: 10,
                node_type: "button".into(),
                props_json: "{}".into(),
            },
            ReactClientProto::AppendChild {
                root_id: root.into(),
                parent_id: 0,
                child_id: 10,
            },
            ReactClientProto::UpdateNode {
                root_id: root.into(),
                node_id: 10,
                props_json: r#"{"disabled":true}"#.into(),
            },
            ReactClientProto::DestroyNode {
                root_id: root.into(),
                node_id: 10,
            },
            ReactClientProto::Complete,
        ];

        let bytes = encode_protos(root, &msgs).unwrap();
        let decoded = decode_protos(&bytes).unwrap();
        assert_eq!(decoded, msgs);
    }

    #[test]
    fn rejects_bad_magic_and_truncation() {
        assert!(matches!(
            decode_batch(b"XXXX"),
            Err(DecodeError::BadMagic(_)) | Err(DecodeError::Truncated)
        ));
        let mut bytes = encode_batch("r", &[BinaryOp::Commit]).unwrap();
        bytes.truncate(8);
        assert_eq!(decode_batch(&bytes), Err(DecodeError::Truncated));
    }

    #[test]
    fn rejects_unknown_opcode() {
        let mut bytes = encode_batch("r", &[]).unwrap();
        // bump op_count to 1 and append bogus opcode
        let op_count_pos = 4 + 2 + 2 + 4 + 1; // after root_id "r"
        bytes[op_count_pos..op_count_pos + 4].copy_from_slice(&1u32.to_le_bytes());
        bytes.push(0xFF);
        assert_eq!(decode_batch(&bytes), Err(DecodeError::UnknownOpcode(0xFF)));
    }

    #[test]
    fn empty_root_id_rejected() {
        assert_eq!(
            encode_batch("", &[BinaryOp::Commit]),
            Err(EncodeError::EmptyRootId)
        );
    }
}
