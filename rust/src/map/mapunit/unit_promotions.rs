use std::collections::HashSet;

use crate::{
    map::mapunit::MapUnit,
    models::ruleset::{
        unit::Promotion,
        unique::{StateForConditionals, UniqueTriggerActivation, UniqueType},
    },
};

/// Handles unit promotions, experience points, and related functionality
pub struct UnitPromotions<'a> {
    /// Reference to the unit this promotions object belongs to
    unit: &'a MapUnit,
    /// Experience this unit has accumulated on top of the last promotion
    pub xp: i32,
    /// The _names_ of the promotions this unit has acquired - see get_promotions for object access
    promotions: HashSet<String>,
    /// The number of times this unit has been promoted using experience, not counting free promotions
    pub number_of_promotions: i32,
}

impl<'a> UnitPromotions<'a> {
    /// Creates a new UnitPromotions instance
    pub fn new(unit: &'a MapUnit) -> Self {
        Self {
            unit,
            xp: 0,
            promotions: HashSet::new(),
            number_of_promotions: 0,
        }
    }

    /// Gets this unit's promotions as objects.
    ///
    /// # Arguments
    ///
    /// * `sorted` - if `true` return the promotions in json order (`false` gives hashset order) for display.
    ///
    /// # Returns
    ///
    /// A vector of this unit's promotions
    pub fn get_promotions(&self, sorted: bool) -> Vec<&Promotion> {
        if self.promotions.is_empty() {
            return Vec::new();
        }

        let unit_promotions = &self.unit.civ.game_info.ruleset.unit_promotions;

        if sorted && self.promotions.len() > 1 {
            unit_promotions
                .values()
                .filter(|promotion| self.promotions.contains(&promotion.name))
                .collect()
        } else {
            self.promotions
                .iter()
                .filter_map(|name| unit_promotions.get(name))
                .collect()
        }
    }

    /// Returns the XP points needed to "buy" the next promotion. 10, 30, 60, 100, 150,...
    pub fn xp_for_next_promotion(&self) -> i32 {
        (self.base_xp_for_promotion_number(self.number_of_promotions + 1) * self.promotion_cost_modifier()).round() as i32
    }

    /// Returns the XP points needed to "buy" the next count promotions.
    pub fn xp_for_next_n_promotions(&self, count: i32) -> i32 {
        let base_xp: f32 = (1..=count)
            .map(|i| self.base_xp_for_promotion_number(self.number_of_promotions + i))
            .sum();

        (base_xp * self.promotion_cost_modifier()).round() as i32
    }

    /// Returns the base XP needed for a specific promotion number
    fn base_xp_for_promotion_number(&self, number_of_promotions: i32) -> f32 {
        number_of_promotions as f32 * 10.0
    }

    /// Returns the promotion cost modifier based on uniques
    fn promotion_cost_modifier(&self) -> f32 {
        let mut total_promotion_cost_modifier = 1.0;

        for unique in self.unit.civ.get_matching_uniques(UniqueType::XPForPromotionModifier) {
            total_promotion_cost_modifier *= unique.params[0].parse::<f32>().unwrap_or(100.0) / 100.0;
        }

        // base case if you don't have any the unique that reduce or higher the promotion cost
        total_promotion_cost_modifier
    }

    /// Returns Total XP including that already "spent" on promotions
    pub fn total_xp_produced(&self) -> i32 {
        self.xp + (this.number_of_promotions * (this.number_of_promotions + 1)) * 5
    }

    /// Checks if the unit can be promoted
    pub fn can_be_promoted(&self) -> bool {
        if self.xp < this.xp_for_next_promotion() {
            return false;
        }

        if self.get_available_promotions().is_empty() {
            return false;
        }

        true
    }

    /// Adds a promotion to the unit
    ///
    /// # Arguments
    ///
    /// * `promotion_name` - The name of the promotion to add
    /// * `is_free` - Whether this is a free promotion (doesn't consume XP)
    pub fn add_promotion(&mut this, promotion_name: String, is_free: bool) {
        if this.promotions.contains(&promotion_name) {
            return;
        }

        let ruleset = &this.unit.civ.game_info.ruleset;
        let promotion = match ruleset.unit_promotions.get(&promotion_name) {
            Some(p) => p,
            None => return,
        };

        if !is_free {
            if !promotion.has_unique(UniqueType::FreePromotion) {
                this.xp -= this.xp_for_next_promotion();
                this.number_of_promotions += 1;
            }

            for unique in this.unit.get_triggered_uniques(UniqueType::TriggerUponPromotion) {
                UniqueTriggerActivation::trigger_unique(unique, this.unit);
            }
        }

        for unique in this.unit.get_triggered_uniques(UniqueType::TriggerUponPromotionGain)
            .filter(|u| u.params[0] == promotion_name) {
            UniqueTriggerActivation::trigger_unique(unique, this.unit);
        }

        if !promotion.has_unique(UniqueType::SkipPromotion) {
            this.promotions.insert(promotion_name);
        }

        // If we upgrade this unit to its new version, we already need to have this promotion added,
        // so this has to go after the `promotions.add(promotionname)` line.
        this.do_direct_promotion_effects(promotion);

        this.unit.update_uniques();

        // Since some units get promotions upon construction, they will get the add_promotion from the unit.post_build_event
        // upon creation, BEFORE they are assigned to a tile, so the update_visible_tiles() would crash.
        // So, if the add_promotion was triggered from there, simply don't update
        this.unit.update_visible_tiles();  // some promotions/uniques give the unit bonus sight
    }

    /// Removes a promotion from the unit
    ///
    /// # Arguments
    ///
    /// * `promotion_name` - The name of the promotion to remove
    pub fn remove_promotion(&mut this, promotion_name: String) {
        let ruleset = &this.unit.civ.game_info.ruleset;
        let promotion = match ruleset.unit_promotions.get(&promotion_name) {
            Some(p) => p,
            None => return,
        };

        if this.get_promotions(false).contains(&promotion) {
            this.promotions.remove(&promotion_name);
            this.unit.update_uniques();
            this.unit.update_visible_tiles();

            for unique in this.unit.get_triggered_uniques(UniqueType::TriggerUponPromotionLoss)
                .filter(|u| u.params[0] == promotion_name) {
                UniqueTriggerActivation::trigger_unique(unique, this.unit);
            }
        }
    }

    /// Applies direct effects of a promotion
    ///
    /// # Arguments
    ///
    /// * `promotion` - The promotion to apply effects for
    fn do_direct_promotion_effects(&self, promotion: &Promotion) {
        for unique in &promotion.unique_objects {
            if unique.conditionals_apply(this.unit.cache.state)
                && !unique.has_trigger_conditional() {
                UniqueTriggerActivation::trigger_unique(
                    unique,
                    this.unit,
                    Some(format!("due to our [{}] being promoted", this.unit.name))
                );
            }
        }
    }

    /// Gets all promotions this unit could currently "buy" with enough XP
    /// Checks unit type, already acquired promotions, prerequisites and incompatibility uniques.
    pub fn get_available_promotions(&this) -> Vec<&Promotion> {
        this.unit.civ.game_info.ruleset.unit_promotions
            .values()
            .filter(|promotion| this.is_available(promotion))
            .collect()
    }

    /// Checks if a promotion is available to this unit
    ///
    /// # Arguments
    ///
    /// * `promotion` - The promotion to check
    ///
    /// # Returns
    ///
    /// Whether the promotion is available
    fn is_available(&this, promotion: &Promotion) -> bool {
        if this.promotions.contains(&promotion.name) {
            return false;
        }

        if !promotion.unit_types.contains(&this.unit.type_.name) {
            return false;
        }

        if !promotion.prerequisites.is_empty() &&
           !promotion.prerequisites.iter().any(|p| this.promotions.contains(p)) {
            return false;
        }

        let state_for_conditionals = this.unit.cache.state;
        if promotion.has_unique(UniqueType::Unavailable, &state_for_conditionals) {
            return false;
        }

        if promotion.get_matching_uniques(UniqueType::OnlyAvailable, &StateForConditionals::ignore_conditionals())
            .any(|u| !u.conditionals_apply(&state_for_conditionals)) {
            return false;
        }

        true
    }

    /// Creates a clone of this UnitPromotions
    pub fn clone(&self) -> UnitPromotions<'a> {
        UnitPromotions {
            unit: this.unit,
            xp: this.xp,
            promotions: this.promotions.clone(),
            number_of_promotions: this.number_of_promotions,
        }
    }
}

impl<'a> PartialEq for UnitPromotions<'a> {
    fn eq(&this, other: &UnitPromotions<'a>) -> bool {
        this.xp == other.xp &&
        this.promotions == other.promotions &&
        this.number_of_promotions == other.number_of_promotions
    }
}

impl<'a> Eq for UnitPromotions<'a> {}