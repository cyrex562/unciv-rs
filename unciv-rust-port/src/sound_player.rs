use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use lazy_static::lazy_static;
use uuid::Uuid;

use crate::unciv_game::{UncivGame, Gdx, ApplicationType};
use crate::unciv_sound::UncivSound;
use crate::concurrency::Concurrency;

/// SoundPlayer manages sound playback in the game
///
/// It handles loading and caching sound resources, with special handling for
/// Android and desktop platforms. It also manages preloading of common sounds.
pub struct SoundPlayer {
    /// Cache of loaded sounds
    sound_map: Arc<Mutex<HashMap<UncivSound, Option<Sound>>>>,

    /// Hash of the current mod list to detect changes
    mod_list_hash: i32,

    /// Current preloader instance
    preloader: Option<Arc<Preloader>>,
}

impl SoundPlayer {
    /// Initialize the sound player for the main menu
    pub fn initialize_for_main_menu() {
        Self::check_cache();
    }

    /// Check if the cache needs to be updated based on mod changes
    fn check_cache() {
        if !UncivGame::is_current_initialized() {
            return;
        }

        let game = UncivGame::current();

        // Get a hash covering all mods
        let game_info = game.game_info();
        let hash1 = game_info.as_ref().map(|gi| gi.ruleset.mods.hash_code()).unwrap_or(0);
        let new_hash = hash1 ^ game.settings().visual_mods.hash_code();

        // If hash is the same, leave the cache as is
        if SOUND_PLAYER.lock().unwrap().mod_list_hash != i32::MIN &&
           SOUND_PLAYER.lock().unwrap().mod_list_hash == new_hash {
            return;
        }

        // Mod list has changed - clear the cache
        Self::clear_cache();
        SOUND_PLAYER.lock().unwrap().mod_list_hash = new_hash;
        debug!("Sound cache cleared");
        Preloader::restart();
    }

    /// Clear the sound cache and release resources
    pub fn clear_cache() {
        Preloader::abort();
        let mut sound_player = SOUND_PLAYER.lock().unwrap();
        let mut sound_map = sound_player.sound_map.lock().unwrap();

        // Dispose all sounds
        for sound in sound_map.values().flatten() {
            sound.dispose();
        }

        sound_map.clear();
        sound_player.mod_list_hash = i32::MIN;
    }

    /// Get the list of folders to look for sounds
    fn get_folders() -> Vec<String> {
        if !UncivGame::is_current_initialized() {
            // Sounds before main menu shouldn't happen, but just in case return a default ""
            // which translates to the built-in assets/sounds folder
            return vec![String::new()];
        }

        let game = UncivGame::current();
        let separator = Path::separator().to_string();

        // Allow mod sounds - preferentially so they can override built-in sounds
        // audiovisual mods after game mods but before built-in sounds
        let mut mod_list = Vec::new();

        // Add sounds from game mods
        if let Some(game_info) = game.game_info() {
            mod_list.extend(game_info.ruleset.mods.iter().cloned());
        }

        // Add sounds from visual mods
        mod_list.extend(game.settings().visual_mods.iter().cloned());

        // Translate the basic mod list into relative folder names
        let mut folders = mod_list.iter()
            .map(|mod_name| format!("mods{}sounds{}", separator, separator))
            .collect::<Vec<_>>();

        // Add the built-in sounds folder
        folders.push(String::new());

        folders
    }

    /// Result of getting a sound, including whether it was freshly loaded
    struct GetSoundResult {
        resource: Sound,
        is_fresh: bool,
    }

    /// Get a sound from cache or load it from resources
    fn get(sound: &UncivSound) -> Option<GetSoundResult> {
        Self::check_cache();

        let sound_map = SOUND_PLAYER.lock().unwrap().sound_map.lock().unwrap();

        // Look for cached sound
        if sound_map.contains_key(sound) {
            return sound_map.get(sound).and_then(|s| {
                s.as_ref().map(|sound| GetSoundResult {
                    resource: sound.clone(),
                    is_fresh: false,
                })
            });
        }

        // Not cached - try loading it
        drop(sound_map); // Release the lock before loading
        Self::create_and_cache_result(sound, Self::get_file(sound))
    }

    /// Get the file handle for a sound
    fn get_file(sound: &UncivSound) -> Option<FileHandle> {
        let file_name = sound.file_name();
        let supported_extensions = ["mp3", "ogg", "wav"];

        for mod_folder in Self::get_folders() {
            for extension in supported_extensions.iter() {
                let path = format!("{}sounds{}{}.{}", mod_folder, Path::separator().to_string(), file_name, extension);

                // Try local file first
                if let Some(local_file) = UncivGame::current().files().get_local_file(&path) {
                    if local_file.exists() {
                        return Some(local_file);
                    }
                }

                // Try internal file
                let internal_file = Gdx::files().internal(&path);
                if internal_file.exists() {
                    return Some(internal_file);
                }
            }
        }

        None
    }

    /// Create and cache a sound result
    fn create_and_cache_result(sound: &UncivSound, file: Option<FileHandle>) -> Option<GetSoundResult> {
        let file = match file {
            Some(f) if f.exists() => f,
            _ => {
                debug!("Sound {} not found!", sound.file_name());
                // Remember that the actual file is missing
                let mut sound_map = SOUND_PLAYER.lock().unwrap().sound_map.lock().unwrap();
                sound_map.insert(sound.clone(), None);
                return None;
            }
        };

        debug!("Sound {} loaded from {}", sound.file_name(), file.path());
        let new_sound = Gdx::audio().new_sound(&file);

        // Store Sound for reuse
        let mut sound_map = SOUND_PLAYER.lock().unwrap().sound_map.lock().unwrap();
        sound_map.insert(sound.clone(), Some(new_sound.clone()));

        Some(GetSoundResult {
            resource: new_sound,
            is_fresh: true,
        })
    }

    /// Play a sound once
    pub fn play(sound: &UncivSound) {
        let volume = UncivGame::current().settings().sound_effects_volume;
        if sound == &UncivSound::Silent || volume < 0.01 {
            return;
        }

        let result = match Self::get(sound) {
            Some(r) => r,
            None => return,
        };

        if Gdx::app().app_type() == ApplicationType::Android {
            Self::play_android(&result.resource, result.is_fresh, volume);
        } else {
            Self::play_desktop(&result.resource, volume);
        }
    }

    /// Play a sound on Android with special handling
    fn play_android(resource: &Sound, is_fresh: bool, volume: f32) {
        // If it's already cached we should be able to play immediately
        if !is_fresh && resource.play(volume) != -1 {
            return;
        }

        Concurrency::run("DelayedSound", move || {
            thread::sleep(Duration::from_millis(40));

            let mut repeat_count = 0;
            while resource.play(volume) == -1 && repeat_count < 12 {
                thread::sleep(Duration::from_millis(20));
                repeat_count += 1;
            }
        });
    }

    /// Play a sound on desktop with special handling
    fn play_desktop(resource: &Sound, volume: f32) {
        if resource.play(volume) != -1 {
            return;
        }

        Concurrency::run_on_gl_thread("SoundRetry", move || {
            thread::sleep(Duration::from_millis(20));
            resource.play(volume);
        });
    }

    /// Play a sound repeatedly
    pub fn play_repeated(sound: &UncivSound, count: i32, delay_ms: i64) {
        Concurrency::run_on_gl_thread(move || {
            Self::play(sound);

            if count > 1 {
                Concurrency::run(move || {
                    for _ in 1..count {
                        thread::sleep(Duration::from_millis(delay_ms));
                        Concurrency::run_on_gl_thread(move || {
                            Self::play(sound);
                        });
                    }
                });
            }
        });
    }
}

/// Manages background loading of sound files
struct Preloader {
    job: Option<thread::JoinHandle<()>>,
}

impl Preloader {
    /// Get the list of sounds to preload
    fn get_preload_list() -> Vec<UncivSound> {
        vec![
            UncivSound::Click,
            UncivSound::Whoosh,
            UncivSound::Construction,
            UncivSound::Promote,
            UncivSound::Upgrade,
            UncivSound::Coin,
            UncivSound::Chimes,
            UncivSound::Choir,
        ]
    }

    /// Create a new preloader
    fn new() -> Arc<Self> {
        let preloader = Arc::new(Self { job: None });
        let preloader_clone = Arc::clone(&preloader);

        let handle = Concurrency::run("SoundPreloader", move || {
            Self::preload(&preloader_clone);
        });

        Arc::get_mut(&mut preloader).unwrap().job = Some(handle);
        preloader
    }

    /// Preload sounds
    fn preload(preloader: &Arc<Self>) {
        for sound in Self::get_preload_list() {
            thread::sleep(Duration::from_millis(10));

            // Skip if already in cache
            let sound_map = SOUND_PLAYER.lock().unwrap().sound_map.lock().unwrap();
            if sound_map.contains_key(&sound) {
                continue;
            }
            drop(sound_map);

            debug!("Preload {:?}", sound);
            SoundPlayer::create_and_cache_result(&sound, SoundPlayer::get_file(&sound));
        }
    }

    /// Abort the preloader
    fn abort() {
        if let Some(preloader) = &SOUND_PLAYER.lock().unwrap().preloader {
            if let Some(handle) = &preloader.job {
                // In Rust we can't cancel threads directly, but we can wait for them to finish
                let _ = handle.join();
            }
        }

        let mut sound_player = SOUND_PLAYER.lock().unwrap();
        sound_player.preloader = None;
    }

    /// Restart the preloader
    fn restart() {
        Self::abort();

        let mut sound_player = SOUND_PLAYER.lock().unwrap();
        sound_player.preloader = Some(Self::new());
    }
}

/// Global instance of the SoundPlayer
lazy_static! {
    static ref SOUND_PLAYER: Mutex<SoundPlayer> = Mutex::new(SoundPlayer {
        sound_map: Arc::new(Mutex::new(HashMap::with_capacity(20))),
        mod_list_hash: i32::MIN,
        preloader: None,
    });
}

/// Debug logging function
fn debug(format: &str, args: std::fmt::Arguments) {
    println!(format, args);
}

/// Trait for hash code calculation
trait HashCode {
    fn hash_code(&self) -> i32;
}

impl HashCode for Vec<String> {
    fn hash_code(&self) -> i32 {
        self.iter().fold(0, |acc, s| acc ^ s.hash_code())
    }
}

impl HashCode for String {
    fn hash_code(&self) -> i32 {
        self.as_bytes().iter().fold(0, |acc, &b| acc.wrapping_add(b as i32))
    }
}

/// File handle for accessing files
pub struct FileHandle {
    path: String,
}

impl FileHandle {
    pub fn exists(&self) -> bool {
        // Implementation would depend on the platform
        true
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Sound resource
#[derive(Clone)]
pub struct Sound {
    id: Uuid,
}

impl Sound {
    pub fn play(&self, volume: f32) -> i64 {
        // Implementation would depend on the platform
        0
    }

    pub fn dispose(&self) {
        // Implementation would depend on the platform
    }
}

/// Audio interface
pub struct Audio {
    // Implementation details
}

impl Audio {
    pub fn new_sound(&self, file: &FileHandle) -> Sound {
        Sound { id: Uuid::new_v4() }
    }
}

/// Extend Gdx with audio functionality
impl Gdx {
    pub fn audio() -> Audio {
        Audio {}
    }

    pub fn files() -> Files {
        Files {}
    }
}

/// Files interface
pub struct Files {
    // Implementation details
}

impl Files {
    pub fn internal(&self, path: &str) -> FileHandle {
        FileHandle { path: path.to_string() }
    }
}