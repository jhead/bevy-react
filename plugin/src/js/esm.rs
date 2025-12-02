use boa_engine::module::ModuleLoader;
use boa_engine::{Context, JsError, JsNativeError, JsObject, JsString, Module, Source};
use tokio::runtime::{self, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) struct FetchModuleLoader {
    runtime: Runtime,
    local_modules: RefCell<HashMap<String, Module>>,
}

impl FetchModuleLoader {
    pub(crate) fn new() -> Self {
        Self {
            runtime: runtime::Builder::new_multi_thread().enable_all().build().unwrap(),
            local_modules: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert(&self, specifier: impl Into<String>, module: Module) {
        let specifier = specifier.into();
        self.local_modules
            .borrow_mut()
            .insert(specifier.clone(), module);
        log::info!("Cached local module: {}", specifier);
    }
}

impl ModuleLoader for FetchModuleLoader {
    fn init_import_meta(
        self: Rc<Self>,
        import_meta: &JsObject,
        module: &Module,
        context: &mut Context,
    ) {
        log::info!("Initializing import meta");

        let Some(module_path) =  module.path().map(|path| path.to_string_lossy().to_string()) else {
            log::warn!("Module path is None while initializing import_meta");
            return;
        };

        // Set import_meta.url = module.path
        if let Err(e) = import_meta.set(JsString::from("url"),JsString::from(module_path), false, context) {
            log::warn!("Failed to set 'url' in import_meta: {:?}", e);
        }
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
        let body = self.runtime.block_on(async {
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
