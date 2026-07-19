use boa_engine::{Context, Source};

/// Register environment shims for browser/Node.js compatibility.
///
/// Timers (`setTimeout` / `setInterval`) come from `boa_runtime::TimeoutExtension`
/// and are drained by the job executor — do not reimplement them here.
pub(crate) fn register_environment_shims(context: &mut Context) {
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

    // 4. RequestAnimationFrame (simulated with setTimeout from boa_runtime)
    globalThis.requestAnimationFrame = function(callback) {
        return setTimeout(function() { callback(Date.now()); }, 16);
    };
    
    globalThis.cancelAnimationFrame = function(id) {
        clearTimeout(id);
    };

    // 5. MessageChannel (React Scheduler)
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

    // 6. Performance
    if (!globalThis.performance) {
        globalThis.performance = {
            now: function() { return Date.now(); }
        };
    }

    // 7. Date locale shims — Boa has no Intl; Date#toLocaleString throws
    // "Function Unimplemented" and will blank a React tree that uses it.
    (function() {
        function formatDateFallback(date, _locales, options) {
            try {
                if (options && options.hour !== undefined) {
                    var h = date.getHours();
                    var m = date.getMinutes();
                    var s = date.getSeconds();
                    var pad = function(n) { return n < 10 ? '0' + n : '' + n; };
                    return pad(h) + ':' + pad(m) + ':' + pad(s);
                }
            } catch (e) {}
            return date.toISOString();
        }
        var proto = Date.prototype;
        var methods = ['toLocaleString', 'toLocaleDateString', 'toLocaleTimeString'];
        for (var i = 0; i < methods.length; i++) {
            (function(name) {
                var original = proto[name];
                proto[name] = function(locales, options) {
                    try {
                        if (typeof original === 'function') {
                            return original.call(this, locales, options);
                        }
                    } catch (e) {}
                    return formatDateFallback(this, locales, options);
                };
            })(methods[i]);
        }
        if (typeof globalThis.Intl === 'undefined') {
            globalThis.Intl = {
                DateTimeFormat: function(_locales, options) {
                    this.format = function(value) {
                        var d = value instanceof Date ? value : new Date(value);
                        return formatDateFallback(d, _locales, options);
                    };
                },
                NumberFormat: function() {
                    this.format = function(value) { return String(value); };
                }
            };
        }
    })();

    console.log('[Shims] Environment initialized (window, process, rAF, MessageChannel, Date locale)');
})();
    "#;

    if let Err(e) = context.eval(Source::from_bytes(shims.as_bytes())) {
        log::error!("Failed to set up environment shims: {:?}", e);
    }
}
