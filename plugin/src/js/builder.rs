use std::rc::Rc;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
#[cfg(target_arch = "wasm32")]
use std::{
    collections::VecDeque,
    sync::Mutex,
};

use boa_engine::{Context, JsError};
use boa_runtime::extensions::{ConsoleExtension, MicrotaskExtension, TimeoutExtension};
#[cfg(feature = "fetch")]
use boa_runtime::extensions::FetchExtension;

/// WASM-compatible clock for Boa — uses js_sys::Date::now() instead of
/// std::time::SystemTime::now(), which panics on wasm32-unknown-unknown.
#[cfg(target_arch = "wasm32")]
struct WasmClock;

#[cfg(target_arch = "wasm32")]
impl boa_engine::context::time::Clock for WasmClock {
    fn now(&self) -> boa_engine::context::time::JsInstant {
        let millis = js_sys::Date::now() as u64;
        boa_engine::context::time::JsInstant::new(millis / 1000, ((millis % 1000) * 1_000_000) as u32)
    }
}

#[cfg(not(target_arch = "wasm32"))]
use crate::js::JsCommand;
use crate::js::console_log::RustLogLogger;
use crate::js::error_report::JsErrorReporter;
use crate::js::host_hooks::LoggingHostHooks;
use crate::js::{JsEngine, JsEngineClient, esm::FetchModuleLoader};
#[cfg(feature = "fetch")]
use crate::js::fetch_api::ReqwestFetcher;

pub struct JsEngineBuilder {
    extensions: Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: Receiver<JsCommand>,
}

impl Default for JsEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JsEngineBuilder {
    pub fn new() -> Self {
        let reporter = JsErrorReporter::default();

        #[cfg(not(target_arch = "wasm32"))]
        let (sender, receiver) = mpsc::channel::<JsCommand>();

        JsEngineBuilder {
            extensions: vec![],
            client: JsEngineClient {
                #[cfg(not(target_arch = "wasm32"))]
                sender,
                // WASM: context slot starts empty; filled by JsEngine::start().
                #[cfg(target_arch = "wasm32")]
                context: Arc::new(Mutex::new(None)),
                // WASM: command queue instead of an mpsc channel.
                #[cfg(target_arch = "wasm32")]
                queue: Arc::new(Mutex::new(VecDeque::new())),
                reporter,
            },
            #[cfg(not(target_arch = "wasm32"))]
            receiver,
        }
    }

    pub fn with_extension(mut self, extension: impl JsEngineExtension) -> Self {
        self.extensions.push(Box::new(extension));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn build(self) -> Result<JsEngine, JsError> {
        let client = self.client.clone();
        let extensions: Arc<Vec<Box<dyn JsEngineExtension>>> = Arc::new(self.extensions);
        let reporter = client.reporter.clone();

        Ok(JsEngine {
            client: self.client,
            context_builder: Box::new(move || {
                build_context(extensions.as_slice(), client.clone(), reporter.clone())
            }),
            receiver: self.receiver,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn build(self) -> Result<JsEngine, JsError> {
        // Build and register extensions synchronously — the client's queue is available
        // for any extension that enqueues commands during registration; they'll be
        // processed on the first flush_event_loop call after start().
        let context = build_context(
            self.extensions.as_slice(),
            self.client.clone(),
            self.client.reporter.clone(),
        )?;

        Ok(JsEngine {
            client: self.client,
            context,
        })
    }
}

pub trait JsEngineExtension: Send + Sync + 'static {
    fn register(&self, context: &mut Context, client: JsEngineClient) -> Result<(), JsError>;
}

pub(crate) fn build_context(
    extensions: &[Box<dyn JsEngineExtension>],
    client: JsEngineClient,
    reporter: JsErrorReporter,
) -> Result<Context, JsError> {
    let host_hooks = Rc::new(LoggingHostHooks {
        reporter: reporter.clone(),
    });

    #[cfg(not(target_arch = "wasm32"))]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()))
        .host_hooks(host_hooks);

    // WASM: install a frame-budgeted job executor. The default SimpleJobExecutor
    // calls futures_lite::block_on, which busy-spins on wasm32 and gets the
    // browser to kill the script. FrameJobExecutor pumps ready work each Bevy
    // frame and returns without waiting for future timeouts / pending fetches.
    #[cfg(target_arch = "wasm32")]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()))
        .clock(Rc::new(WasmClock))
        .job_executor(Rc::new(frame_jobs::FrameJobExecutor::new()))
        .host_hooks(host_hooks);

    let mut context = context_builder.build()?;

    boa_runtime::register(
        (
            ConsoleExtension(RustLogLogger::new(reporter)),
            TimeoutExtension {},
            MicrotaskExtension {},
        ),
        None,
        &mut context,
    )?;

    #[cfg(feature = "fetch")]
    {
        use boa_runtime::extensions::RuntimeExtension;
        FetchExtension(ReqwestFetcher::new()).register(None, &mut context)?;
    }

    for extension in extensions {
        extension.register(&mut context, client.clone())?;
    }

    Ok(context)
}

/// Budgeted, non-blocking job executor for wasm32.
///
/// Unlike [`boa_engine::job::SimpleJobExecutor`], this never calls `block_on`.
/// Each `run_jobs` invocation (driven by Bevy's per-frame `flush_event_loop`)
/// runs a bounded amount of ready work and returns, leaving future timeouts and
/// pending async fetches for subsequent frames.
///
/// # Soundness model (`NativeAsyncJob` across frames)
///
/// Boa's [`NativeAsyncJob::call`] returns a future that borrows
/// `&RefCell<&mut Context>`. [`SimpleJobExecutor`] keeps that `RefCell` on the
/// stack for the entire `block_on(run_jobs_async)` — impossible on wasm32
/// without busy-spin. We instead:
///
/// 1. Own an [`Rc`]-backed [`ContextGate`] (Drop-able; **no** `Box::leak`).
/// 2. **Attach** the live `Context` raw pointer at the start of each `run_jobs`
///    and **detach** (null) before returning — pending futures may only be
///    polled while attached. After attach, sync and async work both go through
///    the gate cell (never the original `&mut Context` parameter) to avoid
///    overlapping exclusive borrows.
/// 3. Start jobs against a layout-reinterpreted `&RefCell<&mut Context>` so
///    the returned future is naturally `'static` relative to the gate allocation
///    (no `mem::transmute` of the future itself).
/// 4. Only call `run_jobs` while the engine mutex holds the same `Context`
///    object (address-stable in `JsEngineClient`); if the address changes while
///    futures are pending, drop them rather than polling a stale pointer.
///
/// Drop order: `pending_async` is declared before `gate` so in-flight futures
/// are dropped while the gate allocation still exists.
#[cfg(target_arch = "wasm32")]
mod frame_jobs {
    use boa_engine::context::time::JsInstant;
    use boa_engine::job::{
        GenericJob, Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob,
    };
    use boa_engine::{Context, JsResult, JsValue};
    use std::cell::{Cell, RefCell};
    use std::collections::{BTreeMap, VecDeque};
    use std::future::Future;
    use std::mem;
    use std::pin::Pin;
    use std::ptr;
    use std::rc::Rc;
    use std::task::{Context as TaskContext, Poll, Waker};

    /// Max drain iterations per `run_jobs` call — keeps a frame from spinning
    /// if jobs keep re-enqueueing (e.g. microtask storms).
    const MAX_TURNS: usize = 64;

    type PendingFuture = Pin<Box<dyn Future<Output = JsResult<JsValue>> + 'static>>;

    /// Shared slot that pending `NativeAsyncJob` futures borrow.
    ///
    /// Stores a raw `*mut Context` (not a Rust reference) so attach/detach can
    /// refresh the pointer each frame without extending a real `&mut` past
    /// `run_jobs`. Boa's API still sees `RefCell<&mut Context>` via
    /// [`ContextGate::as_boa_cell`].
    struct ContextGate {
        ptr: RefCell<*mut Context>,
        /// Last non-null pointer written by `attach`, for stability checks.
        attached_addr: Cell<*mut Context>,
    }

    impl ContextGate {
        fn new() -> Self {
            Self {
                ptr: RefCell::new(ptr::null_mut()),
                attached_addr: Cell::new(ptr::null_mut()),
            }
        }

        fn attach_ptr(&self, p: *mut Context) -> AttachGuard<'_> {
            debug_assert!(!p.is_null());
            *self.ptr.borrow_mut() = p;
            self.attached_addr.set(p);
            AttachGuard { gate: self }
        }

        fn detach(&self) {
            *self.ptr.borrow_mut() = ptr::null_mut();
        }

        /// Reinterpret the raw-pointer cell as the type [`NativeAsyncJob::call`] needs.
        ///
        /// # Safety
        ///
        /// - `*mut Context` and `&mut Context` are layout-compatible (thin pointers).
        /// - Callers may only `borrow_mut` through the returned cell while `attach_ptr`
        ///   has stored a live, uniquely usable `Context` pointer (engine mutex held,
        ///   wasm32 single-threaded). Detach nulls the pointer before `run_jobs` returns.
        /// - After `attach_ptr`, the original `&mut Context` from `run_jobs` must not be
        ///   used again for the rest of the call — all access goes through this cell so
        ///   async and sync jobs never form overlapping `&mut Context`s.
        unsafe fn as_boa_cell(&self) -> &RefCell<&mut Context> {
            unsafe {
                &*( &self.ptr as *const RefCell<*mut Context>
                    as *const RefCell<&mut Context>)
            }
        }
    }

    /// RAII: null the gate pointer when leaving `run_jobs` so a stray poll after
    /// detach cannot observe a dangling `&mut Context`.
    struct AttachGuard<'a> {
        gate: &'a ContextGate,
    }

    impl Drop for AttachGuard<'_> {
        fn drop(&mut self) {
            self.gate.detach();
        }
    }

    pub(super) struct FrameJobExecutor {
        promise_jobs: RefCell<VecDeque<PromiseJob>>,
        async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
        timeout_jobs: RefCell<BTreeMap<JsInstant, TimeoutJob>>,
        generic_jobs: RefCell<VecDeque<GenericJob>>,
        /// In-flight async jobs. Declared before `gate` so Drop runs first.
        pending_async: RefCell<Vec<PendingFuture>>,
        /// Owned gate (via `Rc`) whose allocation outlives pending futures.
        gate: Rc<ContextGate>,
    }

    impl FrameJobExecutor {
        pub(super) fn new() -> Self {
            Self {
                promise_jobs: RefCell::default(),
                async_jobs: RefCell::default(),
                timeout_jobs: RefCell::default(),
                generic_jobs: RefCell::default(),
                pending_async: RefCell::default(),
                gate: Rc::new(ContextGate::new()),
            }
        }

        fn clear(&self) {
            self.promise_jobs.borrow_mut().clear();
            self.async_jobs.borrow_mut().clear();
            self.timeout_jobs.borrow_mut().clear();
            self.generic_jobs.borrow_mut().clear();
            self.pending_async.borrow_mut().clear();
        }

        /// If Context moved/recreated while jobs were pending, drop them — polling
        /// would use a stale pointer through the gate.
        fn ensure_context_stable(&self, context: *mut Context) {
            let prev = self.gate.attached_addr.get();
            if !prev.is_null()
                && prev != context
                && !self.pending_async.borrow().is_empty()
            {
                log::error!(
                    "FrameJobExecutor: Context address changed with {} pending async job(s); dropping them",
                    self.pending_async.borrow().len()
                );
                self.pending_async.borrow_mut().clear();
            }
        }

        fn drain_due_timeouts(&self, context: &mut Context) -> JsResult<usize> {
            let now = context.clock().now();
            let mut timeouts = self.timeout_jobs.borrow_mut();
            let mut keep = timeouts.split_off(&now);
            keep.retain(|_, job| !job.is_cancelled());
            let due = mem::replace(&mut *timeouts, keep);
            drop(timeouts);

            let mut ran = 0;
            for job in due.into_values() {
                job.call(context)?;
                ran += 1;
            }
            Ok(ran)
        }

        fn drain_promise_jobs(&self, context: &mut Context) -> JsResult<usize> {
            let jobs = mem::take(&mut *self.promise_jobs.borrow_mut());
            let ran = jobs.len();
            for job in jobs {
                job.call(context)?;
            }
            Ok(ran)
        }

        fn drain_one_generic(&self, context: &mut Context) -> JsResult<usize> {
            if let Some(job) = self.generic_jobs.borrow_mut().pop_front() {
                job.call(context)?;
                Ok(1)
            } else {
                Ok(0)
            }
        }

        fn has_ready_sync_work(&self, context: &Context) -> bool {
            if !self.promise_jobs.borrow().is_empty() || !self.generic_jobs.borrow().is_empty() {
                return true;
            }
            let now = context.clock().now();
            self.timeout_jobs.borrow().iter().any(|(t, _)| &now >= t)
        }

        /// Poll in-flight host futures once (noop waker — next Bevy frame re-pumps).
        ///
        /// # Safety
        ///
        /// Gate must be attached to a live Context for the duration of this call.
        unsafe fn poll_pending_async(&self) -> JsResult<usize> {
            let waker = Waker::noop();
            let mut task_cx = TaskContext::from_waker(waker);
            let mut pending = self.pending_async.borrow_mut();
            let mut i = 0;
            let mut completed = 0;
            while i < pending.len() {
                match pending[i].as_mut().poll(&mut task_cx) {
                    Poll::Ready(Ok(_)) => {
                        let _ = pending.swap_remove(i);
                        completed += 1;
                    }
                    Poll::Ready(Err(err)) => {
                        let _ = pending.swap_remove(i);
                        return Err(err);
                    }
                    Poll::Pending => {
                        i += 1;
                    }
                }
            }
            Ok(completed)
        }

        /// Start queued `NativeAsyncJob`s against the attached gate cell.
        ///
        /// # Safety
        ///
        /// `cell` must be the attached gate's boa cell, and must remain valid for
        /// as long as the returned futures stay in `pending_async` (gate `Rc` held
        /// by this executor; pending cleared before `gate` drops).
        unsafe fn start_async_jobs(&self, cell: &'static RefCell<&mut Context>) {
            let jobs = mem::take(&mut *self.async_jobs.borrow_mut());
            if jobs.is_empty() {
                return;
            }
            let mut pending = self.pending_async.borrow_mut();
            for job in jobs {
                // `cell` is `'static` relative to the gate allocation, so the
                // future from `call` is `'static` — no future transmute.
                let fut: PendingFuture = Box::pin(job.call(cell));
                pending.push(fut);
            }
        }
    }

    impl JobExecutor for FrameJobExecutor {
        fn enqueue_job(self: Rc<Self>, job: Job, context: &mut Context) {
            match job {
                Job::PromiseJob(job) => self.promise_jobs.borrow_mut().push_back(job),
                Job::AsyncJob(job) => self.async_jobs.borrow_mut().push_back(job),
                Job::TimeoutJob(job) => {
                    let now = context.clock().now();
                    self.timeout_jobs
                        .borrow_mut()
                        .insert(now + job.timeout(), job);
                }
                Job::GenericJob(job) => self.generic_jobs.borrow_mut().push_back(job),
                _ => log::warn!("FrameJobExecutor: ignoring unsupported job variant"),
            }
        }

        fn run_jobs(self: Rc<Self>, context: &mut Context) -> JsResult<()> {
            // Convert to a raw pointer immediately. After this, all Context access
            // goes through the gate cell (same pattern as SimpleJobExecutor's
            // RefCell), so sync drains and async polls never alias `&mut Context`.
            let ctx_ptr = context as *mut Context;
            self.ensure_context_stable(ctx_ptr);
            let _attach = self.gate.attach_ptr(ctx_ptr);

            // SAFETY: gate allocation lives as long as `self.gate`; attached for
            // this call; detached by `_attach` Drop before return.
            let cell: &'static RefCell<&mut Context> = unsafe {
                let gate: &'static ContextGate = &*Rc::as_ptr(&self.gate);
                gate.as_boa_cell()
            };

            unsafe {
                self.start_async_jobs(cell);
            }

            for _ in 0..MAX_TURNS {
                let async_done = match unsafe { self.poll_pending_async() } {
                    Ok(n) => n,
                    Err(err) => {
                        self.clear();
                        return Err(err);
                    }
                };

                unsafe {
                    self.start_async_jobs(cell);
                }

                let sync_ran = match (|| {
                    let mut n = 0;
                    {
                        let mut ctx = cell.borrow_mut();
                        n += self.drain_due_timeouts(&mut ctx)?;
                        n += self.drain_one_generic(&mut ctx)?;
                        n += self.drain_promise_jobs(&mut ctx)?;
                        ctx.clear_kept_objects();
                    }
                    Ok::<usize, boa_engine::JsError>(n)
                })() {
                    Ok(n) => n,
                    Err(err) => {
                        self.clear();
                        return Err(err);
                    }
                };

                unsafe {
                    self.start_async_jobs(cell);
                }

                let idle = {
                    let ctx = cell.borrow();
                    async_done == 0 && sync_ran == 0 && !self.has_ready_sync_work(&ctx)
                };
                if idle {
                    // Fully drained, or only future timeouts / Pending host
                    // futures remain — yield back to Bevy/browser.
                    break;
                }
            }

            Ok(())
        }
    }
}
