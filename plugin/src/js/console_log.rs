//! Forward `console.*` to the Rust `log` crate.

use boa_engine::{Context, JsResult};
use boa_gc::{Finalize, Trace};
use boa_runtime::console::{ConsoleState, Logger};

use crate::js::error_report::{
    JsErrorReporter, JsErrorSource, append_vm_stack,
};

/// Logger that maps JS console levels onto `log::{info,warn,error,debug,trace}`.
#[derive(Debug, Trace, Finalize)]
pub struct RustLogLogger {
    #[unsafe_ignore_trace]
    reporter: JsErrorReporter,
}

impl RustLogLogger {
    pub fn new(reporter: JsErrorReporter) -> Self {
        Self { reporter }
    }
}

impl Logger for RustLogLogger {
    fn log(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        log::info!("{msg:>indent$}");
        Ok(())
    }

    fn info(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        self.log(msg, state, context)
    }

    fn warn(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        log::warn!("{msg:>indent$}");
        Ok(())
    }

    fn error(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        let stack = append_vm_stack(None, context);
        if let Some(ref stack) = stack {
            log::error!("{msg:>indent$}\n{stack}");
        } else {
            log::error!("{msg:>indent$}");
        }
        self.reporter.report_message(
            JsErrorSource::Console,
            msg,
            stack,
        );
        Ok(())
    }

    fn debug(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        log::debug!("{msg:>indent$}");
        Ok(())
    }

    fn trace(&self, msg: String, state: &ConsoleState, context: &mut Context) -> JsResult<()> {
        let indent = state.indent();
        log::trace!("{msg:>indent$}");
        for frame in context.stack_trace() {
            let name = frame.code_block().name().to_std_string_escaped();
            log::trace!("{name:>indent$}");
        }
        Ok(())
    }
}
