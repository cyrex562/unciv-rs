use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    sync::{Arc, Mutex},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

/// Tag for identifying the source of a log message
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Tag {
    pub name: String,
}

impl Tag {
    /// Create a new tag with the given name
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

/// Backend for logging functionality
pub trait LogBackend: Send + Sync {
    /// Log a debug message
    fn debug(&self, tag: &Tag, cur_thread_name: &str, msg: &str);

    /// Log an error message
    fn error(&self, tag: &Tag, cur_thread_name: &str, msg: &str);

    /// Check if this is a release build
    fn is_release(&self) -> bool;

    /// Get system information
    fn get_system_info(&self) -> String;
}

/// Default implementation of LogBackend for testing
pub struct DefaultLogBackend;

impl LogBackend for DefaultLogBackend {
    fn debug(&self, tag: &Tag, cur_thread_name: &str, msg: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        println!("{} [{}] [{}] {}", now, cur_thread_name, tag.name, msg);
    }

    fn error(&self, tag: &Tag, cur_thread_name: &str, msg: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        println!(
            "{} [{}] [{}] [ERROR] {}",
            now, cur_thread_name, tag.name, msg
        );
    }

    fn is_release(&self) -> bool {
        false
    }

    fn get_system_info(&self) -> String {
        String::new()
    }
}

/// Main logging utility
pub struct Log {
    disable_logs_from: Arc<Mutex<HashSet<String>>>,
    enable_logs_from: Arc<Mutex<HashSet<String>>>,
    backend: Arc<dyn LogBackend>,
}

impl Log {
    /// Create a new Log instance
    pub fn new(backend: Arc<dyn LogBackend>) -> Self {
        let disable_logs_from = env::var("noLog")
            .unwrap_or_else(|_| {
                "Battle,Music,Sounds,Translations,WorkerAutomation,assignRegions".to_string()
            })
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let enable_logs_from = env::var("onlyLog")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        Self {
            disable_logs_from: Arc::new(Mutex::new(disable_logs_from)),
            enable_logs_from: Arc::new(Mutex::new(enable_logs_from)),
            backend,
        }
    }

    /// Check if a tag should be logged
    pub fn should_log(&self, tag: &Tag) -> bool {
        !self.backend.is_release() && !self.is_tag_disabled(tag)
    }

    /// Log a debug message with optional parameters
    pub fn debug(&self, msg: &str, params: &[&dyn fmt::Debug]) {
        if self.backend.is_release() {
            return;
        }
        self.debug_with_tag(&self.get_tag(), msg, params);
    }

    /// Log a debug message with a specific tag and optional parameters
    pub fn debug_with_tag(&self, tag: &Tag, msg: &str, params: &[&dyn fmt::Debug]) {
        if !self.should_log(tag) {
            return;
        }

        let formatted_message = if params.is_empty() {
            msg.to_string()
        } else {
            format_message(msg, params)
        };

        self.do_log(
            |t, thread_name, m| self.backend.debug(t, thread_name, m),
            tag,
            &formatted_message,
        );
    }

    /// Log a debug message with a throwable
    pub fn debug_with_throwable(&self, msg: &str, throwable: &dyn std::error::Error) {
        if self.backend.is_release() {
            return;
        }
        self.debug_with_tag_and_throwable(&self.get_tag(), msg, throwable);
    }

    /// Log a debug message with a specific tag and throwable
    pub fn debug_with_tag_and_throwable(
        &self,
        tag: &Tag,
        msg: &str,
        throwable: &dyn std::error::Error,
    ) {
        if !self.should_log(tag) {
            return;
        }

        let throwable_message = build_throwable_message(msg, throwable);

        self.do_log(
            |t, thread_name, m| self.backend.debug(t, thread_name, m),
            tag,
            &throwable_message,
        );
    }

    /// Log an error message with optional parameters
    pub fn error(&self, msg: &str, params: &[&dyn fmt::Debug]) {
        self.error_with_tag(&self.get_tag(), msg, params);
    }

    /// Log an error message with a specific tag and optional parameters
    pub fn error_with_tag(&self, tag: &Tag, msg: &str, params: &[&dyn fmt::Debug]) {
        let formatted_message = if params.is_empty() {
            msg.to_string()
        } else {
            format_message(msg, params)
        };

        self.do_log(
            |t, thread_name, m| self.backend.error(t, thread_name, m),
            tag,
            &formatted_message,
        );
    }

    /// Log an error message with a throwable
    pub fn error_with_throwable(&self, msg: &str, throwable: &dyn std::error::Error) {
        self.error_with_tag_and_throwable(&self.get_tag(), msg, throwable);
    }

    /// Log an error message with a specific tag and throwable
    pub fn error_with_tag_and_throwable(
        &self,
        tag: &Tag,
        msg: &str,
        throwable: &dyn std::error::Error,
    ) {
        let throwable_message = build_throwable_message(msg, throwable);

        self.do_log(
            |t, thread_name, m| self.backend.error(t, thread_name, m),
            tag,
            &throwable_message,
        );
    }

    /// Get system information
    pub fn get_system_info(&self) -> String {
        self.backend.get_system_info()
    }

    /// Check if a tag is disabled
    fn is_tag_disabled(&self, tag: &Tag) -> bool {
        let disable_logs = self.disable_logs_from.lock().unwrap();
        let enable_logs = self.enable_logs_from.lock().unwrap();

        disable_logs.iter().any(|s| tag.name.contains(s))
            || (!enable_logs.is_empty() && !enable_logs.iter().any(|s| tag.name.contains(s)))
    }

    /// Get the current tag based on the call stack
    fn get_tag(&self) -> Tag {
        let backtrace = std::backtrace::Backtrace::capture();
        let frames = backtrace.frames();

        for frame in frames.iter() {
            if let Some(symbols) = frame.symbols() {
                for symbol in symbols {
                    if let Some(name) = symbol.name() {
                        let name_str = name.to_string();
                        if !name_str.contains("com::unciv::utils::Log") {
                            let simple_class_name = name_str.split('.').last().unwrap_or(&name_str);
                            return Tag::new(remove_anonymous_suffix(simple_class_name));
                        }
                    }
                }
            }
        }

        Tag::new("Unknown".to_string())
    }

    /// Execute a log operation
    fn do_log<F>(&self, logger: F, tag: &Tag, msg: &str)
    where
        F: FnOnce(&Tag, &str, &str),
    {
        let thread_name = thread::current().name().unwrap_or("unnamed").to_string();
        logger(tag, &thread_name, msg);
    }
}

/// Format a message with parameters
fn format_message(msg: &str, params: &[&dyn fmt::Debug]) -> String {
    if params.is_empty() {
        return msg.to_string();
    }

    let mut result = msg.to_string();
    for (i, param) in params.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        let value = format!("{:?}", param);
        result = result.replace(&placeholder, &value);
    }

    result
}

/// Build a message with throwable information
fn build_throwable_message(msg: &str, throwable: &dyn std::error::Error) -> String {
    format!("{} | {}", msg, throwable)
}

/// Remove anonymous class suffix from a tag
fn remove_anonymous_suffix(tag: &str) -> String {
    lazy_static! {
        static ref ANONYMOUS_CLASS_PATTERN: Regex = Regex::new(r"(\$\d+)+$").unwrap();
    }

    ANONYMOUS_CLASS_PATTERN.replace_all(tag, "").to_string()
}

// Global instance
lazy_static! {
    static ref LOG: Arc<Log> = Arc::new(Log::new(Arc::new(DefaultLogBackend)));
}

/// Initialize the global Log instance with a custom backend
pub fn init_log(backend: Arc<dyn LogBackend>) {
    unsafe {
        let log_ptr = &LOG as *const Arc<Log> as *mut Arc<Log>;
        *log_ptr = Arc::new(Log::new(backend));
    }
}

/// Get the global Log instance
pub fn get_log() -> Arc<Log> {
    LOG.clone()
}

/// Shortcut for Log::debug
pub fn debug(msg: &str, params: &[&dyn fmt::Debug]) {
    LOG.debug(msg, params);
}

/// Shortcut for Log::debug_with_tag
pub fn debug_with_tag(tag: &Tag, msg: &str, params: &[&dyn fmt::Debug]) {
    LOG.debug_with_tag(tag, msg, params);
}

/// Shortcut for Log::debug_with_throwable
pub fn debug_with_throwable(msg: &str, throwable: &dyn std::error::Error) {
    LOG.debug_with_throwable(msg, throwable);
}

/// Shortcut for Log::debug_with_tag_and_throwable
pub fn debug_with_tag_and_throwable(tag: &Tag, msg: &str, throwable: &dyn std::error::Error) {
    LOG.debug_with_tag_and_throwable(tag, msg, throwable);
}

/// Shortcut for Log::error
pub fn error(msg: &str, params: &[&dyn fmt::Debug]) {
    LOG.error(msg, params);
}

/// Shortcut for Log::error_with_tag
pub fn error_with_tag(tag: &Tag, msg: &str, params: &[&dyn fmt::Debug]) {
    LOG.error_with_tag(tag, msg, params);
}

/// Shortcut for Log::error_with_throwable
pub fn error_with_throwable(msg: &str, throwable: &dyn std::error::Error) {
    LOG.error_with_throwable(msg, throwable);
}

/// Shortcut for Log::error_with_tag_and_throwable
pub fn error_with_tag_and_throwable(tag: &Tag, msg: &str, throwable: &dyn std::error::Error) {
    LOG.error_with_tag_and_throwable(tag, msg, throwable);
}
