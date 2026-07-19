//! Host hooks: uncaught promise rejections → Rust log + error reporter.

use boa_engine::builtins::promise::{OperationType, PromiseState};
use boa_engine::context::HostHooks;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsObject};

use crate::js::error_report::{
    JsErrorReporter, JsErrorSource, append_vm_stack, format_js_value,
};

/// [`HostHooks`] that forward unhandled promise rejections to `log` / [`JsErrorReporter`].
pub struct LoggingHostHooks {
    pub reporter: JsErrorReporter,
}

impl HostHooks for LoggingHostHooks {
    fn promise_rejection_tracker(
        &self,
        promise: &JsObject,
        operation: OperationType,
        context: &mut Context,
    ) {
        if operation != OperationType::Reject {
            return;
        }

        let Ok(js_promise) = JsPromise::from_object(promise.clone()) else {
            return;
        };

        let PromiseState::Rejected(reason) = js_promise.state() else {
            return;
        };

        let (message, stack) = format_js_value(&reason, context);
        let stack = append_vm_stack(stack, context);
        self.reporter
            .report_message(JsErrorSource::UncaughtRejection, message, stack);
    }
}
