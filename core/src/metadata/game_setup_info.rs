use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::logic::game_info::GameInfo;
use crate::logic::map::map_parameters::{MapParameters, MapShape};
use crate::metadata::game_parameters::GameParameters;
use crate::metadata::game_settings::GameSettings;

/// Represents the setup information for a new game
#[derive(Clone, Debug)]
pub struct GameSetupInfo {
    /// The game parameters for the setup
    pub game_parameters: GameParameters,
    /// The map parameters for the setup
    pub map_parameters: MapParameters,
    /// The map file handle (not serialized)
    #[serde(skip)]
    pub map_file: Option<File>,
}

impl Default for GameSetupInfo {
    fn default() -> Self {
        GameSetupInfo {
            game_parameters: GameParameters::default(),
            map_parameters: MapParameters::default(),
            map_file: None,
        }
    }
}

impl GameSetupInfo {
    /// Creates a new GameSetupInfo with the given game parameters and map parameters
    pub fn new(game_parameters: GameParameters, map_parameters: MapParameters) -> Self {
        GameSetupInfo {
            game_parameters,
            map_parameters,
            map_file: None,
        }
    }

    /// Creates a new GameSetupInfo from an existing GameInfo, cloning the setup including map seed
    pub fn from_game_info(game_info: &GameInfo) -> Self {
        GameSetupInfo {
            game_parameters: game_info.game_parameters.clone(),
            map_parameters: game_info.tile_map.map_parameters.clone(),
            map_file: None,
        }
    }

    /// Creates a new GameSetupInfo by cloning an existing one and reseeding the map
    pub fn from_setup(setup: &GameSetupInfo) -> Self {
        let mut new_setup = GameSetupInfo {
            game_parameters: setup.game_parameters.clone(),
            map_parameters: setup.map_parameters.clone(),
            map_file: None,
        };
        new_setup.map_parameters.reseed();
        new_setup
    }

    /// Gets a cloned and reseeded GameSetupInfo from saved settings if present, otherwise a default instance.
    ///
    /// # Arguments
    ///
    /// * `settings` - The game settings to get the last game setup from
    /// * `default_difficulty` - Overrides difficulty only when no saved settings found, so a virgin
    ///   Unciv installation can QuickStart with a different difficulty than New Game defaults to.
    pub fn from_settings(settings: &GameSettings, default_difficulty: Option<&str>) -> Self {
        if let Some(last_setup) = &settings.last_game_setup {
            let mut setup = Self::from_setup(last_setup);
            setup.map_parameters.reseed();
            setup
        } else {
            let mut setup = GameSetupInfo::default();
            if let Some(difficulty) = default_difficulty {
                setup.game_parameters.difficulty = difficulty.to_string();
            }
            setup.map_parameters.shape = MapShape::Rectangular;
            setup.map_parameters.world_wrap = true;
            setup
        }
    }

    /// Sets the map file from a path
    pub fn set_map_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let file = File::open(path)?;
        self.map_file = Some(file);
        Ok(())
    }

    /// Gets the map file content as a string
    pub fn get_map_file_content(&self) -> std::io::Result<Option<String>> {
        if let Some(mut file) = self.map_file.as_ref() {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_game_setup_info() {
        let setup = GameSetupInfo::default();
        assert_eq!(setup.game_parameters.difficulty, "Chieftain");
        assert_eq!(setup.map_parameters.shape, MapShape::Rectangular);
        assert!(setup.map_parameters.world_wrap);
        assert!(setup.map_file.is_none());
    }

    #[test]
    fn test_from_setup() {
        let mut original = GameSetupInfo::default();
        original.map_parameters.seed = 12345;

        let new_setup = GameSetupInfo::from_setup(&original);

        assert_eq!(new_setup.game_parameters.difficulty, original.game_parameters.difficulty);
        assert_ne!(new_setup.map_parameters.seed, original.map_parameters.seed);
    }

    #[test]
    fn test_from_settings_with_default_difficulty() {
        let settings = GameSettings::default();
        let setup = GameSetupInfo::from_settings(&settings, Some("Deity"));

        assert_eq!(setup.game_parameters.difficulty, "Deity");
        assert_eq!(setup.map_parameters.shape, MapShape::Rectangular);
        assert!(setup.map_parameters.world_wrap);
    }
}