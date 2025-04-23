/// Game constants organized by category
pub mod constants {
    /// Game terrain types and features
    pub mod terrain {
        pub const IMPASSABLE: &str = "Impassable";
        pub const OCEAN: &str = "Ocean";
        pub const COAST: &str = "Coast";
        pub const COASTAL: &str = "Coastal";
        pub const RIVER: &str = "River";
        pub const MOUNTAIN: &str = "Mountain";
        pub const HILL: &str = "Hill";
        pub const PLAINS: &str = "Plains";
        pub const LAKES: &str = "Lakes";
        pub const DESERT: &str = "Desert";
        pub const GRASSLAND: &str = "Grassland";
        pub const TUNDRA: &str = "Tundra";
        pub const SNOW: &str = "Snow";

        pub mod vegetation {
            pub const FOREST: &str = "Forest";
            pub const JUNGLE: &str = "Jungle";
            pub const ALL: [&str; 2] = [FOREST, JUNGLE];
        }

        pub mod water {
            pub const FRESH_WATER: &str = "Fresh water";
            pub const FRESH_WATER_FILTER: &str = "Fresh Water";
        }
    }

    /// Diplomatic agreements and treaties
    pub mod diplomacy {
        pub const PEACE_TREATY: &str = "Peace Treaty";
        pub const RESEARCH_AGREEMENT: &str = "Research Agreement";
        pub const DEFENSIVE_PACT: &str = "Defensive Pact";
        pub const OPEN_BORDERS: &str = "Open Borders";
        pub const CITY_STATES: &str = "City-States";
        pub const TRADABLE: &str = "Tradable";
    }

    /// UI-related constants
    pub mod ui {
        pub const OK: &str = "OK";
        pub const CLOSE: &str = "Close";
        pub const CANCEL: &str = "Cancel";
        pub const YES: &str = "Yes";
        pub const NO: &str = "No";
        pub const LOADING: &str = "Loading...";
        pub const WORKING: &str = "Working...";
        pub const DEFAULT_FONT_SIZE: i32 = 18;
        pub const HEADING_FONT_SIZE: i32 = 24;
    }

    /// File system related constants
    pub mod files {
        pub mod extensions {
            pub const SAVE: &str = "json";
            pub const MAP: &str = "map";
            pub const MOD: &str = "zip";
        }

        pub mod directories {
            pub const MODS: &str = "mods";
            pub const SAVES: &str = "saves";
            pub const MAPS: &str = "maps";
            pub const MUSIC: &str = "music";
            pub const SOUNDS: &str = "sounds";
            pub const VOICE: &str = "voice";
            pub const IMAGES: &str = "images";
        }

        pub mod paths {
            pub const ATLAS: &str = "game.atlas";
            pub const SKIN: &str = "skin/skin.json";
            pub const FONT: &str = "fonts/OpenSans-Regular.ttf";
        }
    }

    /// Game configuration defaults
    pub mod defaults {
        pub const TILESET: &str = "HexaRealm";
        pub const FALLBACK_TILESET: &str = "FantasyHex";
        pub const UNITSET: &str = "AbsoluteUnits";
        pub const SKIN: &str = "Minimal";
        pub const FALLBACK_SKIN: &str = "Minimal";
    }

    /// Game mechanics constants
    pub mod mechanics {
        pub const MINIMUM_MOVEMENT_EPSILON: f32 = 0.05;
        pub const AI_PREFER_INQUISITOR_OVER_MISSIONARY_PRESSURE_DIFFERENCE: f32 = 3000.0;
        pub const NO_ID: i32 = -1;
    }

    /// URLs and external resources
    pub mod urls {
        pub const UNCIV_REPO: &str = "https://github.com/yairm210/Unciv/";
        pub const WIKI: &str = "https://yairm210.github.io/Unciv/";
        pub const UNCIV_XYZ_SERVER: &str = "https://uncivserver.xyz";
    }

    pub const BARBARIAN_ENCAMPMENT: &'static str = "Barbarian Encampment";
}
