use std::rc::Rc;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
#[cfg(target_arch = "wasm32")]
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use boa_engine::{Context, JsError};

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
use boa_runtime::extensions::{ConsoleExtension, MicrotaskExtension, TimeoutExtension};

#[cfg(not(target_arch = "wasm32"))]
use crate::js::JsCommand;
use crate::js::{JsEngine, JsEngineClient, esm::FetchModuleLoader};

pub struct JsEngineBuilder {
    extensions: Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: Receiver<JsCommand>,
}

impl JsEngineBuilder {
    pub fn new() -> Self {
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
        let extensions = self.extensions;

        Ok(JsEngine {
            client: self.client,
            context_builder: Box::new(move || build_context(&extensions, client.clone())),
            receiver: self.receiver,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn build(self) -> Result<JsEngine, JsError> {
        // Build and register extensions synchronously — the client's queue is available
        // for any extension that enqueues commands during registration; they'll be
        // processed on the first flush_event_loop call after start().
        let context = build_context(&self.extensions, self.client.clone())?;

        Ok(JsEngine {
            client: self.client,
            context,
        })
    }
}

pub trait JsEngineExtension: Send + Sync + 'static {
    fn register(&self, context: &mut Context, client: JsEngineClient) -> Result<(), JsError>;
}

fn build_context(
    extensions: &Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
) -> Result<Context, JsError> {
    #[cfg(not(target_arch = "wasm32"))]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()));

    // WASM: install a frame-budgeted job executor. The default SimpleJobExecutor
    // calls futures_lite::block_on, which busy-spins on wasm32 and gets the
    // browser to kill the script. FrameJobExecutor pumps ready work each Bevy
    // frame and returns without waiting for future timeouts / pending fetches.
    #[cfg(target_arch = "wasm32")]
    let context_builder = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()))
        .clock(Rc::new(WasmClock))
        .job_executor(Rc::new(frame_jobs::FrameJobExecutor::new()));

    let mut context = context_builder.build()?;

    boa_runtime::register(
        (
            ConsoleExtension::default(),
            TimeoutExtension {},
            MicrotaskExtension {},
        ),
        None,
        &mut context,
    )?;

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
#[cfg(target_arch = "wasm32")]
mod frame_jobs {
    use boa_engine::context::time::JsInstant;
    use boa_engine::job::{
        GenericJob, Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob,
    };
    use boa_engine::{Context, JsResult};
    use std::cell::{Cell, RefCell};
    use std::collections::{BTreeMap, VecDeque};
    use std::future::Future;
    use std::mem;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context as TaskContext, Poll, Waker};

    /// Max drain iterations per `run_jobs` call — keeps a frame from spinning
    /// if jobs keep re-enqueueing (e.g. microtask storms).
    const MAX_TURNS: usize = 64;

    pub(super) struct FrameJobExecutor {
        promise_jobs: RefCell<VecDeque<PromiseJob>>,
        async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
        timeout_jobs: RefCell<BTreeMap<JsInstant, TimeoutJob>>,
        generic_jobs: RefCell<VecDeque<GenericJob>>,
        /// In-flight NativeAsyncJob futures (lifetime-erased; see `run_jobs`).
        pending_async: RefCell<Vec<Pin<Box<dyn Future<Output = JsResult<JsValue>> + 'static>>>>,
        /// Leaked cell that pending futures borrow. Holds a raw context pointer
        /// refreshed at the start of each pump. See SAFETY on `context_cell`.
        context_cell: Cell<Option<&'static RefCell<*mut Context>>>,
    }

    // JsValue is needed for the Future output type alias above.
    use boa_engine::JsValue;

    impl FrameJobExecutor {
        pub(super) fn new() -> Self {
            Self {
                promise_jobs: RefCell::default(),
                async_jobs: RefCell::default(),
                timeout_jobs: RefCell::default(),
                generic_jobs: RefCell::default(),
                pending_async: RefCell::default(),
                context_cell: Cell::new(None),
            }
        }

        fn clear(&self) {
            self.promise_jobs.borrow_mut().clear();
            self.async_jobs.borrow_mut().clear();
            self.timeout_jobs.borrow_mut().clear();
            self.generic_jobs.borrow_mut().clear();
            self.pending_async.borrow_mut().clear();
        }

        fn context_cell(&self, context: &mut Context) -> &'static RefCell<*mut Context> {
            let ptr = context as *mut Context;
            match self.context_cell.get() {
                Some(cell) => {
                    *cell.borrow_mut() = ptr;
                    cell
                }
                None => {
                    let cell: &'static RefCell<*mut Context> =
                        Box::leak(Box::new(RefCell::new(ptr)));
                    self.context_cell.set(Some(cell));
                    cell
                }
            }
        }

        /// View the raw-pointer cell as the `&RefCell<&mut Context>` that
        /// [`NativeAsyncJob::call`] expects.
        ///
        /// # Safety
        ///
        /// Callers must ensure `cell` currently stores a valid `*mut Context`
        /// uniquely usable for the duration of any borrow_mut taken through the
        /// returned reference (wasm32 is single-threaded; we only poll while the
        /// engine mutex is held in `flush_event_loop`).
        unsafe fn as_context_ref_cell(
            cell: &'static RefCell<*mut Context>,
        ) -> &'static RefCell<&'static mut Context> {
            unsafe { &*(cell as *const RefCell<*mut Context> as *const RefCell<&'static mut Context>) }
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

        fn poll_pending_async(&self) -> JsResult<usize> {
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

        fn start_async_jobs(&self, cell: &'static RefCell<&'static mut Context>) {
            let jobs = mem::take(&mut *self.async_jobs.borrow_mut());
            if jobs.is_empty() {
                return;
            }
            let mut pending = self.pending_async.borrow_mut();
            for job in jobs {
                // Coerce to dyn Future first (thin → fat pointer), then erase the
                // context-borrow lifetime so the future can resume on later frames.
                let fut: Pin<Box<dyn Future<Output = JsResult<JsValue>> + '_>> =
                    Box::pin(job.call(cell));
                // SAFETY: Futures are only polled from `run_jobs` while
                // `context_cell` points at the live engine Context (mutex held).
                // Lifetimes are erased so in-flight host futures (e.g. reqwest
                // fetch) can resume on later Bevy frames after returning to the
                // browser event loop.
                let fut_static: Pin<Box<dyn Future<Output = JsResult<JsValue>> + 'static>> =
                    unsafe { mem::transmute(fut) };
                pending.push(fut_static);
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
            let raw_cell = self.context_cell(context);
            // SAFETY: pointer just written above; wasm32 single-threaded flush.
            let cell = unsafe { Self::as_context_ref_cell(raw_cell) };

            self.start_async_jobs(cell);

            for _ in 0..MAX_TURNS {
                // Poll host futures once per turn (no park / block_on).
                let async_done = match self.poll_pending_async() {
                    Ok(n) => n,
                    Err(err) => {
                        self.clear();
                        return Err(err);
                    }
                };

                // Start any async jobs enqueued by the poll above.
                self.start_async_jobs(cell);

                let sync_ran = match (|| {
                    let mut n = 0;
                    n += self.drain_due_timeouts(context)?;
                    n += self.drain_one_generic(context)?;
                    n += self.drain_promise_jobs(context)?;
                    context.clear_kept_objects();
                    Ok::<usize, boa_engine::JsError>(n)
                })() {
                    Ok(n) => n,
                    Err(err) => {
                        self.clear();
                        return Err(err);
                    }
                };

                // Newly enqueued async work from sync jobs.
                self.start_async_jobs(cell);

                if async_done == 0 && sync_ran == 0 && !self.has_ready_sync_work(context) {
                    // Idle: either fully drained, or only future timeouts /
                    // Pending host futures remain — yield back to Bevy/browser.
                    break;
                }
            }

            Ok(())
        }
    }
}
