use boa_gc::{Finalize, Trace, empty_trace};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Mutex;

/// Global counter for node IDs (used across threads)
static NODE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Thread-safe client for sending React RPC messages to the Bevy main thread
#[derive(Clone, Debug, Finalize)]
pub struct ReactClient {
    tx: SyncSender<ReactClientProto>,
}

unsafe impl Trace for ReactClient {
    empty_trace!();
}

/// RPC Protocol messages from JS React reconciler to Bevy
#[derive(Clone, Debug)]
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
    rx: Mutex<Receiver<ReactClientProto>>,
}

impl ReactClientReceiver {
    /// Try to receive the next message without blocking
    pub fn try_recv(&self) -> Option<ReactClientProto> {
        self.rx.lock().ok()?.try_recv().ok()
    }
}

// Implement Send + Sync for ReactClientReceiver
unsafe impl Send for ReactClientReceiver {}
unsafe impl Sync for ReactClientReceiver {}

impl ReactClient {
    /// Create a new ReactClient and its corresponding receiver
    pub fn new() -> (ReactClient, ReactClientReceiver) {
        // Main message channel (bounded for backpressure)
        let (tx, rx) = mpsc::sync_channel(256);

        (
            ReactClient { tx },
            ReactClientReceiver { rx: Mutex::new(rx) },
        )
    }

    /// Request a new unique node ID (thread-safe atomic operation)
    pub fn next_id(&self) -> u64 {
        NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1
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
        self.tx
            .send(ReactClientProto::CreateNode {
                root_id,
                node_id,
                node_type,
                props_json,
            })
            .expect("Failed to send CreateNode");
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
        self.tx
            .send(ReactClientProto::CreateText { root_id, node_id, content })
            .expect("Failed to send CreateText");
        node_id
    }

    /// Append a child to a parent
    pub fn append_child(&self, root_id: String, parent_id: u64, child_id: u64) {
        log::debug!(
            "ReactClient::append_child parent={} child={}",
            parent_id,
            child_id
        );
        self.tx
            .send(ReactClientProto::AppendChild {
                root_id,
                parent_id,
                child_id,
            })
            .expect("Failed to send AppendChild");
    }

    /// Remove a child from a parent
    pub fn remove_child(&self, root_id: String, parent_id: u64, child_id: u64) {
        log::debug!(
            "ReactClient::remove_child parent={} child={}",
            parent_id,
            child_id
        );
        self.tx
            .send(ReactClientProto::RemoveChild {
                root_id,
                parent_id,
                child_id,
            })
            .expect("Failed to send RemoveChild");
    }

    /// Update node properties
    pub fn update_node(&self, root_id: String, node_id: u64, props_json: String) {
        log::debug!(
            "ReactClient::update_node id={} props={}",
            node_id,
            props_json
        );
        self.tx
            .send(ReactClientProto::UpdateNode {
                root_id,
                node_id,
                props_json,
            })
            .expect("Failed to send UpdateNode");
    }

    /// Update text content
    pub fn update_text(&self, root_id: String, node_id: u64, content: String) {
        log::debug!("ReactClient::update_text id={} content={}", node_id, content);
        self.tx
            .send(ReactClientProto::UpdateText { root_id, node_id, content })
            .expect("Failed to send UpdateText");
    }

    /// Destroy a node
    pub fn destroy_node(&self, root_id: String, node_id: u64) {
        log::debug!("ReactClient::destroy_node id={}", node_id);
        self.tx
            .send(ReactClientProto::DestroyNode { root_id, node_id })
            .expect("Failed to send DestroyNode");
    }

    /// Clear the container
    pub fn clear_container(&self, root_id: String) {
        log::debug!("ReactClient::clear_container");
        self.tx
            .send(ReactClientProto::ClearContainer { root_id })
            .expect("Failed to send ClearContainer");
    }

    /// Signal completion
    pub fn complete(&self) {
        self.tx
            .send(ReactClientProto::Complete)
            .expect("Failed to send Complete");
    }
}
