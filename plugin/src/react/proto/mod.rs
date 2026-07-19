//! Binary mutation protocol (BRRP) for React → Bevy ops.
//!
//! See [`docs/PROTO.md`](../../../../docs/PROTO.md) for the full schema, string-table
//! interning, and frame-aligned commit model.
//!
//! Enable the `binary_ops` Cargo feature to register `__react_commit_ops`. The TS
//! reconciler then auto-detects that native and uses BRRP by default (force enum with
//! `binaryOps: false` or `__BEVY_REACT_BINARY_OPS = 0`).

mod codec;

pub use codec::{
    decode_batch, decode_protos, encode_batch, encode_batch_with, encode_protos, BinaryOp,
    DecodeError, EncodeError, EncodeOptions, FLAG_STRING_TABLE, MAGIC, VERSION,
};
