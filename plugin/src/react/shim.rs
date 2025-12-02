use boa_engine::{Context, Source};

/// Register environment shims for browser/Node.js compatibility.
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
