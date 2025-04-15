use crate::map::hex_math::HexMath;

/// Predefined map sizes with their dimensions and game modifiers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Predefined {
    /// Tiny map size
    Tiny,
    /// Small map size
    Small,
    /// Medium map size
    Medium,
    /// Large map size
    Large,
    /// Huge map size
    Huge,
}

impl Predefined {
    /// Gets the radius for this predefined map size
    pub fn radius(&self) -> i32 {
        match self {
            Predefined::Tiny => 10,
            Predefined::Small => 15,
            Predefined::Medium => 20,
            Predefined::Large => 30,
            Predefined::Huge => 40,
        }
    }

    /// Gets the width for this predefined map size
    pub fn width(&self) -> i32 {
        match self {
            Predefined::Tiny => 23,
            Predefined::Small => 33,
            Predefined::Medium => 44,
            Predefined::Large => 66,
            Predefined::Huge => 87,
        }
    }

    /// Gets the height for this predefined map size
    pub fn height(&self) -> i32 {
        match self {
            Predefined::Tiny => 15,
            Predefined::Small => 21,
            Predefined::Medium => 29,
            Predefined::Large => 43,
            Predefined::Huge => 57,
        }
    }

    /// Gets the technology cost multiplier for this predefined map size
    pub fn tech_cost_multiplier(&self) -> f32 {
        match self {
            Predefined::Tiny => 1.0,
            Predefined::Small => 1.0,
            Predefined::Medium => 1.1,
            Predefined::Large => 1.2,
            Predefined::Huge => 1.3,
        }
    }

    /// Gets the technology cost per city modifier for this predefined map size
    pub fn tech_cost_per_city_modifier(&self) -> f32 {
        match self {
            Predefined::Tiny => 0.05,
            Predefined::Small => 0.05,
            Predefined::Medium => 0.05,
            Predefined::Large => 0.0375,
            Predefined::Huge => 0.025,
        }
    }

    /// Gets the policy cost per city modifier for this predefined map size
    pub fn policy_cost_per_city_modifier(&self) -> f32 {
        match self {
            Predefined::Tiny => 0.1,
            Predefined::Small => 0.1,
            Predefined::Medium => 0.1,
            Predefined::Large => 0.075,
            Predefined::Huge => 0.05,
        }
    }

    /// Safely converts a string to a Predefined value
    pub fn safe_value_of(name: &str) -> Predefined {
        match name {
            "Tiny" => Predefined::Tiny,
            "Small" => Predefined::Small,
            "Medium" => Predefined::Medium,
            "Large" => Predefined::Large,
            "Huge" => Predefined::Huge,
            _ => Predefined::Tiny,
        }
    }

    /// Gets all predefined map size names
    pub fn names() -> Vec<String> {
        vec![
            "Tiny".to_string(),
            "Small".to_string(),
            "Medium".to_string(),
            "Large".to_string(),
            "Huge".to_string(),
        ]
    }
}

/// Encapsulates the "map size" concept, without also choosing a shape.
///
/// Predefined sizes are kept in the [Predefined] enum, instances derived from these have the same [name] and copied dimensions.
/// Custom sizes always have [custom] as [name], even if created with the exact same dimensions as a [Predefined].
#[derive(Debug, Clone)]
pub struct MapSize {
    /// The name of the map size
    pub name: String,
    /// The radius of the map (for hexagonal maps)
    pub radius: i32,
    /// The width of the map (for rectangular maps)
    pub width: i32,
    /// The height of the map (for rectangular maps)
    pub height: i32,
}

impl MapSize {
    /// Not a [Predefined] enum value, but a String used in [name] to indicate user-defined dimensions.
    /// Do not mistake for [MapGeneratedMainType::custom].
    pub const CUSTOM: &'static str = "Custom";

    /// Creates a new MapSize with the given name and dimensions
    pub fn new(name: String, radius: i32, width: i32, height: i32) -> Self {
        Self {
            name,
            radius,
            width,
            height,
        }
    }

    /// Creates a new MapSize from a predefined size
    pub fn from_predefined(size: Predefined) -> Self {
        Self::new(
            format!("{:?}", size),
            size.radius(),
            size.width(),
            size.height(),
        )
    }

    /// Creates a new MapSize from a name string
    pub fn from_name(name: &str) -> Self {
        Self::from_predefined(Predefined::safe_value_of(name))
    }

    /// Creates a new MapSize with a custom radius
    pub fn from_radius(radius: i32) -> Self {
        let mut size = Self::new(Self::CUSTOM.to_string(), radius, 0, 0);
        size.set_new_radius(radius);
        size
    }

    /// Creates a new MapSize with custom width and height
    pub fn from_dimensions(width: i32, height: i32) -> Self {
        let radius = HexMath::get_equivalent_hexagonal_radius(width, height);
        Self::new(Self::CUSTOM.to_string(), radius, width, height)
    }

    /// Creates a tiny map size
    pub fn tiny() -> Self {
        Self::from_predefined(Predefined::Tiny)
    }

    /// Creates a small map size
    pub fn small() -> Self {
        Self::from_predefined(Predefined::Small)
    }

    /// Creates a medium map size
    pub fn medium() -> Self {
        Self::from_predefined(Predefined::Medium)
    }

    /// Creates a large map size
    pub fn large() -> Self {
        Self::from_predefined(Predefined::Large)
    }

    /// Creates a huge map size
    pub fn huge() -> Self {
        Self::from_predefined(Predefined::Huge)
    }

    /// Gets all predefined map size names
    pub fn names() -> Vec<String> {
        Predefined::names()
    }

    /// Creates a clone of this MapSize
    pub fn clone(&self) -> Self {
        Self::new(self.name.clone(), self.radius, self.width, self.height)
    }

    /// Gets the predefined size or the next smaller predefined size
    pub fn get_predefined_or_next_smaller(&self) -> Predefined {
        if self.name != Self::CUSTOM {
            return Predefined::safe_value_of(&self.name);
        }

        for predef in [Predefined::Huge, Predefined::Large, Predefined::Medium, Predefined::Small, Predefined::Tiny].iter() {
            if self.radius >= predef.radius() {
                return *predef;
            }
        }

        Predefined::Tiny
    }

    /// Check custom dimensions, fix if too extreme
    ///
    /// # Arguments
    ///
    /// * `world_wrap` - whether world wrap is on
    ///
    /// # Returns
    ///
    /// None if size was acceptable, otherwise untranslated reason message
    pub fn fix_undesired_sizes(&mut self, world_wrap: bool) -> Option<String> {
        if self.name != Self::CUSTOM {
            return None;  // predefined sizes are OK
        }

        // world-wrap maps must always have an even width, so round down silently
        if world_wrap && self.width % 2 != 0 {
            self.width -= 1;
        }

        // check for any bad condition and bail if none of them
        let message = if world_wrap && self.width < 32 {
            // otherwise horizontal scrolling will show edges, empirical
            Some("World wrap requires a minimum width of 32 tiles".to_string())
        } else if self.width < 3 || self.height < 3 || self.radius < 2 {
            Some("The provided map dimensions were too small".to_string())
        } else if self.radius > 500 {
            Some("The provided map dimensions were too big".to_string())
        } else if self.height * 16 < self.width || self.width * 16 < self.height {
            // aspect ratio > 16:1
            Some("The provided map dimensions had an unacceptable aspect ratio".to_string())
        } else {
            None
        };

        if message.is_none() {
            return None;
        }

        // fix the size - not knowing whether hexagonal or rectangular is used
        let new_radius = if self.radius < 2 {
            2
        } else if self.radius > 500 {
            500
        } else if world_wrap && self.radius < 15 {
            // minimum for hexagonal but more than required for rectangular
            15
        } else {
            self.radius
        };

        self.set_new_radius(new_radius);

        // tell the caller that map dimensions have changed and why
        message
    }

    /// Sets a new radius and updates width and height accordingly
    fn set_new_radius(&mut self, radius: i32) {
        self.radius = radius;
        let size = HexMath::get_equivalent_rectangular_size(radius);
        self.width = size.x as i32;
        self.height = size.y as i32;
    }
}

impl ToString for MapSize {
    /// For debugging and MapGenerator console output
    fn to_string(&self) -> String {
        if self.name == Self::CUSTOM {
            format!("{}x{}", self.width, self.height)
        } else {
            self.name.clone()
        }
    }
}