use std::{
    rc::Rc,
    sync::mpsc::{self, Receiver},
};

use boa_engine::{Context, JsError};
use boa_runtime::extensions::{ConsoleExtension, MicrotaskExtension, TimeoutExtension};

use crate::js::{JsCommand, JsEngine, JsEngineClient, esm::FetchModuleLoader};

pub struct JsEngineBuilder {
    extensions: Vec<Box<dyn JsEngineExtension>>,
    client: JsEngineClient,
    receiver: Receiver<JsCommand>,
}

impl JsEngineBuilder {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        JsEngineBuilder {
            extensions: vec![],
            client: JsEngineClient { sender },
            receiver,
        }
    }

    pub fn with_extension(mut self, extension: impl JsEngineExtension) -> Self {
        self.extensions.push(Box::new(extension));
        self
    }

    pub fn build(self) -> Result<JsEngine, JsError> {
        let client = self.client.clone();
        let extensions = self.extensions;

        Ok(JsEngine {
            client: self.client,
            context_builder: Box::new(move || build_context(&extensions, client.clone())),
            receiver: self.receiver,
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
    let mut context = Context::builder()
        .module_loader(Rc::new(FetchModuleLoader::new()))
        .build()?;

    // Register Boa runtime extensions
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
