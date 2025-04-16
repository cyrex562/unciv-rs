// Source: orig_src/core/src/com/unciv/logic/files/FileChooser.kt
// Ported to Rust

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::cmp::Ordering;
use directories::ProjectDirs;
use path_clean::clean;

use crate::constants::Constants;
use crate::unciv_game::UncivGame;
use crate::unciv_sound::UncivSound;
use crate::log::Log;

/// Type alias for the result listener callback
pub type ResultListener = Box<dyn FnOnce(bool, FileHandle) + Send>;

/// A file picker implementation for the game
///
/// This is a Rust port of the original Kotlin FileChooser class.
/// It provides functionality to browse and select files in the filesystem.
pub struct FileChooser {
    /// The title of the file chooser dialog
    title: Option<String>,

    /// The starting file or directory
    start_file: Option<FileHandle>,

    /// Callback for when a file is selected
    result_listener: Option<ResultListener>,

    /// Filter for files to display
    filter: Box<dyn Fn(&FileHandle) -> bool + Send>,

    /// Whether directory browsing is enabled
    directory_browsing_enabled: bool,

    /// Whether folder selection is allowed
    allow_folder_select: bool,

    /// Whether to show absolute paths
    show_absolute_path: bool,

    /// Whether the file name input is enabled
    file_name_enabled: bool,

    /// The current directory being browsed
    current_dir: Option<FileHandle>,

    /// The current file name input
    result: Option<String>,

    /// The maximum height of the dialog
    max_height: f32,

    /// The absolute path to the local data folder
    absolute_local_path: String,

    /// The absolute path to the external storage folder
    absolute_external_path: String,
}

impl FileChooser {
    /// Create a new file chooser
    pub fn new(
        title: Option<String>,
        start_file: Option<FileHandle>,
        result_listener: Option<ResultListener>,
    ) -> Self {
        // Get the absolute paths for local and external storage
        let absolute_local_path = Self::get_local_data_path();
        let absolute_external_path = Self::get_external_storage_path();

        Self {
            title,
            start_file,
            result_listener,
            filter: Box::new(|_| true),
            directory_browsing_enabled: true,
            allow_folder_select: false,
            show_absolute_path: false,
            file_name_enabled: false,
            current_dir: None,
            result: None,
            max_height: 600.0 * 0.6, // 60% of screen height
            absolute_local_path,
            absolute_external_path,
        }
    }

    /// Create a save dialog
    pub fn create_save_dialog(
        title: Option<String>,
        path: Option<FileHandle>,
        result_listener: Option<ResultListener>,
    ) -> Self {
        let mut chooser = Self::new(title, path, result_listener);
        chooser.file_name_enabled = true;
        chooser.set_ok_button_text("Save");
        chooser
    }

    /// Create a load dialog
    pub fn create_load_dialog(
        title: Option<String>,
        path: Option<FileHandle>,
        result_listener: Option<ResultListener>,
    ) -> Self {
        let mut chooser = Self::new(title, path, result_listener);
        chooser.file_name_enabled = false;
        chooser.set_ok_button_text("Load");
        chooser
    }

    /// Create a filter for specific file extensions
    pub fn create_extension_filter(extensions: &[String]) -> Box<dyn Fn(&FileHandle) -> bool + Send> {
        let extensions = extensions.to_vec();
        Box::new(move |file| {
            if let Some(ext) = file.extension() {
                extensions.iter().any(|e| e.to_lowercase() == ext.to_lowercase())
            } else {
                false
            }
        })
    }

    /// Set the filter for files to display
    pub fn set_filter(&mut self, filter: Box<dyn Fn(&FileHandle) -> bool + Send>) {
        self.filter = filter;
        self.reset_list();
    }

    /// Set whether directory browsing is enabled
    pub fn set_directory_browsing_enabled(&mut self, enabled: bool) {
        self.directory_browsing_enabled = enabled;
        self.reset_list();
    }

    /// Set whether folder selection is allowed
    pub fn set_allow_folder_select(&mut self, allow: bool) {
        self.allow_folder_select = allow;
        self.reset_list();
    }

    /// Set whether to show absolute paths
    pub fn set_show_absolute_path(&mut self, show: bool) {
        self.show_absolute_path = show;
        self.reset_list();
    }

    /// Set whether the file name input is enabled
    pub fn set_file_name_enabled(&mut self, enabled: bool) {
        self.file_name_enabled = enabled;
    }

    /// Set the OK button text
    pub fn set_ok_button_text(&mut self, text: &str) {
        // In a real implementation, this would update the UI button text
        // For now, we'll just store it
        self.ok_button_text = Some(text.to_string());
    }

    /// Get the local data path
    fn get_local_data_path() -> String {
        if let Some(proj_dirs) = ProjectDirs::from("com", "yairm210", "Unciv") {
            proj_dirs.data_dir().to_string_lossy().to_string()
        } else {
            // Fallback to a default path
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(".unciv").to_string_lossy().to_string()
        }
    }

    /// Get the external storage path
    fn get_external_storage_path() -> String {
        // On desktop, this would be a different location than the local path
        // For simplicity, we'll use a subdirectory of the local path
        let local_path = Self::get_local_data_path();
        Path::new(&local_path).join("external").to_string_lossy().to_string()
    }

    /// Make a file handle absolute
    fn make_absolute(&self, file: &FileHandle) -> FileHandle {
        if file.is_absolute() {
            file.clone()
        } else {
            FileHandle::from_path(&file.absolute_path())
        }
    }

    /// Make a file handle relative
    fn make_relative(&self, file: &FileHandle) -> FileHandle {
        if !file.is_absolute() {
            return file.clone();
        }

        let path = file.path();

        if path.starts_with(&self.absolute_local_path) {
            let relative_path = path.strip_prefix(&self.absolute_local_path)
                .unwrap_or(Path::new(""))
                .strip_prefix(std::path::MAIN_SEPARATOR)
                .unwrap_or(Path::new(""));

            FileHandle::from_path(relative_path)
        } else if path.starts_with(&self.absolute_external_path) {
            let relative_path = path.strip_prefix(&self.absolute_external_path)
                .unwrap_or(Path::new(""))
                .strip_prefix(std::path::MAIN_SEPARATOR)
                .unwrap_or(Path::new(""));

            FileHandle::external(relative_path)
        } else {
            file.clone()
        }
    }

    /// Initialize the directory
    fn initial_directory(&mut self, start_file: Option<&FileHandle>) {
        let directory = match start_file {
            None if Self::is_external_storage_available() => {
                FileHandle::from_path(&self.absolute_external_path)
            },
            None => {
                FileHandle::from_path(&self.absolute_local_path)
            },
            Some(file) if file.is_directory() => {
                file.clone()
            },
            Some(file) => {
                self.result = Some(file.name().to_string());
                file.parent()
            }
        };

        self.change_directory(&self.make_absolute(&directory));
    }

    /// Switch between local and external storage
    fn switch_domain(&mut self) {
        if let Some(current) = &self.current_dir {
            let current_path = current.path();

            let new_path = if !Self::is_external_storage_available() {
                self.absolute_local_path.clone()
            } else if current_path.starts_with(&self.absolute_external_path) &&
                      !current_path.starts_with(&self.absolute_local_path) {
                self.absolute_local_path.clone()
            } else {
                self.absolute_external_path.clone()
            };

            self.change_directory(&FileHandle::from_path(&new_path));
        }
    }

    /// Change the current directory
    fn change_directory(&mut self, directory: &FileHandle) {
        self.current_dir = Some(directory.clone());

        let relative_file = if self.show_absolute_path {
            directory.clone()
        } else {
            self.make_relative(directory)
        };

        // Update the UI with the current directory
        // In a real implementation, this would update the UI elements

        // List files in the directory
        let mut items = Vec::new();

        if let Ok(entries) = directory.list() {
            for entry in entries {
                if !self.directory_browsing_enabled && entry.is_directory() {
                    continue;
                }

                if entry.is_hidden() {
                    continue;
                }

                if (self.filter)(&entry) {
                    items.push(FileListItem::new(&entry));
                }
            }
        }

        // Sort items: directories first, then files alphabetically
        items.sort_by(|a, b| {
            match (a.is_folder, b.is_folder) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.label.cmp(&b.label),
            }
        });

        // Add parent directory if needed
        if self.directory_browsing_enabled && directory.parent().is_some() {
            let parent = directory.parent().unwrap();
            items.insert(0, FileListItem::new_parent(&parent));
        }

        // Update the UI with the file list
        // In a real implementation, this would update the UI elements

        self.enable_ok_button();
    }

    /// Get the result file handle
    fn get_result(&self) -> FileHandle {
        if let Some(current_dir) = &self.current_dir {
            if let Some(result) = &self.result {
                if !result.is_empty() {
                    return current_dir.child(result);
                }
            }
            return current_dir.clone();
        }

        // Fallback to a default file handle
        FileHandle::from_path(".")
    }

    /// Reset the file list
    fn reset_list(&mut self) {
        if let Some(current_dir) = &self.current_dir {
            self.change_directory(current_dir);
        }
    }

    /// Enable or disable the OK button based on the current state
    fn enable_ok_button(&mut self) {
        let enabled = if self.file_name_enabled {
            self.get_save_enable()
        } else {
            self.get_load_enable()
        };

        // In a real implementation, this would update the UI button state
        self.ok_button_enabled = enabled;
    }

    /// Check if the OK button should be enabled for loading
    fn get_load_enable(&self) -> bool {
        // In a real implementation, this would check the selected file
        // For now, we'll just return true
        true
    }

    /// Check if the OK button should be enabled for saving
    fn get_save_enable(&self) -> bool {
        if let Some(current_dir) = &self.current_dir {
            if !current_dir.exists() {
                return false;
            }

            if self.allow_folder_select {
                return true;
            }

            if let Some(result) = &self.result {
                return !result.is_empty() &&
                       !result.starts_with(' ') &&
                       !result.ends_with(' ');
            }
        }

        false
    }

    /// Report the result to the listener
    fn report_result(&self, success: bool) {
        let file = self.get_result();

        if !(success && self.file_name_enabled && file.exists()) {
            if let Some(listener) = &self.result_listener {
                listener(success, file);
            }
            return;
        }

        // Show confirmation dialog for overwriting
        // In a real implementation, this would show a UI dialog
        // For now, we'll just call the listener directly
        if let Some(listener) = &self.result_listener {
            listener(success, file);
        }
    }

    /// Check if external storage is available
    fn is_external_storage_available() -> bool {
        // In a real implementation, this would check the platform
        // For now, we'll just return true
        true
    }

    /// Show the file chooser dialog
    pub fn show(&mut self) {
        if self.current_dir.is_none() {
            self.initial_directory(self.start_file.as_ref());
        }

        // In a real implementation, this would show the UI dialog
        // For now, we'll just simulate the result
        self.report_result(true);
    }

    /// Close the file chooser dialog
    pub fn close(&mut self) {
        // In a real implementation, this would close the UI dialog
    }

    // UI-related fields that would be used in a real implementation
    ok_button_text: Option<String>,
    ok_button_enabled: bool,
}

/// A file list item representing a file or directory
pub struct FileListItem {
    /// The display label
    pub label: String,

    /// The file handle
    pub file: FileHandle,

    /// Whether this is a folder
    pub is_folder: bool,
}

impl FileListItem {
    /// Create a new file list item
    pub fn new(file: &FileHandle) -> Self {
        let is_folder = file.is_directory();
        let label = if is_folder {
            format!("  {}", file.name())
        } else {
            file.name().to_string()
        };

        Self {
            label,
            file: file.clone(),
            is_folder,
        }
    }

    /// Create a new file list item for a parent directory
    pub fn new_parent(file: &FileHandle) -> Self {
        Self {
            label: "  ..".to_string(),
            file: file.clone(),
            is_folder: true,
        }
    }
}

/// A file handle representing a file or directory
#[derive(Clone)]
pub struct FileHandle {
    /// The path to the file
    path: PathBuf,

    /// Whether this is an external file
    is_external: bool,

    /// Whether this is an internal file
    is_internal: bool,
}

impl FileHandle {
    /// Create a new file handle from a path
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            is_external: false,
            is_internal: false,
        }
    }

    /// Create a new external file handle
    pub fn external(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            is_external: true,
            is_internal: false,
        }
    }

    /// Create a new absolute file handle
    pub fn absolute(path: impl AsRef<Path>) -> Self {
        Self {
            path: clean(path.as_ref()),
            is_external: false,
            is_internal: false,
        }
    }

    /// Create a new internal file handle
    pub fn internal(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            is_external: false,
            is_internal: true,
        }
    }

    /// Get the path as a string
    pub fn path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    /// Get the absolute path
    pub fn absolute_path(&self) -> PathBuf {
        if self.is_absolute() {
            self.path.clone()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(&self.path)
        }
    }

    /// Get the file name
    pub fn name(&self) -> String {
        self.path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<String> {
        self.path.extension()
            .map(|s| s.to_string_lossy().to_string())
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.path.is_dir()
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    /// Check if this file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Check if this is an absolute path
    pub fn is_absolute(&self) -> bool {
        self.path.is_absolute()
    }

    /// Check if this is an external file
    pub fn is_external(&self) -> bool {
        self.is_external
    }

    /// Check if this is a hidden file
    pub fn is_hidden(&self) -> bool {
        if let Some(name) = self.path.file_name() {
            name.to_string_lossy().starts_with('.')
        } else {
            false
        }
    }

    /// Get the parent directory
    pub fn parent(&self) -> Option<FileHandle> {
        self.path.parent().map(|p| FileHandle::from_path(p))
    }

    /// Get a child file or directory
    pub fn child(&self, name: impl AsRef<Path>) -> FileHandle {
        FileHandle::from_path(self.path.join(name))
    }

    /// List files in this directory
    pub fn list(&self) -> Result<Vec<FileHandle>, std::io::Error> {
        if !self.is_directory() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();

        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();

            files.push(FileHandle::from_path(path));
        }

        Ok(files)
    }

    /// Get the file type
    pub fn file_type(&self) -> FileType {
        if self.is_external {
            FileType::External
        } else if self.path.starts_with(FileChooser::get_local_data_path()) {
            FileType::Local
        } else {
            FileType::Absolute
        }
    }
}

/// The type of a file
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    /// A local file
    Local,

    /// An external file
    External,

    /// An absolute file
    Absolute,
}