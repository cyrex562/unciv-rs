use std::sync::Arc;
use std::thread;

/// Minimal concurrency compatibility shim for legacy code
pub struct Concurrency;

impl Concurrency {
    /// Run a function on the main thread (in Rust, just runs it immediately)
    pub fn run_on_main_thread<F: FnOnce() + Send + 'static>(f: F) {
        // In a real GUI/game engine, this would post to the main thread event loop.
        // Here, we just run it directly.
        f();
    }
    /// Run a function in a new thread
    pub fn run<F: FnOnce() + Send + 'static>(_name: &str, f: F) -> thread::JoinHandle<()> {
        thread::spawn(f)
    }
}

// For compatibility with code expecting a static instance
lazy_static::lazy_static! {
    pub static ref CONCURRENCY: Arc<Concurrency> = Arc::new(Concurrency);
}
