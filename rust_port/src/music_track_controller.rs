use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use crate::unciv_game::Gdx;
use crate::log::Log;
use crate::music_controller::MusicController;

/// Wraps one Gdx Music instance and manages loading, playback, fading and cleanup
pub struct MusicTrackController {
    /// Internal state of this Music track
    state: State,

    /// The music resource
    music: Option<Music>,

    /// Volume level
    volume: f32,

    /// Fade step for volume transitions
    fade_step: f32,

    /// Current fade volume level
    fade_volume: f32,
}

/// Internal state of a Music track
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// No music loaded
    None,

    /// Music is currently loading
    Loading,

    /// Music is loaded but not playing
    Idle,

    /// Music is fading in
    FadeIn,

    /// Music is playing at full volume
    Playing,

    /// Music is fading out
    FadeOut,

    /// An error occurred
    Error,
}

impl State {
    /// Check if the state allows playback
    fn can_play(&self) -> bool {
        match self {
            State::None | State::Loading | State::Error => false,
            _ => true,
        }
    }
}

impl MusicTrackController {
    /// Create a new music track controller
    pub fn new(volume: f32, initial_fade_volume: f32) -> Self {
        Self {
            state: State::None,
            music: None,
            volume,
            fade_step: MusicController::default_fading_step(),
            fade_volume: initial_fade_volume,
        }
    }

    /// Clean up and dispose resources
    pub fn clear(&mut self) {
        self.state = State::None;
        if let Some(music) = self.music.take() {
            music.dispose();
        }
    }

    /// Loads a file into this controller's music and optionally calls on_success when done.
    /// Failures are silently logged to console, and on_error is called.
    /// Callbacks run on the background thread.
    ///
    /// # Panics
    ///
    /// Panics if called in the wrong state (fresh or cleared instance only)
    pub fn load<F, G>(&mut self, file: FileHandle, on_error: Option<F>, on_success: Option<G>)
    where
        F: FnOnce(&mut MusicTrackController) + Send + 'static,
        G: FnOnce(&mut MusicTrackController) + Send + 'static,
    {
        assert!(self.state == State::None && self.music.is_none(),
                "MusicTrackController.load should only be called once");

        self.state = State::Loading;

        // Clone what we need for the background thread
        let file_clone = file.clone();
        let on_error = on_error.map(|f| Arc::new(Mutex::new(Some(f))));
        let on_success = on_success.map(|f| Arc::new(Mutex::new(Some(f))));
        let controller = Arc::new(Mutex::new(self));

        // Load in a background thread
        thread::spawn(move || {
            let result = Self::load_internal(&file_clone);

            match result {
                Ok(music) => {
                    let mut controller = controller.lock().unwrap();

                    // Check if we were cleared while loading
                    if controller.state != State::Loading {
                        controller.clear();
                        return;
                    }

                    controller.music = Some(music);
                    controller.state = State::Idle;
                    debug!("Music loaded {}", file_clone.path());

                    // Call success callback if provided
                    if let Some(on_success) = on_success {
                        if let Some(callback) = on_success.lock().unwrap().take() {
                            callback(&mut controller);
                        }
                    }
                },
                Err(e) => {
                    let mut controller = controller.lock().unwrap();
                    Self::audio_exception_handler(&mut controller, e);
                    controller.state = State::Error;

                    // Call error callback if provided
                    if let Some(on_error) = on_error {
                        if let Some(callback) = on_error.lock().unwrap().take() {
                            callback(&mut controller);
                        }
                    }
                }
            }
        });
    }

    /// Internal method to load music from a file
    fn load_internal(file: &FileHandle) -> Result<Music, Box<dyn std::error::Error>> {
        Ok(Gdx::audio().new_music(file))
    }

    /// Called by the MusicController in its timer "tick" event handler, implements fading
    pub fn timer_tick(&mut self) -> State {
        if self.state == State::FadeIn {
            self.fade_in_step();
        }
        if self.state == State::FadeOut {
            self.fade_out_step();
        }
        self.state
    }

    /// Starts fadeIn or fadeOut.
    ///
    /// Note this does _not_ set the current fade "percentage" to allow smoothly
    /// changing direction mid-fade
    ///
    /// # Arguments
    ///
    /// * `fade` - The fade state to transition to
    /// * `step` - Overrides current fade step only if >0
    pub fn start_fade(&mut self, fade: State, step: f32) {
        if !self.state.can_play() {
            return;
        }

        if step > 0.0 {
            self.fade_step = step;
        }

        self.state = fade;
    }

    /// Graceful shutdown tick event - fade out then report Idle
    ///
    /// # Returns
    ///
    /// `true` if shutdown can proceed, `false` if still fading out
    pub fn shutdown_tick(&mut self) -> bool {
        if !self.state.can_play() {
            self.state = State::Idle;
        }

        if self.state == State::Idle {
            return true;
        }

        if self.state != State::FadeOut {
            self.state = State::FadeOut;
            self.fade_step = MusicController::default_fading_step();
        }

        self.timer_tick() == State::Idle
    }

    /// Check if the music is currently playing
    ///
    /// # Returns
    ///
    /// `true` if the music is playing, `false` otherwise
    pub fn is_playing(&self) -> bool {
        self.state.can_play() && self.music.as_ref().map_or(false, |m| m.is_playing())
    }

    /// Calls play() on the wrapped Gdx Music, catching exceptions to console.
    ///
    /// # Returns
    ///
    /// `true` if playback started successfully, `false` otherwise
    ///
    /// # Panics
    ///
    /// Panics if called on uninitialized instance
    pub fn play(&mut self) -> bool {
        if !self.state.can_play() || self.music.is_none() {
            self.clear(); // reset to correct state
            return false;
        }

        // Unexplained observed exception: Gdx.Music.play fails with
        // "Unable to allocate audio buffers. AL Error: 40964" (AL_INVALID_OPERATION)
        // Approach: This track dies, parent controller will enter state Silence thus
        // retry after a while.
        if let Some(music) = &self.music {
            if self.try_play(music) {
                return true;
            }
        }

        self.state = State::Error;
        false
    }

    /// Adjust master volume without affecting a fade-in/out
    pub fn set_volume(&mut self, new_volume: f32) {
        self.volume = new_volume;
        if let Some(music) = &self.music {
            music.set_volume(self.volume * self.fade_volume);
        }
    }

    /// Fade in step implementation
    fn fade_in_step(&mut self) {
        // fade-in: linearly ramp fadeVolume to 1.0, then continue playing
        self.fade_volume += self.fade_step;

        if let Some(music) = &self.music {
            if self.fade_volume < 1.0 && music.is_playing() {
                music.set_volume(self.volume * self.fade_volume);
                return;
            }

            music.set_volume(self.volume);
        }

        self.fade_volume = 1.0;
        self.state = State::Playing;
    }

    /// Fade out step implementation
    fn fade_out_step(&mut self) {
        // fade-out: linearly ramp fadeVolume to 0.0, then act according to Status
        //   (Playing->Silence/Pause/Shutdown)
        // This needs to guard against the music backend breaking mid-fade away during game shutdown
        self.fade_volume -= self.fade_step;

        if let Some(music) = &self.music {
            if self.fade_volume >= 0.001 && music.is_playing() {
                music.set_volume(self.volume * self.fade_volume);
                return;
            }

            self.fade_volume = 0.0;
            music.set_volume(0.0);
            music.pause();
        }

        self.state = State::Idle;
    }

    /// Try to play the music
    fn try_play(&self, music: &Music) -> bool {
        match std::panic::catch_unwind(|| {
            music.set_volume(self.volume * self.fade_volume);
            // for fade-over this could be called by the end of the previous track:
            if !music.is_playing() {
                music.play();
            }
            true
        }) {
            Ok(result) => result,
            Err(e) => {
                Self::audio_exception_handler(self, e);
                false
            }
        }
    }

    /// Handle audio exceptions
    fn audio_exception_handler(controller: &mut MusicTrackController, ex: Box<dyn std::any::Any + Send + 'static>) {
        controller.clear();
        Log::error("Error playing music", ex);
    }
}

/// Debug logging function
fn debug(format: &str, args: std::fmt::Arguments) {
    println!(format, args);
}

/// Music resource
pub struct Music {
    id: Uuid,
}

impl Music {
    /// Create a new music instance
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    /// Check if the music is playing
    pub fn is_playing(&self) -> bool {
        // Implementation would depend on the platform
        false
    }

    /// Play the music
    pub fn play(&self) {
        // Implementation would depend on the platform
    }

    /// Pause the music
    pub fn pause(&self) {
        // Implementation would depend on the platform
    }

    /// Set the volume
    pub fn set_volume(&self, volume: f32) {
        // Implementation would depend on the platform
    }

    /// Dispose of the music resource
    pub fn dispose(&self) {
        // Implementation would depend on the platform
    }
}

/// File handle for accessing files
#[derive(Clone)]
pub struct FileHandle {
    path: String,
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(path: String) -> Self {
        Self { path }
    }

    /// Get the path of the file
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Extend Gdx with audio functionality
impl Gdx {
    /// Get the audio interface
    pub fn audio() -> Audio {
        Audio {}
    }
}

/// Audio interface
pub struct Audio {
    // Implementation details
}

impl Audio {
    /// Create a new music instance from a file
    pub fn new_music(&self, file: &FileHandle) -> Music {
        Music::new()
    }
}