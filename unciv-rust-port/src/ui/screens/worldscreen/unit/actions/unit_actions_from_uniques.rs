// ... existing code ...

    pub fn get_setup_actions(&self, unit: &MapUnit) -> Vec<UnitAction> {
        let mut actions = Vec::new();

        for unique in unit.get_usable_unit_action_uniques() {
            if unique.contains("Can be set up to increase") {
                if unit.current_movement == 0.0 {
                    actions.push(UnitAction::new("Setup", "OtherIcons/Setup"));
                } else {
                    actions.push(UnitAction::new("Setup")
                        .with_title("Setup")
                        .with_side_effects(Box::new(|unit: &mut MapUnit| {
                            unit.action = UnitActionType::Setup;
                            unit.current_movement = 0.0;
                        })));
                }
            }
        }

        actions
    }

    pub fn get_paradrop_actions(&self, unit: &MapUnit) -> Vec<UnitAction> {
        let mut actions = Vec::new();

        if unit.can_paradrop() && unit.current_movement > 0.0 {
            actions.push(UnitAction::new("Paradrop")
                .with_title("Paradrop")
                .with_side_effects(Box::new(|unit: &mut MapUnit| {
                    unit.action = UnitActionType::Paradrop;
                })));
        }

        actions
    }

    pub fn get_air_sweep_actions(&self, unit: &MapUnit) -> Vec<UnitAction> {
        let mut actions = Vec::new();

        if unit.can_air_sweep() && unit.current_movement > 0.0 {
            actions.push(UnitAction::new("Air Sweep")
                .with_title("Air Sweep")
                .with_side_effects(Box::new(|unit: &mut MapUnit| {
                    unit.action = UnitActionType::AirSweep;
                })));
        }

        actions
    }

    pub fn get_construct_improvement_actions(&self, unit: &MapUnit, tile: &Tile) -> Vec<UnitAction> {
        let mut actions = Vec::new();

        for unique in unit.get_usable_unit_action_uniques() {
            if let Some(improvement_name) = unique.get_improvement_to_build() {
                if unit.can_build_improvement(improvement_name, tile) {
                    actions.push(UnitAction::new("Construct")
                        .with_title(format!("Construct {}", improvement_name))
                        .with_side_effects(Box::new(move |unit: &mut MapUnit| {
                            unit.action = UnitActionType::Construct(improvement_name.to_string());
                            unit.current_movement = 0.0;
                        })));
                }
            }
        }

        actions
    }

    fn get_leaders_we_promised_not_to_settle_near(&self, civ: &Civilization, tile: &Tile) -> Vec<String> {
        let mut leaders = Vec::new();

        for other_civ in self.game_info.civilizations.values() {
            if other_civ.id != civ.id && other_civ.knows_about(civ) {
                if let Some(promise) = other_civ.diplomacy.get(&civ.id) {
                    if promise.contains_promise_not_to_settle_near() &&
                       other_civ.has_city_close_to(tile, PROMISE_NOT_TO_SETTLE_DISTANCE) {
                        leaders.push(other_civ.leader_name.clone());
                    }
                }
            }
        }

        leaders
    }
// ... existing code ...
