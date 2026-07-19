//! Binary mutation protocol (BRRP) for React → Bevy ops.
//!
//! See [`docs/PROTO.md`](../../../../docs/PROTO.md) for the full schema, string-table
//! interning, and frame-aligned commit model.
//!
//! The enum RPC path (`ReactClientProto` over `mpsc`) remains the default. This module
//! encodes the same hot ops as a compact little-endian batch. Enable the `binary_ops`
//! Cargo feature to register `__react_commit_ops`, then opt the TS reconciler in via
//! `binaryOps: true` or `globalThis.__BEVY_REACT_BINARY_OPS = 1`.

mod codec;

pub use codec::{
    decode_batch, decode_protos, encode_batch, encode_batch_with, encode_protos, BinaryOp,
    DecodeError, EncodeError, EncodeOptions, FLAG_STRING_TABLE, MAGIC, VERSION,
};
