/// Type of modifier that can be applied to uniques
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModifierType {
    /// Ensures use only as leading Unique
    None,
    /// For conditional modifiers
    Conditional,
    /// For other modifiers, disallows use as leading Unique
    Other,
}

/// Expresses which RulesetObject types a UniqueType is applicable to.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UniqueTarget {
    /// Documentation string for the target
    pub documentation_string: String,
    /// Parent target that this target inherits from
    pub inherits_from: Option<Box<UniqueTarget>>,
    /// Type of modifier for this target
    pub modifier_type: ModifierType,
}

impl UniqueTarget {
    /// Create a new unique target
    pub fn new(
        documentation_string: String,
        inherits_from: Option<Box<UniqueTarget>>,
        modifier_type: ModifierType,
    ) -> Self {
        Self {
            documentation_string,
            inherits_from,
            modifier_type,
        }
    }

    /// Create a new unique target with no documentation or inheritance
    pub fn simple() -> Self {
        Self {
            documentation_string: String::new(),
            inherits_from: None,
            modifier_type: ModifierType::None,
        }
    }

    /// Checks whether a specific UniqueTarget `self` as e.g. given by IHasUniques.get_unique_target works with `unique_target` as e.g. declared in UniqueType
    pub fn can_accept_unique_target(&self, unique_target: &UniqueTarget) -> bool {
        if self == unique_target {
            return true;
        }
        if let Some(parent) = &self.inherits_from {
            return parent.can_accept_unique_target(unique_target);
        }
        false
    }

    /// Get all targets that can display their Uniques
    pub fn displayable() -> Vec<&'static UniqueTarget> {
        vec![
            &BUILDING, &UNIT, &UNIT_TYPE, &IMPROVEMENT, &TECH, &FOLLOWER_BELIEF,
            &FOUNDER_BELIEF, &TERRAIN, &RESOURCE, &POLICY, &PROMOTION, &NATION,
            &RUINS, &SPEED, &EVENT_CHOICE,
        ]
    }

    /// Get all targets that can include suppression
    pub fn can_include_suppression() -> Vec<&'static UniqueTarget> {
        vec![
            &TRIGGERABLE,    // Includes Global and covers most IHasUnique's
            &TERRAIN, &SPEED, // IHasUnique targets without inheritsFrom
            &MOD_OPTIONS,     // For suppressions that target something that doesn't have Uniques
            &META_MODIFIER,   // Allows use as Conditional-like syntax
        ]
    }
}

// Define all unique targets as static constants
lazy_static! {
    /// Only includes uniques that have immediate effects, caused by UniqueTriggerActivation
    pub static ref TRIGGERABLE: UniqueTarget = UniqueTarget::new(
        "Uniques that have immediate, one-time effects. These can be added to techs to trigger when researched, \
        to policies to trigger when adopted, to eras to trigger when reached, to buildings to trigger when built. \
        Alternatively, you can add a TriggerCondition to them to make them into Global uniques that activate upon \
        a specific event. They can also be added to units to grant them the ability to trigger this effect as an action, \
        which can be modified with UnitActionModifier and UnitTriggerCondition conditionals.".to_string(),
        None,
        ModifierType::None,
    );

    /// Uniques that have immediate, one-time effects on a unit
    pub static ref UNIT_TRIGGERABLE: UniqueTarget = UniqueTarget::new(
        "Uniques that have immediate, one-time effects on a unit. They can be added to units (on unit, unit type, \
        or promotion) to grant them the ability to trigger this effect as an action, which can be modified with \
        UnitActionModifier and UnitTriggerCondition conditionals.".to_string(),
        Some(Box::new(TRIGGERABLE.clone())),
        ModifierType::None,
    );

    /// Global uniques that apply to all civilizations
    pub static ref GLOBAL: UniqueTarget = UniqueTarget::new(
        "Uniques that apply globally. Civs gain the abilities of these uniques from nation uniques, reached eras, \
        researched techs, adopted policies, built buildings, religion 'founder' uniques, owned resources, and \
        ruleset-wide global uniques.".to_string(),
        Some(Box::new(TRIGGERABLE.clone())),
        ModifierType::None,
    );

    // Civilization-specific targets
    pub static ref NATION: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref PERSONALITY: UniqueTarget = UniqueTarget::simple();

    pub static ref ERA: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref TECH: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref POLICY: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref FOUNDER_BELIEF: UniqueTarget = UniqueTarget::new(
        "Uniques for Founder and Enhancer type Beliefs, that will apply to the founder of this religion".to_string(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref FOLLOWER_BELIEF: UniqueTarget = UniqueTarget::new(
        "Uniques for Pantheon and Follower type beliefs, that will apply to each city where the religion is the majority religion".to_string(),
        Some(Box::new(TRIGGERABLE.clone())),
        ModifierType::None,
    );

    // City-specific targets
    pub static ref BUILDING: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref WONDER: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(BUILDING.clone())),
        ModifierType::None,
    );

    // Unit-specific targets
    pub static ref UNIT_ACTION: UniqueTarget = UniqueTarget::new(
        "Uniques that affect a unit's actions, and can be modified by UnitActionModifiers".to_string(),
        Some(Box::new(UNIT_TRIGGERABLE.clone())),
        ModifierType::None,
    );

    pub static ref UNIT: UniqueTarget = UniqueTarget::new(
        "Uniques that can be added to units, unit types, or promotions".to_string(),
        Some(Box::new(UNIT_ACTION.clone())),
        ModifierType::None,
    );

    pub static ref UNIT_TYPE: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(UNIT.clone())),
        ModifierType::None,
    );

    pub static ref PROMOTION: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(UNIT.clone())),
        ModifierType::None,
    );

    // Tile-specific targets
    pub static ref TERRAIN: UniqueTarget = UniqueTarget::simple();

    pub static ref IMPROVEMENT: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(TRIGGERABLE.clone())),
        ModifierType::None,
    );

    pub static ref RESOURCE: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref RUINS: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(UNIT_TRIGGERABLE.clone())),
        ModifierType::None,
    );

    // Other targets
    pub static ref SPEED: UniqueTarget = UniqueTarget::simple();
    pub static ref TUTORIAL: UniqueTarget = UniqueTarget::simple();

    pub static ref CITY_STATE: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::None,
    );

    pub static ref MOD_OPTIONS: UniqueTarget = UniqueTarget::simple();
    pub static ref EVENT: UniqueTarget = UniqueTarget::simple();

    pub static ref EVENT_CHOICE: UniqueTarget = UniqueTarget::new(
        String::new(),
        Some(Box::new(TRIGGERABLE.clone())),
        ModifierType::None,
    );

    // Modifier targets
    pub static ref CONDITIONAL: UniqueTarget = UniqueTarget::new(
        "Modifiers that can be added to other uniques to limit when they will be active".to_string(),
        None,
        ModifierType::Conditional,
    );

    pub static ref TRIGGER_CONDITION: UniqueTarget = UniqueTarget::new(
        "Special conditionals that can be added to Triggerable uniques, to make them activate upon specific actions.".to_string(),
        Some(Box::new(GLOBAL.clone())),
        ModifierType::Other,
    );

    pub static ref UNIT_TRIGGER_CONDITION: UniqueTarget = UniqueTarget::new(
        "Special conditionals that can be added to UnitTriggerable uniques, to make them activate upon specific actions.".to_string(),
        Some(Box::new(TRIGGER_CONDITION.clone())),
        ModifierType::Other,
    );

    pub static ref UNIT_ACTION_MODIFIER: UniqueTarget = UniqueTarget::new(
        "Modifiers that can be added to UnitAction uniques as conditionals".to_string(),
        None,
        ModifierType::Other,
    );

    pub static ref META_MODIFIER: UniqueTarget = UniqueTarget::new(
        "Modifiers that can be added to other uniques changing user experience, not their behavior".to_string(),
        None,
        ModifierType::Other,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_accept_unique_target() {
        // Building can accept Global uniques
        assert!(BUILDING.can_accept_unique_target(&GLOBAL));
        // Global cannot accept Building uniques
        assert!(!GLOBAL.can_accept_unique_target(&BUILDING));
        // Building can accept Building uniques
        assert!(BUILDING.can_accept_unique_target(&BUILDING));
    }

    #[test]
    fn test_displayable_targets() {
        let displayable = UniqueTarget::displayable();
        assert!(displayable.contains(&&*BUILDING));
        assert!(displayable.contains(&&*UNIT));
        assert!(displayable.contains(&&*TECH));
    }

    #[test]
    fn test_can_include_suppression_targets() {
        let suppressible = UniqueTarget::can_include_suppression();
        assert!(suppressible.contains(&&*TRIGGERABLE));
        assert!(suppressible.contains(&&*TERRAIN));
        assert!(suppressible.contains(&&*MOD_OPTIONS));
    }
}