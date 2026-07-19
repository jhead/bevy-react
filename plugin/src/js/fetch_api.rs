//! Host `fetch()` backed by reqwest, gated on the crate `fetch` feature.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::{Context, JsData, JsResult, JsString, js_error};
use boa_gc::{Finalize, Trace};
use boa_runtime::fetch::Fetcher;
use boa_runtime::fetch::request::JsRequest;
use boa_runtime::fetch::response::JsResponse;

/// Reqwest-backed [`Fetcher`] for `globalThis.fetch`.
///
/// Native uses the blocking client (Boa's default job executor drives async
/// fetch via `futures_lite::block_on`, which cannot poll Tokio futures).
/// WASM awaits the async client; [`FrameJobExecutor`] polls it each frame.
#[derive(Debug, Trace, Finalize, JsData)]
pub struct ReqwestFetcher {
    #[cfg(not(target_arch = "wasm32"))]
    #[unsafe_ignore_trace]
    blocking: reqwest::blocking::Client,
    #[cfg(target_arch = "wasm32")]
    #[unsafe_ignore_trace]
    client: reqwest::Client,
}

impl Default for ReqwestFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestFetcher {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            blocking: reqwest::blocking::Client::new(),
            #[cfg(target_arch = "wasm32")]
            client: reqwest::Client::new(),
        }
    }
}

impl Fetcher for ReqwestFetcher {
    async fn fetch(
        self: Rc<Self>,
        request: JsRequest,
        _context: &RefCell<&mut Context>,
    ) -> JsResult<JsResponse> {
        let request = request.into_inner();
        let url = request.uri().to_string();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut builder = self
                .blocking
                .request(request.method().clone(), &url);
            for (key, value) in request.headers() {
                builder = builder.header(key, value);
            }
            let body = request.body().clone();
            let builder = if body.is_empty() {
                builder
            } else {
                builder.body(body)
            };

            let resp = builder.send().map_err(|e| {
                js_error!(TypeError: "fetch failed: {}", e)
            })?;

            let status = resp.status();
            let headers = resp.headers().clone();
            let bytes = resp.bytes().map_err(|e| {
                js_error!(TypeError: "fetch body error: {}", e)
            })?;

            let mut http_builder = http::Response::builder().status(status.as_u16());
            for key in headers.keys() {
                for value in headers.get_all(key) {
                    http_builder = http_builder.header(key.as_str(), value);
                }
            }

            http_builder
                .body(bytes.to_vec())
                .map_err(|e| js_error!(TypeError: "fetch response build error: {}", e))
                .map(|response| JsResponse::basic(JsString::from(url), response))
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut builder = self.client.request(request.method().clone(), &url);
            for (key, value) in request.headers() {
                builder = builder.header(key, value);
            }
            let body = request.body().clone();
            let builder = if body.is_empty() {
                builder
            } else {
                builder.body(body)
            };

            let resp = builder.send().await.map_err(|e| {
                js_error!(TypeError: "fetch failed: {}", e)
            })?;

            let status = resp.status();
            let headers = resp.headers().clone();
            let bytes = resp.bytes().await.map_err(|e| {
                js_error!(TypeError: "fetch body error: {}", e)
            })?;

            let mut http_builder = http::Response::builder().status(status.as_u16());
            for key in headers.keys() {
                for value in headers.get_all(key) {
                    http_builder = http_builder.header(key.as_str(), value);
                }
            }

            http_builder
                .body(bytes.to_vec())
                .map_err(|e| js_error!(TypeError: "fetch response build error: {}", e))
                .map(|response| JsResponse::basic(JsString::from(url), response))
        }
    }
}
