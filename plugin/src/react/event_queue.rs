//! Native React event queue (host → JS) and focus command bridge (JS → host).
//!
//! Bevy systems push structured events here; a fixed `__react_flush_events()` script
//! drains the queue on the JS thread and invokes the registered dispatcher callback.
//! No module-name string interpolation or async `import()` for event delivery.
//!
//! JS `__react_request_focus` / `__react_request_blur` enqueue [`ReactFocusCommand`]s
//! that Bevy drains alongside [`RequestReactFocus`](crate::react::systems::RequestReactFocus)
//! messages.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bevy::input::keyboard::Key;
use bevy::input::mouse::MouseScrollUnit;
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use boa_gc::{Finalize, Trace, empty_trace};
use serde_json::{Value, json};

/// A single UI event to deliver into the JS React tree.
#[derive(Clone, Debug)]
pub struct ReactEvent {
    pub root_id: String,
    pub node_id: u64,
    pub event_type: String,
    /// JSON object (or `null`) passed as the 4th argument to the JS dispatcher.
    pub payload_json: String,
}

/// Programmatic focus/blur requested from JS native functions.
#[derive(Clone, Debug)]
pub enum ReactFocusCommand {
    Focus {
        node_id: u64,
        root_id: Option<String>,
    },
    Blur,
}

#[derive(Default)]
struct ReactEventQueueInner {
    events: VecDeque<ReactEvent>,
    focus_commands: VecDeque<ReactFocusCommand>,
}

/// Thread-safe queue shared between Bevy systems and Boa native functions.
#[derive(Clone, Default, Finalize, Resource)]
pub struct ReactEventQueue {
    inner: Arc<Mutex<ReactEventQueueInner>>,
}

unsafe impl Trace for ReactEventQueue {
    empty_trace!();
}

impl ReactEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, event: ReactEvent) {
        if let Ok(mut q) = self.inner.lock() {
            q.events.push_back(event);
        }
    }

    pub fn push_event(
        &self,
        root_id: impl Into<String>,
        node_id: u64,
        event_type: impl Into<String>,
        payload: Value,
    ) {
        self.push(ReactEvent {
            root_id: root_id.into(),
            node_id,
            event_type: event_type.into(),
            payload_json: payload.to_string(),
        });
    }

    /// Enqueue a focus request from JS (`__react_request_focus`).
    pub fn request_focus(&self, node_id: u64, root_id: Option<String>) {
        if let Ok(mut q) = self.inner.lock() {
            q.focus_commands
                .push_back(ReactFocusCommand::Focus { node_id, root_id });
        }
    }

    /// Enqueue a blur request from JS (`__react_request_blur`).
    pub fn request_blur(&self) {
        if let Ok(mut q) = self.inner.lock() {
            q.focus_commands.push_back(ReactFocusCommand::Blur);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .lock()
            .map(|q| q.events.is_empty())
            .unwrap_or(true)
    }

    pub fn drain(&self) -> Vec<ReactEvent> {
        self.inner
            .lock()
            .map(|mut q| q.events.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn drain_focus_commands(&self) -> Vec<ReactFocusCommand> {
        self.inner
            .lock()
            .map(|mut q| q.focus_commands.drain(..).collect())
            .unwrap_or_default()
    }
}

/// Fixed script executed on the JS thread to drain [`ReactEventQueue`].
pub const FLUSH_EVENTS_SCRIPT: &str = "__react_flush_events();";

/// Build a pointer payload from optional relative + window cursor positions.
pub fn pointer_payload(
    relative: Option<&RelativeCursorPosition>,
    window_cursor: Option<Vec2>,
) -> Value {
    if let Some(rel) = relative
        && let Some(normalized) = rel.normalized
    {
        return json!({
            "x": normalized.x,
            "y": normalized.y,
            "normalized": true,
            "cursorOver": rel.cursor_over,
        });
    }

    if let Some(pos) = window_cursor {
        return json!({
            "x": pos.x,
            "y": pos.y,
            "normalized": false,
        });
    }

    Value::Null
}

/// Build a wheel event payload (DOM-like `deltaX` / `deltaY` / `deltaMode`).
///
/// `deltaMode`: `0` = pixel, `1` = line (matches `WheelEvent.DOM_DELTA_*`).
pub fn wheel_payload(delta: Vec2, unit: MouseScrollUnit) -> Value {
    let delta_mode = match unit {
        MouseScrollUnit::Pixel => 0,
        MouseScrollUnit::Line => 1,
    };
    json!({
        "deltaX": delta.x,
        "deltaY": delta.y,
        "deltaMode": delta_mode,
    })
}

/// Build a scroll position payload after applying a wheel delta.
pub fn scroll_payload(scroll_left: f32, scroll_top: f32, delta: Vec2) -> Value {
    json!({
        "scrollLeft": scroll_left,
        "scrollTop": scroll_top,
        "deltaX": delta.x,
        "deltaY": delta.y,
    })
}

/// Convert Bevy logical [`Key`] to a DOM-like `KeyboardEvent.key` string.
pub fn logical_key_to_string(key: &Key) -> String {
    match key {
        Key::Character(s) => s.to_string(),
        Key::Unidentified(_) => "Unidentified".to_string(),
        Key::Dead(_) => "Dead".to_string(),
        Key::Alt | Key::AltGraph => "Alt".to_string(),
        Key::CapsLock => "CapsLock".to_string(),
        Key::Control => "Control".to_string(),
        Key::Fn => "Fn".to_string(),
        Key::FnLock => "FnLock".to_string(),
        Key::NumLock => "NumLock".to_string(),
        Key::ScrollLock => "ScrollLock".to_string(),
        Key::Shift => "Shift".to_string(),
        Key::Symbol => "Symbol".to_string(),
        Key::SymbolLock => "SymbolLock".to_string(),
        Key::Meta => "Meta".to_string(),
        Key::Hyper => "Hyper".to_string(),
        Key::Super => "Super".to_string(),
        Key::Enter => "Enter".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::Space => " ".to_string(),
        Key::ArrowDown => "ArrowDown".to_string(),
        Key::ArrowLeft => "ArrowLeft".to_string(),
        Key::ArrowRight => "ArrowRight".to_string(),
        Key::ArrowUp => "ArrowUp".to_string(),
        Key::End => "End".to_string(),
        Key::Home => "Home".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::Insert => "Insert".to_string(),
        Key::Escape => "Escape".to_string(),
        Key::ContextMenu => "ContextMenu".to_string(),
        other => {
            // Named keys (media, browser, etc.): Debug name without the enum path noise.
            let debug = format!("{other:?}");
            debug
                .strip_prefix("Key::")
                .unwrap_or(&debug)
                .to_string()
        }
    }
}

/// Keyboard modifier flags from [`ButtonInput<KeyCode>`].
pub fn keyboard_modifiers(keyboard: &ButtonInput<KeyCode>) -> Value {
    json!({
        "shiftKey": keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        "ctrlKey": keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
        "altKey": keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
        "metaKey": keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]),
    })
}
