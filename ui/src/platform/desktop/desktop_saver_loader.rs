// Source: orig_src/desktop/src/com/unciv/app/desktop/DesktopSaverLoader.kt
// Ported to Rust

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use native_dialog::{FileDialog, MessageDialog, MessageType};
use crate::game::UncivGame;
use crate::utils::log::Log;
use crate::utils::platform_saver_loader::{PlatformSaverLoader, Cancelled};

/// Saver loader implementation for desktop platform
pub struct DesktopSaverLoader;

impl DesktopSaverLoader {
    /// Creates a new desktop saver loader
    pub fn new() -> Self {
        Self
    }
}

impl PlatformSaverLoader for DesktopSaverLoader {
    /// Saves a game to a file
    fn save_game(&self, data: &str, suggested_location: Option<&str>,
                on_saved: Box<dyn FnOnce(String)>,
                on_error: Box<dyn FnOnce(Box<dyn std::error::Error + Send + Sync>)>) {
        // Create a channel to communicate between threads
        let (tx, rx): (Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
                      Receiver<Result<(), Box<dyn std::error::Error + Send + Sync>>>) = channel();

        // Spawn a thread to handle the file dialog
        thread::spawn(move || {
            let result = Self::pick_file(
                |stream, location| {
                    // Write data to the file
                    if let Err(e) = writeln!(stream, "{}", data) {
                        return Err(Box::new(e));
                    }
                    on_saved(location);
                    Ok(())
                },
                |e| on_error(e),
                FileDialog::new().set_location(suggested_location.unwrap_or("")),
                |path| File::create(path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                suggested_location
            );

            // Send the result back to the main thread
            tx.send(result).unwrap_or_default();
        });

        // Wait for the result
        if let Ok(Err(e)) = rx.recv() {
            on_error(e);
        }
    }

    /// Loads a game from a file
    fn load_game(&self,
                on_loaded: Box<dyn FnOnce(String, String)>,
                on_error: Box<dyn FnOnce(Box<dyn std::error::Error + Send + Sync>)>) {
        // Create a channel to communicate between threads
        let (tx, rx): (Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
                      Receiver<Result<(), Box<dyn std::error::Error + Send + Sync>>>) = channel();

        // Spawn a thread to handle the file dialog
        thread::spawn(move || {
            let result = Self::pick_file(
                |stream, location| {
                    // Read data from the file
                    let mut data = String::new();
                    if let Err(e) = stream.read_to_string(&mut data) {
                        return Err(Box::new(e));
                    }
                    on_loaded(data, location);
                    Ok(())
                },
                |e| on_error(e),
                FileDialog::new(),
                |path| File::open(path).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                None
            );

            // Send the result back to the main thread
            tx.send(result).unwrap_or_default();
        });

        // Wait for the result
        if let Ok(Err(e)) = rx.recv() {
            on_error(e);
        }
    }

    /// Picks a file using a file dialog
    fn pick_file<T, F, G, H>(
        on_success: F,
        on_error: G,
        file_dialog: FileDialog,
        create_value: H,
        suggested_location: Option<&str>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce(T, String) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
        G: FnOnce(Box<dyn std::error::Error + Send + Sync>),
        H: FnOnce(PathBuf) -> io::Result<T>,
    {
        // Show the file dialog
        match file_dialog.show_open_single_file() {
            Ok(Some(path)) => {
                // Create the value from the file
                match create_value(path.clone()) {
                    Ok(value) => {
                        // Call the success callback
                        on_success(value, path.to_string_lossy().to_string())
                    },
                    Err(e) => {
                        // Call the error callback
                        on_error(Box::new(e));
                        Err(Box::new(io::Error::new(io::ErrorKind::Other, "Failed to create value from file")))
                    }
                }
            },
            Ok(None) => {
                // User cancelled
                on_error(Box::new(Cancelled));
                Err(Box::new(Cancelled))
            },
            Err(e) => {
                // Call the error callback
                on_error(Box::new(e.clone()));
                Err(Box::new(e))
            }
        }
    }
}