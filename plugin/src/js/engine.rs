//! JavaScript Engine
//!
//! Manages the Boa JavaScript runtime with a dedicated worker thread
//! and proper event loop integration.

use boa_engine::module::ModuleLoader;
use boa_engine::{Context, JsError, JsNativeError, JsObject, JsString, Module, Source};
use boa_runtime::extensions::{
    ConsoleExtension, MicrotaskExtension, TimeoutExtension, UrlExtension,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::protocol::{JsCallback, JsCallbackRegistry};
use super::websocket::WebSocketJsCallback;

/// Commands that can be sent to the JS engine thread.
#[derive(Debug)]
pub enum JsCommand {
    /// Load an ESM module with the given name and source.
    LoadEsmModule { name: String, source: String },
    /// Execute a JS script (non-module).
    Execute { source: String },
    /// Tick the event loop (run pending jobs and timers).
    Tick,
    /// Shutdown the JS engine.
    Shutdown,
}

/// Client handle for communicating with the JS engine thread.
///
/// This is cheap to clone and can be shared across threads.
/// Uses interior mutability to satisfy thread-safety requirements.
#[derive(Clone)]
pub struct JsEngineClient {
    sender: Arc<Mutex<Sender<JsCommand>>>,
}

impl JsEngineClient {
    /// Send a tick command to flush the JS event loop.
    pub fn tick(&self) {
        if let Ok(sender) = self.sender.lock() {
            if let Err(e) = sender.send(JsCommand::Tick) {
                log::warn!("Failed to send tick command: {}", e);
            }
        }
    }

    /// Load an ES module.
    pub fn load_esm_module(&self, name: impl Into<String>, source: impl Into<String>) {
        if let Ok(sender) = self.sender.lock() {
            if let Err(e) = sender.send(JsCommand::LoadEsmModule {
                name: name.into(),
                source: source.into(),
            }) {
                log::error!("Failed to send load ESM module command: {}", e);
            }
        }
    }

    /// Execute a script.
    pub fn execute(&self, source: impl Into<String>) {
        if let Ok(sender) = self.sender.lock() {
            if let Err(e) = sender.send(JsCommand::Execute {
                source: source.into(),
            }) {
                log::error!("Failed to send execute command: {}", e);
            }
        }
    }

    /// Shutdown the JS engine.
    pub fn shutdown(&self) {
        if let Ok(sender) = self.sender.lock() {
            let _ = sender.send(JsCommand::Shutdown);
        }
    }
}

/// JavaScript engine with dedicated worker thread.
pub struct JsEngine {
    client: JsEngineClient,
    _handle: JoinHandle<()>,
}

impl JsEngine {
    /// Start a new JS engine with the given callback registry.
    pub fn start(callbacks: JsCallbackRegistry) -> Self {
        let (sender, receiver) = mpsc::channel();
        let client = JsEngineClient {
            sender: Arc::new(Mutex::new(sender)),
        };

        // Clone client for the JS thread (needed for WebSocket event dispatch)
        let client_for_thread = client.clone();

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_js_loop(receiver, callbacks, client_for_thread);
            }));

            if let Err(e) = result {
                log::error!("JS engine panicked: {:?}", e);
            }
        });

        Self {
            client,
            _handle: handle,
        }
    }

    /// Get a client handle for communicating with the engine.
    pub fn client(&self) -> JsEngineClient {
        self.client.clone()
    }
}

struct FetchModuleLoader {
    pub local_modules: RefCell<HashMap<String, Module>>,
}

impl FetchModuleLoader {
    pub fn insert(&self, specifier: impl Into<String>, module: Module) {
        let specifier = specifier.into();
        self.local_modules
            .borrow_mut()
            .insert(specifier.clone(), module);
        log::info!("Cached local module: {}", specifier);
    }
}

impl Default for FetchModuleLoader {
    fn default() -> Self {
        Self {
            local_modules: RefCell::new(HashMap::new()),
        }
    }
}

/// Static Tokio runtime for async operations (e.g. module loading, WebSocket).
pub(crate) static TOKIO: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime")
});

impl ModuleLoader for FetchModuleLoader {
    fn init_import_meta(
        self: Rc<Self>,
        import_meta: &JsObject,
        module: &Module,
        context: &mut Context,
    ) {
        log::info!("Initializing import meta");
        import_meta.set(
            JsString::from_str("url").unwrap(),
            JsString::from_str(module.path().unwrap().to_string_lossy().as_ref()).unwrap(),
            false,
            context,
        ).unwrap();
    }

    async fn load_imported_module(
        self: std::rc::Rc<Self>,
        referrer: boa_engine::module::Referrer,
        specifier: boa_engine::JsString,
        context: &std::cell::RefCell<&mut boa_engine::Context>,
    ) -> boa_engine::JsResult<boa_engine::Module> {
        log::debug!(
            "Loading imported module: {}, referrer={:?}",
            specifier.to_std_string_escaped(),
            referrer.path()
        );

        // Resolve the specifier first, then check cache with the resolved key.
        let referrer_path = referrer.path();
        let resolved_specifier = match referrer_path {
            Some(path) => {
                let spec_str = specifier.to_std_string_lossy();

                // Try to resolve as absolute URL, otherwise resolve as relative URL with base.
                if let Ok(base_url) = url::Url::parse(&path.to_string_lossy()) {
                    // Always use URL resolution for anything that parses as a URL.
                    match url::Url::options()
                        .base_url(Some(&base_url))
                        .parse(&spec_str)
                    {
                        Ok(new_url) => new_url.to_string(),
                        Err(_) => spec_str.clone(),
                    }
                } else {
                    // Local file path logic.
                    let base = Path::new(&path);
                    let joined = if spec_str.starts_with('/') {
                        PathBuf::from(spec_str.clone())
                    } else {
                        base.parent().unwrap_or(base).join(&spec_str)
                    };
                    joined.to_string_lossy().to_string()
                }
            }
            None => specifier.to_std_string_lossy(),
        };

        log::debug!("Resolved specifier: {}", resolved_specifier);

        // Check cache with resolved specifier to avoid duplicate loading.
        if let Some(module) = self.local_modules.borrow().get(&resolved_specifier) {
            log::debug!("Cache hit for module: {}", resolved_specifier);
            return Ok(module.clone());
        }

        // Run reqwest in a blocking task using the static runtime,
        // because this might be called from a context without a tokio runtime (e.g. Bevy ECS thread).
        let body = TOKIO.block_on(async {
            let response = reqwest::get(&resolved_specifier).await.map_err(|e| {
                JsError::from_native(
                    JsNativeError::typ()
                        .with_message(format!("Fetch error: {}", e.to_string()))
                        .into(),
                )
            })?;

            let body = response.text().await.map_err(|e| {
                JsError::from_native(
                    JsNativeError::typ()
                        .with_message(format!("Fetch resposne error: {}", e.to_string()))
                        .into(),
                )
            })?;

            Ok::<_, boa_engine::JsError>(body)
        })?;

        let src = Source::from_bytes(body.as_bytes()).with_path(Path::new(&resolved_specifier));
        let module = Module::parse(src, None, &mut context.borrow_mut())?;

        self.insert(resolved_specifier.clone(), module.clone());
        Ok(module)
    }
}

/// Main loop for the JS engine thread.
fn run_js_loop(receiver: Receiver<JsCommand>, callbacks: JsCallbackRegistry, client: JsEngineClient) {
    log::info!("JS engine thread started");

    let mut context = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::default()))
        // .module_loader(Rc::new(MapModuleLoader::new()))
        .build()
        .unwrap();

    // Register Boa runtime extensions
    if let Err(e) = boa_runtime::register(
        (
            ConsoleExtension::default(),
            TimeoutExtension {},
            MicrotaskExtension {},
            UrlExtension {},
        ),
        None,
        &mut context,
    ) {
        log::error!("Failed to register Boa runtime extensions: {:?}", e);
    }

    // Register environment shims (process, window, MessageChannel, etc.)
    register_environment_shims(&mut context);

    // Register WebSocket support (needed for Vite HMR)
    // Pass the client so WebSocket can push events directly to JS
    let ws_callback = WebSocketJsCallback::new(client);
    ws_callback.register(&mut context);

    // Register all callbacks from the registry
    callbacks.register_all(&mut context);

    log::info!("JS runtime initialized");

    // Process commands
    loop {
        match receiver.recv() {
            Ok(JsCommand::Execute { source }) => {
                log::info!("Executing script ({} bytes)...", source.len());

                let source = Source::from_bytes(source.as_bytes());

                if let Err(e) = context.eval(source) {
                    log::error!("Failed to execute script: {:?}", e);
                }

                flush_event_loop(&mut context);
            }
            Ok(JsCommand::LoadEsmModule { name, source }) => {
                log::info!("Loading ES module {} ({} bytes)...", name, source.len());

                let source = Source::from_bytes(source.as_bytes());
                let module = Module::parse(source, None, &mut context).unwrap();
                let _promise = module.load_link_evaluate(&mut context);

                if let Some(loader) = context.downcast_module_loader::<FetchModuleLoader>() {
                    log::info!("Registering ESM module: {}", name);
                    loader.insert(name, module);
                }

                flush_event_loop(&mut context);
            }
            Ok(JsCommand::Tick) => {
                flush_event_loop(&mut context);
            }
            Ok(JsCommand::Shutdown) => {
                log::info!("JS engine shutting down");
                break;
            }
            Err(e) => {
                log::error!("JS engine channel error: {}", e);
                break;
            }
        }
    }

    log::info!("JS engine thread stopped");
}

/// Flush the event loop: process pending events, run microtasks (Jobs), and pending macrotasks (timers).
fn flush_event_loop(context: &mut Context) {
    if let Err(e) = context.run_jobs() {
        if let Some(e) = e.as_opaque() {
            let msg = e.to_json(context).unwrap_or_default();
            log::error!("Error running Boa jobs: {:?}", msg);
        } else {
            log::error!("Error running Boa jobs: {:?}", e);
        }
    }
}

/// Register environment shims for browser/Node.js compatibility.
fn register_environment_shims(context: &mut Context) {
    let shims = r#"
(function() {
    // 1. Global Object & Window
    globalThis.window = globalThis;
    globalThis.self = globalThis;

    // 2. Location (needed for URL resolution with relative paths)
    globalThis.location = {
        href: 'http://localhost:5173/',
        origin: 'http://localhost:5173',
        protocol: 'http:',
        host: 'localhost:5173',
        hostname: 'localhost',
        port: '5173',
        pathname: '/',
        search: '',
        hash: ''
    };

    // 3. Process Environment (needed for React production/development mode checks)
    globalThis.process = {
        env: {
            NODE_ENV: 'development' 
        }
    };

    // 4. Event Loop & Timers
    // We maintain a priority queue of timers
    var timers = [];
    var timerIdCounter = 0;

    function schedule_interval(callback, delay, id) {
         timers.push({
            id: id,
            callback: function() {
                callback();
                schedule_interval(callback, delay, id);
            },
            args: [],
            dueTime: Date.now() + delay
        });
    }

    // 5. RequestAnimationFrame (simulated with setTimeout)
    globalThis.requestAnimationFrame = function(callback) {
        return setTimeout(function() { callback(Date.now()); }, 16);
    };
    
    globalThis.cancelAnimationFrame = function(id) {
        clearTimeout(id);
    };

    // 6. MessageChannel (React Scheduler)
    // Uses setTimeout(0) to schedule a macrotask, yielding to the event loop.
    globalThis.MessageChannel = function MessageChannel() {
        var self = this;
        this.port1 = {
            onmessage: null,
            postMessage: function(data) {
                if (self.port2.onmessage) {
                    setTimeout(function() {
                        self.port2.onmessage({ data: data });
                    }, 0);
                }
            }
        };
        this.port2 = {
            onmessage: null,
            postMessage: function(data) {
                if (self.port1.onmessage) {
                    setTimeout(function() {
                        self.port1.onmessage({ data: data });
                    }, 0);
                }
            }
        };
    };

    // 7. Performance
    if (!globalThis.performance) {
        globalThis.performance = {
            now: function() { return Date.now(); }
        };
    }

    console.log('[Shims] Environment initialized (window, process, event loop)');
})();
    "#;

    if let Err(e) = context.eval(Source::from_bytes(shims.as_bytes())) {
        log::error!("Failed to set up environment shims: {:?}", e);
    }
}
