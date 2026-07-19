use boa_gc::{Finalize, Trace, empty_trace};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

/// Global counter for node IDs (used across threads)
static NODE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Thread-safe client for sending React RPC messages to the Bevy main thread.
///
/// Uses an **unbounded** channel so `__react_commit_ops` / per-op natives never
/// block the JS thread while Bevy is inside `execute(flush_events)` — a bounded
/// `sync_channel` deadlocks once the buffer fills during a re-entrant commit.
#[derive(Clone, Debug, Finalize)]
pub struct ReactClient {
    tx: Sender<ReactClientProto>,
}

unsafe impl Trace for ReactClient {
    empty_trace!();
}

/// RPC Protocol messages from JS React reconciler to Bevy
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReactClientProto {
    /// Create a UI node (NodeBundle, ButtonBundle, etc.)
    CreateNode {
        root_id: String,
        node_id: u64,
        node_type: String,
        props_json: String,
    },
    /// Create a text node
    CreateText { root_id: String, node_id: u64, content: String },
    /// Append a child to a parent node
    AppendChild { root_id: String, parent_id: u64, child_id: u64 },
    /// Insert a child before another child in a parent node
    InsertBefore { root_id: String, parent_id: u64, child_id: u64, before_id: u64 },
    /// Remove a child from a parent node
    RemoveChild { root_id: String, parent_id: u64, child_id: u64 },
    /// Update node properties
    UpdateNode { root_id: String, node_id: u64, props_json: String },
    /// Update text content
    UpdateText { root_id: String, node_id: u64, content: String },
    /// Destroy a node
    DestroyNode { root_id: String, node_id: u64 },
    /// Clear the container (root)
    ClearContainer { root_id: String },
    /// Signal that a batch of operations is complete
    Complete,
}

/// Thread-safe receiver wrapper for the Bevy system
pub struct ReactClientReceiver {
    rx: Arc<Mutex<Receiver<ReactClientProto>>>,
}

impl ReactClientReceiver {
    /// Try to receive the next message without blocking
    pub fn try_recv(&self) -> Option<ReactClientProto> {
        let rx = self.rx.lock().ok()?;
        rx.try_recv().ok()
    }
}

impl ReactClient {
    /// Create a new ReactClient and its corresponding receiver
    pub fn new() -> (ReactClient, ReactClientReceiver) {
        // Unbounded: JS must never block inside `execute(flush_events)` while
        // committing binary/enum ops (bounded sync_channel deadlocks at capacity).
        let (tx, rx) = mpsc::channel();

        (
            ReactClient { tx },
            ReactClientReceiver { rx: Arc::new(Mutex::new(rx)) },
        )
    }

    /// Request a new unique node ID (thread-safe atomic operation)
    pub fn next_id(&self) -> u64 {
        NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Send an RPC message; log and drop if the Bevy receiver is gone.
    fn send(&self, msg: ReactClientProto) {
        if let Err(e) = self.tx.send(msg) {
            log::error!("React RPC channel closed: {e}");
        }
    }

    /// Create a new UI node
    pub fn create_node(&self, root_id: String, node_type: String, props_json: String) -> u64 {
        let node_id = self.next_id();
        log::debug!(
            "ReactClient::create_node id={} type={} props={}",
            node_id,
            node_type,
            props_json
        );
        self.send(ReactClientProto::CreateNode {
            root_id,
            node_id,
            node_type,
            props_json,
        });
        node_id
    }

    /// Create a new text node
    pub fn create_text(&self, root_id: String, content: String) -> u64 {
        let node_id = self.next_id();
        log::debug!(
            "ReactClient::create_text id={} content={}",
            node_id,
            content
        );
        self.send(ReactClientProto::CreateText {
            root_id,
            node_id,
            content,
        });
        node_id
    }

    /// Append a child to a parent
    pub fn append_child(&self, root_id: String, parent_id: u64, child_id: u64) {
        log::debug!(
            "ReactClient::append_child parent={} child={}",
            parent_id,
            child_id
        );
        self.send(ReactClientProto::AppendChild {
            root_id,
            parent_id,
            child_id,
        });
    }

    /// Insert a child before another child
    pub fn insert_before(&self, root_id: String, parent_id: u64, child_id: u64, before_id: u64) {
        log::debug!(
            "ReactClient::insert_before parent={} child={} before={}",
            parent_id,
            child_id,
            before_id
        );
        self.send(ReactClientProto::InsertBefore {
            root_id,
            parent_id,
            child_id,
            before_id,
        });
    }

    /// Remove a child from a parent
    pub fn remove_child(&self, root_id: String, parent_id: u64, child_id: u64) {
        log::debug!(
            "ReactClient::remove_child parent={} child={}",
            parent_id,
            child_id
        );
        self.send(ReactClientProto::RemoveChild {
            root_id,
            parent_id,
            child_id,
        });
    }

    /// Update node properties
    pub fn update_node(&self, root_id: String, node_id: u64, props_json: String) {
        log::debug!(
            "ReactClient::update_node id={} props={}",
            node_id,
            props_json
        );
        self.send(ReactClientProto::UpdateNode {
            root_id,
            node_id,
            props_json,
        });
    }

    /// Update text content
    pub fn update_text(&self, root_id: String, node_id: u64, content: String) {
        log::debug!("ReactClient::update_text id={} content={}", node_id, content);
        self.send(ReactClientProto::UpdateText {
            root_id,
            node_id,
            content,
        });
    }

    /// Destroy a node
    pub fn destroy_node(&self, root_id: String, node_id: u64) {
        log::debug!("ReactClient::destroy_node id={}", node_id);
        self.send(ReactClientProto::DestroyNode { root_id, node_id });
    }

    /// Clear the container
    pub fn clear_container(&self, root_id: String) {
        log::debug!("ReactClient::clear_container");
        self.send(ReactClientProto::ClearContainer { root_id });
    }

    /// Signal completion
    pub fn complete(&self) {
        self.send(ReactClientProto::Complete);
    }

    /// Decode a BRRP binary batch and enqueue the resulting RPC messages.
    ///
    /// Used by the `binary_ops` feature path (`__react_commit_ops`). Per-op
    /// natives remain available as a dual path (force with `binaryOps: false`).
    ///
    /// Advances [`NODE_ID_COUNTER`] past any create ids so a later enum-path
    /// allocation cannot collide with JS-allocated binary ids.
    pub fn commit_binary_ops(&self, bytes: &[u8]) -> Result<(), crate::react::proto::DecodeError> {
        let msgs = crate::react::proto::decode_protos(bytes)?;
        for msg in &msgs {
            if let Some(id) = proto_node_id(msg) {
                bump_node_id_counter_to_at_least(id);
            }
        }
        for msg in msgs {
            self.send(msg);
        }
        Ok(())
    }
}

fn proto_node_id(msg: &ReactClientProto) -> Option<u64> {
    match msg {
        ReactClientProto::CreateNode { node_id, .. }
        | ReactClientProto::CreateText { node_id, .. } => Some(*node_id),
        _ => None,
    }
}

fn bump_node_id_counter_to_at_least(id: u64) {
    loop {
        let cur = NODE_ID_COUNTER.load(Ordering::SeqCst);
        if cur >= id {
            return;
        }
        if NODE_ID_COUNTER
            .compare_exchange(cur, id, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return;
        }
    }
}
