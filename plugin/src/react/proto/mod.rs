//! Binary mutation protocol (BRRP) for React → Bevy ops.
//!
//! See [`docs/PROTO.md`](../../../../docs/PROTO.md) for the full schema, string-interning
//! design notes, and frame-aligned commit model.
//!
//! The enum RPC path (`ReactClientProto` over `mpsc`) remains the default. This module
//! encodes the same hot ops as a compact little-endian batch. Enable the `binary_ops`
//! Cargo feature to register `__react_commit_ops` for a dual path from JS.

mod codec;

pub use codec::{
    decode_batch, decode_protos, encode_batch, encode_protos, BinaryOp, DecodeError,
    EncodeError, FLAG_STRING_TABLE, MAGIC, VERSION,
};
