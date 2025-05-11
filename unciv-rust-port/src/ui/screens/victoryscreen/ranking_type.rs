// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/RankingType.kt

use egui::{Color32, Image};
use crate::ui::images::ImageGetter;

/// Enum representing different ranking types for victory screen statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RankingType {
    /// Score ranking
    Score,
    /// Population ranking
    Population,
    /// Growth ranking
    Growth,
    /// Production ranking
    Production,
    /// Gold ranking
    Gold,
    /// Territory ranking
    Territory,
    /// Force ranking
    Force,
    /// Happiness ranking
    Happiness,
    /// Technologies ranking
    Technologies,
    /// Culture ranking
    Culture,
}

impl RankingType {
    /// Gets the label for this ranking type
    pub fn label(&self) -> &'static str {
        match self {
            RankingType::Score => "Score",
            RankingType::Population => "Population",
            RankingType::Growth => "Growth",
            RankingType::Production => "Production",
            RankingType::Gold => "Gold",
            RankingType::Territory => "Territory",
            RankingType::Force => "Force",
            RankingType::Happiness => "Happiness",
            RankingType::Technologies => "Technologies",
            RankingType::Culture => "Culture",
        }
    }

    /// Gets the image for this ranking type
    pub fn get_image(&self) -> Option<Image> {
        match self {
            RankingType::Score => {
                let mut image = ImageGetter::get_image("OtherIcons/Score");
                image.set_color(Color32::from_rgb(178, 34, 34)); // FIREBRICK
                Some(image)
            },
            RankingType::Population => ImageGetter::get_stat_icon("Population"),
            RankingType::Growth => ImageGetter::get_stat_icon("Food"),
            RankingType::Production => None, // Already has icon when translated
            RankingType::Gold => None, // Already has icon when translated
            RankingType::Territory => {
                let image = ImageGetter::get_image("OtherIcons/Hexagon");
                Some(image)
            },
            RankingType::Force => {
                let image = ImageGetter::get_image("OtherIcons/Shield");
                Some(image)
            },
            RankingType::Happiness => None, // Already has icon when translated
            RankingType::Technologies => ImageGetter::get_stat_icon("Science"),
            RankingType::Culture => None, // Already has icon when translated
        }
    }

    /// Gets the ID for serialization
    pub fn id_for_serialization(&self) -> char {
        match self {
            RankingType::Score => 'S',
            RankingType::Population => 'N',
            RankingType::Growth => 'C',
            RankingType::Production => 'P',
            RankingType::Gold => 'G',
            RankingType::Territory => 'T',
            RankingType::Force => 'F',
            RankingType::Happiness => 'H',
            RankingType::Technologies => 'W',
            RankingType::Culture => 'A',
        }
    }

    /// Creates a RankingType from its serialization ID
    pub fn from_id_for_serialization(char: char) -> Option<Self> {
        match char {
            'S' => Some(RankingType::Score),
            'N' => Some(RankingType::Population),
            'C' => Some(RankingType::Growth),
            'P' => Some(RankingType::Production),
            'G' => Some(RankingType::Gold),
            'T' => Some(RankingType::Territory),
            'F' => Some(RankingType::Force),
            'H' => Some(RankingType::Happiness),
            'W' => Some(RankingType::Technologies),
            'A' => Some(RankingType::Culture),
            _ => None,
        }
    }
}