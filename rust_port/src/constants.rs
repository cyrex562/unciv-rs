/// Game constants used throughout the codebase
pub struct Constants;

impl Constants {
    // Units
    pub const SETTLER: &'static str = "Settler";
    pub const ERA_SPECIFIC_UNIT: &'static str = "Era Starting Unit";
    pub const ALL: [&'static str; 2] = ["All", "all"];
    pub const NO_ID: i32 = -1;

    // Languages
    pub const ENGLISH: &'static str = "English";

    // Terrain
    pub const IMPASSABLE: &'static str = "Impassable";
    pub const OCEAN: &'static str = "Ocean";

    /// The "Coast" _terrain_
    pub const COAST: &'static str = "Coast";
    /// The "Coastal" terrain _filter_
    pub const COASTAL: &'static str = "Coastal";

    /// Used as filter and the name of the pseudo-TerrainFeature defining river Stats
    pub const RIVER: &'static str = "River";

    pub const MOUNTAIN: &'static str = "Mountain";
    pub const HILL: &'static str = "Hill";
    pub const PLAINS: &'static str = "Plains";
    pub const LAKES: &'static str = "Lakes";
    pub const DESERT: &'static str = "Desert";
    pub const GRASSLAND: &'static str = "Grassland";
    pub const TUNDRA: &'static str = "Tundra";
    pub const SNOW: &'static str = "Snow";

    pub const FOREST: &'static str = "Forest";
    pub const JUNGLE: &'static str = "Jungle";
    pub const ICE: &'static str = "Ice";
    pub const VEGETATION: [&'static str; 2] = [Self::FOREST, Self::JUNGLE];

    // Note the difference in case. **Not** interchangeable!
    // TODO this is very opaque behaviour to modders
    /// The "Fresh water" terrain _unique_
    pub const FRESH_WATER: &'static str = "Fresh water";
    /// The "Fresh Water" terrain _filter_
    pub const FRESH_WATER_FILTER: &'static str = "Fresh Water";

    pub const BARBARIAN_ENCAMPMENT: &'static str = "Barbarian encampment";
    pub const CITY_CENTER: &'static str = "City center";

    // Treaties
    pub const PEACE_TREATY: &'static str = "Peace Treaty";
    pub const RESEARCH_AGREEMENT: &'static str = "Research Agreement";
    pub const DEFENSIVE_PACT: &'static str = "Defensive Pact";

    // Agreements
    pub const OPEN_BORDERS: &'static str = "Open Borders";

    /// Used as origin in StatMap or ResourceSupplyList, or the toggle button in DiplomacyOverviewTab
    pub const CITY_STATES: &'static str = "City-States";
    /// Used as origin in ResourceSupplyList
    pub const TRADABLE: &'static str = "Tradable";

    pub const RANDOM: &'static str = "Random";
    pub const UNKNOWN_NATION_NAME: &'static str = "???";
    pub const UNKNOWN_CITY_NAME: &'static str = "???";

    pub const FORT: &'static str = "Fort";

    pub const FUTURE_TECH: &'static str = "Future Tech";
    // Easter egg name. Is to avoid conflicts when players name their own religions.
    // This religion name should never be displayed.
    pub const NO_RELIGION_NAME: &'static str = "The religion of TheLegend27";
    pub const SPY_HIDEOUT: &'static str = "Spy Hideout";

    pub const NEUTRAL_VICTORY_TYPE: &'static str = "Neutral";

    pub const CANCEL_IMPROVEMENT_ORDER: &'static str = "Cancel improvement order";
    pub const TUTORIAL_POPUP_NAME_PREFIX: &'static str = "Tutorial: ";
    pub const THIS_UNIT: &'static str = "This Unit";
    pub const TARGET_UNIT: &'static str = "Target Unit";

    pub const OK: &'static str = "OK";
    pub const CLOSE: &'static str = "Close";
    pub const CANCEL: &'static str = "Cancel";
    pub const YES: &'static str = "Yes";
    pub const NO: &'static str = "No";
    pub const LOADING: &'static str = "Loading...";
    pub const WORKING: &'static str = "Working...";

    pub const BARBARIANS: &'static str = "Barbarians";
    pub const SPECTATOR: &'static str = "Spectator";

    pub const EMBARKED: &'static str = "Embarked";
    pub const WOUNDED: &'static str = "Wounded";

    pub const REMOVE: &'static str = "Remove ";
    pub const REPAIR: &'static str = "Repair";

    pub const UNIQUE_OR_DELIMITER: &'static str = "\" OR \"";
    /// U+241 - Unit separator character. Used to join texts and split them with a char that is virtually guaranteed to not be used in normal text.
    pub const STRING_SPLIT_CHARACTER: char = '␟';

    pub const SIMULATION_CIV1: &'static str = "SimulationCiv1";
    pub const SIMULATION_CIV2: &'static str = "SimulationCiv2";

    pub const DROPBOX_MULTIPLAYER_SERVER: &'static str = "Dropbox";
    pub const UNCIV_XYZ_SERVER: &'static str = "https://uncivserver.xyz";

    pub const DEFAULT_TILESET: &'static str = "HexaRealm";
    /// Default for TileSetConfig.fallbackTileSet - Don't change unless you've also moved the crosshatch, borders, and arrows as well
    pub const DEFAULT_FALLBACK_TILESET: &'static str = "FantasyHex";
    pub const DEFAULT_UNITSET: &'static str = "AbsoluteUnits";
    pub const DEFAULT_SKIN: &'static str = "Minimal";
    pub const DEFAULT_FALLBACK_SKIN: &'static str = "Minimal";

    /// Use this to determine whether a MapUnit's movement is exhausted
    /// (currentMovement <= this) if and only if a fuzzy comparison is needed to account for Float rounding errors.
    /// _Most_ checks do compare to 0!
    pub const MINIMUM_MOVEMENT_EPSILON: f32 = 0.05;  // 0.1f was used previously, too - here for global searches
    pub const AI_PREFER_INQUISITOR_OVER_MISSIONARY_PRESSURE_DIFFERENCE: f32 = 3000.0;

    pub const DEFAULT_FONT_SIZE: i32 = 18;
    pub const HEADING_FONT_SIZE: i32 = 24;

    /// URL to the root of the Unciv repository, including trailing slash
    /// Note: Should the project move, this covers external links, but not comments e.g. mentioning issues
    pub const UNCIV_REPO_URL: &'static str = "https://github.com/yairm210/Unciv/";
    /// URL to the wiki, including trailing slash
    pub const WIKI_URL: &'static str = "https://yairm210.github.io/Unciv/";

    // File extensions
    pub const SAVE_FILE_EXTENSION: &'static str = "json";
    pub const MAP_FILE_EXTENSION: &'static str = "map";
    pub const MOD_FILE_EXTENSION: &'static str = "zip";

    // Directories
    pub const MODS_FOLDER_NAME: &'static str = "mods";
    pub const SAVES_FOLDER_NAME: &'static str = "saves";
    pub const MAPS_FOLDER_NAME: &'static str = "maps";
    pub const MUSIC_FOLDER_NAME: &'static str = "music";
    pub const SOUNDS_FOLDER_NAME: &'static str = "sounds";
    pub const VOICE_FOLDER_NAME: &'static str = "voice";
    pub const IMAGES_FOLDER_NAME: &'static str = "images";

    // File paths
    pub const ATLAS_PATH: &'static str = "game.atlas";
    pub const SKIN_PATH: &'static str = "skin/skin.json";
    pub const FONT_PATH: &'static str = "fonts/OpenSans-Regular.ttf";
}