//! Shared JS → Rust error reporting for console, uncaught rejections, and engine failures.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use boa_engine::{Context, JsError, JsResult, JsString, JsValue, NativeFunction, js_string};
use boa_gc::{Finalize, Trace, empty_trace};

use crate::js::sourcemap_enrich::enrich_stack_with_sourcemaps;

/// Where a reported JS runtime error originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsErrorSource {
    Console,
    UncaughtRejection,
    Script,
    ModuleLoad,
    Job,
    Panic,
    /// Explicit report from React (`__react_report_error` / ErrorBoundary).
    React,
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
#[derive(Clone, Debug, Default, Finalize)]
pub struct JsErrorReporter {
    latest: Arc<Mutex<Option<JsErrorRecord>>>,
    generation: Arc<AtomicU64>,
}

unsafe impl Trace for JsErrorReporter {
    empty_trace!();
}

impl JsErrorReporter {
    pub fn report(&self, mut record: JsErrorRecord) {
        record.stack = enrich_stack_with_sourcemaps(record.stack);

        match record.source {
            JsErrorSource::Console => {
                // console.error already logged by the logger; still surface as error state.
            }
            JsErrorSource::UncaughtRejection
            | JsErrorSource::Script
            | JsErrorSource::ModuleLoad
            | JsErrorSource::Job
            | JsErrorSource::React => {
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

    pub fn report_message(
        &self,
        source: JsErrorSource,
        message: impl Into<String>,
        stack: Option<String>,
    ) {
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

/// Register `__react_report_error(message, stack?)` for React / app code.
pub fn register_report_error(
    context: &mut Context,
    reporter: JsErrorReporter,
) -> Result<(), JsError> {
    context.register_global_callable(
        JsString::from("__react_report_error"),
        2,
        NativeFunction::from_copy_closure_with_captures(
            move |_this: &JsValue, args: &[JsValue], reporter: &JsErrorReporter, ctx: &mut Context| {
                report_error_fn(args, reporter, ctx)
            },
            reporter,
        ),
    )?;
    Ok(())
}

fn report_error_fn(
    args: &[JsValue],
    reporter: &JsErrorReporter,
    context: &mut Context,
) -> JsResult<JsValue> {
    let message = args
        .first()
        .map(|v| {
            v.to_string(context)
                .map(|s| s.to_std_string_lossy())
                .unwrap_or_else(|_| v.display().to_string())
        })
        .unwrap_or_else(|| "Unknown error".to_string());

    let stack = args.get(1).and_then(|v| {
        if v.is_undefined() || v.is_null() {
            return None;
        }
        let s = v
            .to_string(context)
            .ok()?
            .to_std_string_lossy();
        if s.is_empty() { None } else { Some(s) }
    });

    reporter.report_message(JsErrorSource::React, message, stack);
    Ok(JsValue::undefined())
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
