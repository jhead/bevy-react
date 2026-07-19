//! Shared JS → Rust error reporting for console, uncaught rejections, and engine failures.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use boa_engine::{Context, JsError, JsValue, js_string};

/// Where a reported JS runtime error originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsErrorSource {
    Console,
    UncaughtRejection,
    Script,
    ModuleLoad,
    Job,
    Panic,
}

/// One captured JS / engine error with optional stack text.
#[derive(Debug, Clone)]
pub struct JsErrorRecord {
    pub message: String,
    pub stack: Option<String>,
    pub source: JsErrorSource,
}

/// Thread-safe sink for the latest JS runtime error and engine generation.
///
/// Bevy polls this each frame into [`crate::js_bevy::JsRuntimeError`].
#[derive(Clone, Debug, Default)]
pub struct JsErrorReporter {
    latest: Arc<Mutex<Option<JsErrorRecord>>>,
    generation: Arc<AtomicU64>,
}

impl JsErrorReporter {
    pub fn report(&self, record: JsErrorRecord) {
        match record.source {
            JsErrorSource::Console => {
                // console.error already logged by the logger; still surface as error state.
            }
            JsErrorSource::UncaughtRejection
            | JsErrorSource::Script
            | JsErrorSource::ModuleLoad
            | JsErrorSource::Job => {
                if let Some(ref stack) = record.stack {
                    log::error!(
                        "JS {:?}: {}\n{}",
                        record.source,
                        record.message,
                        stack
                    );
                } else {
                    log::error!("JS {:?}: {}", record.source, record.message);
                }
            }
            JsErrorSource::Panic => {
                log::error!("JS engine panic: {}", record.message);
            }
        }

        if let Ok(mut slot) = self.latest.lock() {
            *slot = Some(record);
        }
    }

    pub fn report_message(&self, source: JsErrorSource, message: impl Into<String>, stack: Option<String>) {
        self.report(JsErrorRecord {
            message: message.into(),
            stack,
            source,
        });
    }

    /// Take the latest error, if any (clears the slot).
    pub fn take(&self) -> Option<JsErrorRecord> {
        self.latest.lock().ok().and_then(|mut slot| slot.take())
    }

    /// Peek at the latest error without clearing.
    pub fn latest(&self) -> Option<JsErrorRecord> {
        self.latest.lock().ok().and_then(|slot| slot.clone())
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Bump after a native JS-thread restart so Bevy can observe recovery.
    pub fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// Format a [`JsError`] into message + optional `.stack`.
pub fn format_js_error(error: &JsError, context: &mut Context) -> (String, Option<String>) {
    if let Some(value) = error.as_opaque() {
        return format_js_value(value, context);
    }
    (format!("{error:?}"), None)
}

/// Format a JS value (often an `Error`) into message + optional `.stack`.
pub fn format_js_value(value: &JsValue, context: &mut Context) -> (String, Option<String>) {
    let message = value
        .to_string(context)
        .map(|s| s.to_std_string_lossy())
        .unwrap_or_else(|_| value.display().to_string());

    let stack = value.as_object().and_then(|obj| {
        let stack_val = obj.get(js_string!("stack"), context).ok()?;
        let stack_str = stack_val.as_string()?;
        let s = stack_str.to_std_string_lossy();
        if s.is_empty() { None } else { Some(s) }
    });

    (message, stack)
}

/// Append Boa's VM call-frame names when available.
pub fn append_vm_stack(stack: Option<String>, context: &Context) -> Option<String> {
    let frames: Vec<String> = context
        .stack_trace()
        .map(|frame| frame.code_block().name().to_std_string_escaped())
        .collect();
    if frames.is_empty() {
        return stack;
    }
    let dump = frames.join("\n");
    Some(match stack {
        Some(existing) => format!("{existing}\n{dump}"),
        None => dump,
    })
}
