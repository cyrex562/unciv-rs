// Source: orig_src/desktop/src/com/unciv/app/desktop/HardenGdxAudio.kt
// Ported to Rust

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::any::Any;
use crate::ui::audio::Music;
use crate::ui::platform::desktop::awt_clipboard::AwtClipboard;
use crate::ui::platform::clipboard::Clipboard;

/// Audio implementation for desktop platform that handles exceptions gracefully
pub struct HardenGdxAudio {
    update_callback: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
    exception_handler: Arc<Mutex<Option<Box<dyn Fn(Box<dyn std::error::Error + Send + Sync>, Arc<dyn Music>) + Send + Sync>>>>,
    awt_clipboard: Arc<Mutex<Option<AwtClipboard>>>,
    music: Arc<Mutex<HashMap<String, Arc<dyn Music>>>>,
    no_device: bool,
}

impl HardenGdxAudio {
    /// Creates a new hardened Gdx audio implementation
    pub fn new() -> Self {
        Self {
            update_callback: Arc::new(Mutex::new(None)),
            exception_handler: Arc::new(Mutex::new(None)),
            awt_clipboard: Arc::new(Mutex::new(None)),
            music: Arc::new(Mutex::new(HashMap::new())),
            no_device: false, // This would be determined by the actual audio device
        }
    }

    /// Installs hooks for update callback and exception handler
    pub fn install_hooks<F, G>(&self, update_callback: Option<F>, exception_handler: Option<G>)
    where
        F: Fn() + Send + Sync + 'static,
        G: Fn(Box<dyn std::error::Error + Send + Sync>, Arc<dyn Music>) + Send + Sync + 'static,
    {
        if let Some(callback) = update_callback {
            let mut update_callback = self.update_callback.lock().unwrap();
            *update_callback = Some(Box::new(callback));
        }

        if let Some(handler) = exception_handler {
            let mut exception_handler = self.exception_handler.lock().unwrap();
            *exception_handler = Some(Box::new(handler));
        }
    }

    /// Gets the clipboard implementation
    pub fn get_clipboard(&self) -> Box<dyn Clipboard> {
        let mut awt_clipboard = self.awt_clipboard.lock().unwrap();
        if awt_clipboard.is_none() {
            *awt_clipboard = Some(AwtClipboard::new());
        }
        Box::new(awt_clipboard.clone().unwrap())
    }

    /// Updates the audio state
    pub fn update(&self) {
        if self.no_device {
            return;
        }

        let music = self.music.lock().unwrap();
        let mut i = 0;
        let music_vec: Vec<_> = music.values().cloned().collect();

        while i < music_vec.len() {
            let item = music_vec[i].clone();
            match std::panic::catch_unwind(|| {
                item.update();
            }) {
                Ok(_) => {},
                Err(e) => {
                    // Dispose the music item
                    item.dispose();

                    // Call the exception handler
                    let exception_handler = self.exception_handler.lock().unwrap();
                    if let Some(handler) = &*exception_handler {
                        let error = match e.downcast::<String>() {
                            Ok(s) => Box::new(std::io::Error::new(std::io::ErrorKind::Other, s.to_string())) as Box<dyn std::error::Error + Send + Sync>,
                            Err(e) => match e.downcast::<&str>() {
                                Ok(s) => Box::new(std::io::Error::new(std::io::ErrorKind::Other, s.to_string())) as Box<dyn std::error::Error + Send + Sync>,
                                Err(_) => Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unknown error")) as Box<dyn std::error::Error + Send + Sync>,
                            },
                        };
                        handler(error, item);
                    }
                },
            }
            i += 1;
        }

        // Call the update callback
        let update_callback = self.update_callback.lock().unwrap();
        if let Some(callback) = &*update_callback {
            callback();
        }
    }

    /// Creates a new music instance
    pub fn new_music(&self, file_path: &str) -> Result<Arc<dyn Music>, Box<dyn std::error::Error + Send + Sync>> {
        // This is a placeholder implementation
        // In a real implementation, this would create a new music instance based on the file path
        let music = Arc::new(DesktopMusic::new(file_path)?);
        let mut music_map = self.music.lock().unwrap();
        music_map.insert(file_path.to_string(), music.clone());
        Ok(music)
    }
}

/// Desktop implementation of the Music trait
pub struct DesktopMusic {
    file_path: String,
    is_playing: bool,
    volume: f32,
}

impl DesktopMusic {
    /// Creates a new desktop music instance
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            file_path: file_path.to_string(),
            is_playing: false,
            volume: 1.0,
        })
    }
}

impl Music for DesktopMusic {
    fn play(&mut self) {
        self.is_playing = true;
        // In a real implementation, this would play the music
    }

    fn pause(&mut self) {
        self.is_playing = false;
        // In a real implementation, this would pause the music
    }

    fn stop(&mut self) {
        self.is_playing = false;
        // In a real implementation, this would stop the music
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        // In a real implementation, this would set the volume
    }

    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn dispose(&mut self) {
        self.is_playing = false;
        // In a real implementation, this would dispose of the music
    }

    fn update(&self) {
        // In a real implementation, this would update the music state
    }
}