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

/// Format a [`JsError`] into message + optional stack.
///
/// Prefers Boa's [`Display`] output when it includes shadow-stack frames
/// (`at name (path:line:col)`), which source-map enrichment can rewrite.
pub fn format_js_error(error: &JsError, context: &mut Context) -> (String, Option<String>) {
    let display = format!("{error}");
    if let Some((message, stack)) = split_boa_display_stack(&display) {
        return (message, Some(stack));
    }
    if let Some(value) = error.as_opaque() {
        return format_js_value(value, context);
    }
    (display, None)
}

/// Split `TypeError: msg\n    at foo (url:1:2)` into message + stack body.
fn split_boa_display_stack(display: &str) -> Option<(String, String)> {
    let idx = display.find("\n    at ")?;
    let message = display[..idx].to_string();
    let stack = display[idx + 1..].to_string(); // keep leading `    at …`
    if stack.trim().is_empty() {
        return None;
    }
    Some((message, stack))
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

/// Append Boa VM frames with `path:line:column` when available.
///
/// Uses [`boa_engine::vm::CallFrame::position`] so Vite module URLs can be
/// source-mapped by [`crate::js::sourcemap_enrich`].
pub fn append_vm_stack(stack: Option<String>, context: &Context) -> Option<String> {
    let frames: Vec<String> = context
        .stack_trace()
        .map(|frame| format_call_frame(frame))
        .collect();
    if frames.is_empty() {
        return stack;
    }
    let dump = frames.join("\n");
    Some(match stack {
        Some(existing) if !existing.is_empty() => {
            // Avoid duplicating frames already present on Error.stack / Display.
            if existing.contains("    at ") && frames.iter().all(|f| existing.contains(f.trim())) {
                existing
            } else {
                format!("{existing}\n{dump}")
            }
        }
        Some(_) | None => dump,
    })
}

fn format_call_frame(frame: &boa_engine::vm::CallFrame) -> String {
    use boa_engine::vm::SourcePath;

    let loc = frame.position();
    let raw_name = loc.function_name.to_std_string_escaped();
    let name = if raw_name.is_empty() {
        "<anonymous>"
    } else {
        raw_name.as_str()
    };

    match &loc.path {
        SourcePath::Path(path) => {
            let path_str = normalize_path_url(&path.to_string_lossy());
            match loc.position {
                Some(pos) => format!(
                    "    at {name} ({path_str}:{}:{})",
                    pos.line_number(),
                    pos.column_number()
                ),
                None => format!("    at {name} ({path_str})"),
            }
        }
        SourcePath::Eval => format!("    at {name} (eval)"),
        SourcePath::Json => format!("    at {name} (JSON.parse)"),
        SourcePath::None => format!("    at {name}"),
    }
}

/// Repair Path-collapsed `http:/host` → `http://host` for stack consumers.
fn normalize_path_url(path: &str) -> String {
    let mut u = path.to_string();
    for (bad, good) in [
        ("https:/", "https://"),
        ("http:/", "http://"),
        ("file:/", "file://"),
    ] {
        if u.starts_with(bad) && !u.starts_with(good) {
            u = format!("{good}{}", &u[bad.len()..]);
            break;
        }
    }
    u
}
