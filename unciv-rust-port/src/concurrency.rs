use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::{self, Thread},
    time::Duration,
};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc,
    task::JoinHandle,
};
use anyhow::Result;
use log::{error, warn};

/// A wrapper around thread pools and coroutine dispatchers
pub struct Concurrency {
    runtime: Arc<Runtime>,
    daemon_threads: Arc<Mutex<Vec<Thread>>>,
    non_daemon_threads: Arc<Mutex<Vec<Thread>>>,
}

impl Concurrency {
    /// Creates a new Concurrency instance with initialized thread pools
    pub fn new() -> Self {
        let runtime = Builder::new_multi_thread()
            .worker_threads(num_cpus::get())
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        Self {
            runtime: Arc::new(runtime),
            daemon_threads: Arc::new(Mutex::new(Vec::new())),
            non_daemon_threads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Runs a blocking task on a non-daemon thread pool
    pub fn run_blocking<F, T>(&self, name: Option<&str>, f: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.runtime.spawn_blocking(f);
        match handle.blocking_wait() {
            Ok(result) => Some(result),
            Err(e) => {
                error!("Error in blocking task {}: {}", name.unwrap_or("unnamed"), e);
                None
            }
        }
    }

    /// Runs a non-blocking task on a daemon thread pool
    pub fn run<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.runtime.spawn(async move {
            if let Err(e) = std::panic::catch_unwind(f) {
                error!("Error in task {}: {:?}", name.unwrap_or("unnamed"), e);
            }
        })
    }

    /// Runs a task on a non-daemon thread pool
    pub fn run_on_non_daemon_thread_pool<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel(1);
        let handle = self.runtime.spawn(async move {
            if let Err(e) = std::panic::catch_unwind(f) {
                error!("Error in non-daemon task {}: {:?}", name.unwrap_or("unnamed"), e);
            }
            let _ = tx.send(()).await;
        });

        // Store the thread handle
        if let Ok(mut threads) = self.non_daemon_threads.lock() {
            threads.push(thread::current());
        }

        handle
    }

    /// Runs a task on the GL thread (in this case, we'll use the main thread)
    pub fn run_on_gl_thread<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.runtime.spawn(async move {
            if let Err(e) = std::panic::catch_unwind(f) {
                error!("Error in GL task {}: {:?}", name.unwrap_or("unnamed"), e);
            }
        })
    }

    /// Stops all thread pools
    pub fn stop_thread_pools(&self) {
        // In Rust, we don't need to explicitly stop thread pools
        // The runtime will be dropped when the Concurrency instance is dropped
        warn!("Thread pools will be stopped when the Concurrency instance is dropped");
    }
}

impl Drop for Concurrency {
    fn drop(&mut self) {
        // Clean up thread pools
        self.runtime.shutdown_timeout(Duration::from_secs(1));
    }
}

// Extension traits for easier usage
pub trait ConcurrencyExt {
    fn launch_on_thread_pool<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static;

    fn launch_on_non_daemon_thread_pool<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static;

    fn launch_on_gl_thread<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static;
}

impl ConcurrencyExt for Concurrency {
    fn launch_on_thread_pool<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.run(name, f)
    }

    fn launch_on_non_daemon_thread_pool<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.run_on_non_daemon_thread_pool(name, f)
    }

    fn launch_on_gl_thread<F>(&self, name: Option<&str>, f: F) -> JoinHandle<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.run_on_gl_thread(name, f)
    }
}

// Global instance
lazy_static::lazy_static! {
    pub static ref CONCURRENCY: Arc<Concurrency> = Arc::new(Concurrency::new());
}