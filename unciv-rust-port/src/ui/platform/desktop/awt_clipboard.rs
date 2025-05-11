// Source: orig_src/desktop/src/com/unciv/app/desktop/AwtClipboard.kt
// Ported to Rust

use std::rc::Rc;
use std::cell::RefCell;
use clipboard::{ClipboardContext, ClipboardProvider};
use crate::ui::platform::clipboard::Clipboard;

/// A clipboard implementation for desktop platforms that uses the system clipboard
///
/// This is a replacement for the default clipboard implementation that avoids
/// stack size limitations that can occur with the default implementation.
pub struct AwtClipboard {
    clipboard: Rc<RefCell<ClipboardContext>>,
}

impl AwtClipboard {
    pub fn new() -> Self {
        Self {
            clipboard: Rc::new(RefCell::new(ClipboardContext::new().unwrap_or_else(|_| {
                panic!("Failed to initialize clipboard")
            }))),
        }
    }
}

impl Clipboard for AwtClipboard {
    fn has_contents(&self) -> bool {
        // In Rust, we can't easily check if the clipboard has contents without trying to get them
        // This is a simplification of the original implementation
        self.get_contents().is_some()
    }

    fn get_contents(&self) -> Option<String> {
        // Try to get the clipboard contents
        self.clipboard.borrow_mut().get_contents().ok()
    }

    fn set_contents(&mut self, content: Option<&str>) {
        // Set the clipboard contents
        if let Some(content) = content {
            let _ = self.clipboard.borrow_mut().set_contents(content.to_string());
        }
    }
}