use crate::civilization::{
    AlertType, Civilization, DiplomacyAction, NotificationCategory, NotificationIcon, PopupAlert,
};
use crate::constants::Constants;
use crate::models::ruleset::nation::PersonalityValue;
use crate::models::ruleset::unique::{UniqueTriggerActivation, UniqueType};

/// Represents the type of war being declared
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarType {
    /// One civ declared war on the other
    DirectWar,
    /// A city state has joined a war through its alliance
    CityStateAllianceWar,
    /// A civilization has joined a war through its defensive pact
    DefensivePactWar,
    /// A civilization has joined a war through a trade
    JoinWar,
    /// Two civilizations are starting a war through a trade
    TeamWar,
    /// Someone attacked our protected city-state
    ProtectedCityStateWar,
    /// Someone attacked our allied city-state
    AlliedCityStateWar,
}

/// Stores the reason for the war
///
/// # Arguments
///
/// * `war_type` - The type of war being declared
/// * `ally_civ` - If the given WarType is CityStateAllianceWar, DefensivePactWar or JoinWar
///               the ally_civ needs to be given
pub struct DeclareWarReason {
    pub war_type: WarType,
    pub ally_civ: Option<Civilization>,
}

impl DeclareWarReason {
    pub fn new(war_type: WarType, ally_civ: Option<Civilization>) -> Self {
        Self { war_type, ally_civ }
    }
}

/// Handles war declarations and their effects
pub struct DeclareWar;

impl DeclareWar {
    /// Declares war with the other civ in this diplomacy manager.
    /// Handles all war effects and diplomatic changes with other civs and such.
    ///
    /// # Arguments
    ///
    /// * `diplomacy_manager` - The diplomacy manager for the civilization declaring war
    /// * `declare_war_reason` - Changes what sort of effects the war has depending on how it was initiated.
    ///                        If it was a direct attack put WarType::DirectWar for the following effects.
    ///                        Influence with city states should only be set to -60
    ///                        when they are attacked directly, not when their ally is attacked.
    ///                        When indirect_city_state_attack is set to true, we thus don't reset the influence with this city state.
    ///                        Should only ever be set to true for calls originating from within this function.
    pub fn declare_war(diplomacy_manager: &mut DiplomacyManager, declare_war_reason: DeclareWarReason) {
        let civ_info = &mut diplomacy_manager.civ_info;
        let other_civ = diplomacy_manager.other_civ();
        let other_civ_diplomacy = diplomacy_manager.other_civ_diplomacy();

        if other_civ.is_city_state && declare_war_reason.war_type == WarType::DirectWar {
            Self::handle_city_state_direct_attack(diplomacy_manager);
        }

        Self::notify_of_war(diplomacy_manager, &declare_war_reason);

        Self::on_war_declared(diplomacy_manager, true, declare_war_reason.war_type);
        Self::on_war_declared(other_civ_diplomacy, false, declare_war_reason.war_type);

        Self::change_opinions(diplomacy_manager, &declare_war_reason);

        Self::break_treaties(diplomacy_manager);

        if other_civ.is_major_civ() {
            for unique in civ_info.get_triggered_uniques(UniqueType::TriggerUponDeclaringWar) {
                UniqueTriggerActivation::trigger_unique(unique, civ_info);
            }
        }
    }

    /// Handles the effects of a direct attack on a city-state
    fn handle_city_state_direct_attack(diplomacy_manager: &mut DiplomacyManager) {
        let civ_info = &mut diplomacy_manager.civ_info;
        let other_civ = diplomacy_manager.other_civ();
        let other_civ_diplomacy = diplomacy_manager.other_civ_diplomacy();

        other_civ_diplomacy.set_influence(-60.0);
        civ_info.num_minor_civs_attacked += 1;
        other_civ.city_state_functions.city_state_attacked(civ_info);

        // You attacked your own ally, you're a right bastard
        if other_civ.get_ally_civ() == civ_info.civ_name {
            other_civ.city_state_functions.update_ally_civ_for_city_state();
            other_civ_diplomacy.set_influence(-120.0);
            for known_civ in civ_info.get_known_civs() {
                known_civ
                    .get_diplomacy_manager(civ_info)
                    .unwrap()
                    .add_modifier(DiplomaticModifiers::BetrayedDeclarationOfFriendship, -10.0);
            }
        }
    }

    /// Notifies civilizations about the war declaration
    fn notify_of_war(diplomacy_manager: &mut DiplomacyManager, declare_war_reason: &DeclareWarReason) {
        let civ_info = &mut diplomacy_manager.civ_info;
        let other_civ = diplomacy_manager.other_civ();

        match declare_war_reason.war_type {
            WarType::DirectWar => {
                other_civ.popup_alerts.add(PopupAlert::new(AlertType::WarDeclaration, civ_info.civ_name.clone()));

                other_civ.add_notification(
                    format!("[{}] has declared war on us!", civ_info.civ_name),
                    NotificationCategory::Diplomacy,
                    other_civ.civ_name.clone(),
                    NotificationIcon::War,
                    civ_info.civ_name.clone(),
                );

                for civ in diplomacy_manager.get_common_known_civs_with_spectators() {
                    civ.add_notification(
                        format!("[{}] has declared war on [{}]!", civ_info.civ_name, other_civ.civ_name),
                        NotificationCategory::Diplomacy,
                        other_civ.civ_name.clone(),
                        NotificationIcon::War,
                        civ_info.civ_name.clone(),
                    );
                }
            }
            WarType::DefensivePactWar | WarType::CityStateAllianceWar | WarType::JoinWar
            | WarType::ProtectedCityStateWar | WarType::AlliedCityStateWar => {
                let ally_civ = declare_war_reason.ally_civ.as_ref().unwrap();
                other_civ.popup_alerts.add(PopupAlert::new(AlertType::WarDeclaration, civ_info.civ_name.clone()));

                let (aggressor, defender) = if declare_war_reason.war_type == WarType::DefensivePactWar {
                    (other_civ, civ_info)
                } else {
                    (civ_info, other_civ)
                };

                defender.add_notification(
                    format!("[{}] has joined [{}] in the war against us!", aggressor.civ_name, ally_civ.civ_name),
                    NotificationCategory::Diplomacy,
                    defender.civ_name.clone(),
                    NotificationIcon::War,
                    ally_civ.civ_name.clone(),
                    aggressor.civ_name.clone(),
                );

                aggressor.add_notification(
                    format!("We have joined [{}] in the war against [{}]!", ally_civ.civ_name, defender.civ_name),
                    NotificationCategory::Diplomacy,
                    defender.civ_name.clone(),
                    NotificationIcon::War,
                    ally_civ.civ_name.clone(),
                    aggressor.civ_name.clone(),
                );

                for civ in diplomacy_manager.get_common_known_civs_with_spectators().filter(|c| c != ally_civ) {
                    civ.add_notification(
                        format!("[{}] has joined [{}] in the war against [{}]!", aggressor.civ_name, ally_civ.civ_name, defender.civ_name),
                        NotificationCategory::Diplomacy,
                        defender.civ_name.clone(),
                        NotificationIcon::War,
                        ally_civ.civ_name.clone(),
                        aggressor.civ_name.clone(),
                    );
                }

                ally_civ.add_notification(
                    format!("[{}] has joined us in the war against [{}]!", aggressor.civ_name, defender.civ_name),
                    NotificationCategory::Diplomacy,
                    defender.civ_name.clone(),
                    NotificationIcon::War,
                    ally_civ.civ_name.clone(),
                    aggressor.civ_name.clone(),
                );
            }
            WarType::TeamWar => {
                let ally_civ = declare_war_reason.ally_civ.as_ref().unwrap();
                // We only want to send these notifications once, it doesn't matter who sends it though
                if civ_info.game_info.civilizations.iter().position(|c| c == civ_info).unwrap() >
                   civ_info.game_info.civilizations.iter().position(|c| c == ally_civ).unwrap() {
                    return;
                }

                other_civ.popup_alerts.add(PopupAlert::new(AlertType::WarDeclaration, civ_info.civ_name.clone()));
                other_civ.popup_alerts.add(PopupAlert::new(AlertType::WarDeclaration, ally_civ.civ_name.clone()));

                civ_info.add_notification(
                    format!("You and [{}] have declared war against [{}]!", ally_civ.civ_name, other_civ.civ_name),
                    NotificationCategory::Diplomacy,
                    other_civ.civ_name.clone(),
                    NotificationIcon::War,
                    ally_civ.civ_name.clone(),
                    civ_info.civ_name.clone(),
                );

                ally_civ.add_notification(
                    format!("You and [{}] have declared war against [{}]!", civ_info.civ_name, other_civ.civ_name),
                    NotificationCategory::Diplomacy,
                    other_civ.civ_name.clone(),
                    NotificationIcon::War,
                    civ_info.civ_name.clone(),
                    ally_civ.civ_name.clone(),
                );

                other_civ.add_notification(
                    format!("[{}] and [{}] have declared war against us!", civ_info.civ_name, ally_civ.civ_name),
                    NotificationCategory::Diplomacy,
                    other_civ.civ_name.clone(),
                    NotificationIcon::War,
                    ally_civ.civ_name.clone(),
                    civ_info.civ_name.clone(),
                );

                for civ in diplomacy_manager.get_common_known_civs_with_spectators().filter(|c| c != ally_civ) {
                    civ.add_notification(
                        format!("[{}] and [{}] have declared war against [{}]!", civ_info.civ_name, ally_civ.civ_name, other_civ.civ_name),
                        NotificationCategory::Diplomacy,
                        other_civ.civ_name.clone(),
                        NotificationIcon::War,
                        ally_civ.civ_name.clone(),
                        civ_info.civ_name.clone(),
                    );
                }
            }
        }
    }

    /// Everything that happens to both sides equally when war is declared by one side on the other
    fn on_war_declared(diplomacy_manager: &mut DiplomacyManager, is_offensive_war: bool, war_type: WarType) {
        // Cancel all trades
        for trade in &diplomacy_manager.trades {
            for offer in trade.their_offers.iter().filter(|o| o.duration > 0 && o.name != Constants::defensive_pact) {
                diplomacy_manager.civ_info.add_notification(
                    format!("[{}] from [{}] has ended", offer.name, diplomacy_manager.other_civ_name),
                    DiplomacyAction::new(diplomacy_manager.other_civ_name.clone(), true),
                    NotificationCategory::Trade,
                    diplomacy_manager.other_civ_name.clone(),
                    NotificationIcon::Trade,
                );
            }
        }
        diplomacy_manager.trades.clear();
        diplomacy_manager.civ_info.trade_requests.retain(|r| r.requesting_civ != diplomacy_manager.other_civ_name);

        // Must come *before* state is "at war" so units know they're not allowed in tiles without open borders anymore
        diplomacy_manager.update_has_open_borders();

        let civ_at_war_with = diplomacy_manager.other_civ();

        // If we attacked, then we need to end all of our defensive pacts according to Civ 5
        if is_offensive_war {
            Self::remove_defensive_pacts(diplomacy_manager);
        }
        diplomacy_manager.diplomatic_status = DiplomaticStatus::War;

        // Defensive pact chains are not allowed now
        if diplomacy_manager.civ_info.is_major_civ() {
            if !is_offensive_war && war_type != WarType::DefensivePactWar && !civ_at_war_with.is_city_state {
                Self::call_in_defensive_pact_allies(diplomacy_manager);
            }
            Self::call_in_city_state_allies(diplomacy_manager);
        }

        if diplomacy_manager.civ_info.is_city_state &&
            diplomacy_manager.civ_info.city_state_functions.get_protector_civs().contains(&civ_at_war_with) {
            diplomacy_manager.civ_info.city_state_functions.remove_protector_civ(civ_at_war_with, true);
        }

        diplomacy_manager.remove_modifier(DiplomaticModifiers::YearsOfPeace);
        diplomacy_manager.set_flag(DiplomacyFlags::DeclinedPeace, diplomacy_manager.civ_info.game_info.ruleset.mod_options.constants.minimum_war_duration); // AI won't propose peace for 10 turns
        diplomacy_manager.set_flag(DiplomacyFlags::DeclaredWar, diplomacy_manager.civ_info.game_info.ruleset.mod_options.constants.minimum_war_duration); // AI won't agree to trade for 10 turns
        diplomacy_manager.remove_flag(DiplomacyFlags::BorderConflict);
    }

    /// Changes opinions of other civilizations based on the war declaration
    fn change_opinions(diplomacy_manager: &mut DiplomacyManager, declare_war_reason: &DeclareWarReason) {
        let civ_info = &mut diplomacy_manager.civ_info;
        let other_civ = diplomacy_manager.other_civ();
        let other_civ_diplomacy = diplomacy_manager.other_civ_diplomacy();
        let war_type = declare_war_reason.war_type;

        other_civ_diplomacy.set_modifier(DiplomaticModifiers::DeclaredWarOnUs, -20.0);
        other_civ_diplomacy.remove_modifier(DiplomaticModifiers::ReturnedCapturedUnits);

        // Apply warmongering
        if war_type == WarType::DirectWar || war_type == WarType::JoinWar || war_type == WarType::TeamWar {
            for third_civ in civ_info.get_known_civs() {
                if !third_civ.is_at_war_with(other_civ)
                    && third_civ.get_diplomacy_manager(other_civ).map_or(false, |d| d.is_relationship_level_gt(RelationshipLevel::Competitor))
                    && third_civ != declare_war_reason.ally_civ.as_ref() {
                    // We don't want this modify to stack if there is a defensive pact
                    third_civ.get_diplomacy_manager(civ_info)
                        .unwrap()
                        .add_modifier(DiplomaticModifiers::WarMongerer, -5.0);
                }
            }
        }

        // Apply shared enemy modifiers
        for third_civ in diplomacy_manager.get_common_known_civs() {
            if (third_civ.is_at_war_with(other_civ) || third_civ == declare_war_reason.ally_civ.as_ref()) && !third_civ.is_at_war_with(civ_info) {
                // Improve our relations
                if third_civ.is_city_state {
                    third_civ.get_diplomacy_manager(civ_info).unwrap().add_influence(10.0);
                } else {
                    third_civ.get_diplomacy_manager(civ_info).unwrap().add_modifier(
                        DiplomaticModifiers::SharedEnemy,
                        5.0 * civ_info.get_personality().modifier_focus(PersonalityValue::Loyal, 0.3)
                    );
                }
            } else if third_civ.is_at_war_with(civ_info) {
                // Improve their relations
                if third_civ.is_city_state {
                    third_civ.get_diplomacy_manager(other_civ).unwrap().add_influence(10.0);
                } else {
                    third_civ.get_diplomacy_manager(other_civ).unwrap().add_modifier(
                        DiplomaticModifiers::SharedEnemy,
                        5.0 * civ_info.get_personality().modifier_focus(PersonalityValue::Loyal, 0.3)
                    );
                }
            }
        }
    }

    /// Breaks treaties between the civilizations at war
    fn break_treaties(diplomacy_manager: &mut DiplomacyManager) {
        let other_civ = diplomacy_manager.other_civ();
        let other_civ_diplomacy = diplomacy_manager.other_civ_diplomacy();

        let mut betrayed_friendship = false;
        let mut betrayed_defensive_pact = false;
        if diplomacy_manager.has_flag(DiplomacyFlags::DeclarationOfFriendship) {
            betrayed_friendship = true;
            diplomacy_manager.remove_flag(DiplomacyFlags::DeclarationOfFriendship);
            other_civ_diplomacy.remove_modifier(DiplomaticModifiers::DeclarationOfFriendship);
        }
        other_civ_diplomacy.remove_flag(DiplomacyFlags::DeclarationOfFriendship);

        if diplomacy_manager.has_flag(DiplomacyFlags::DefensivePact) {
            betrayed_defensive_pact = true;
            diplomacy_manager.remove_flag(DiplomacyFlags::DefensivePact);
            other_civ_diplomacy.remove_modifier(DiplomaticModifiers::DefensivePact);
        }
        other_civ_diplomacy.remove_flag(DiplomacyFlags::DefensivePact);

        if betrayed_friendship || betrayed_defensive_pact {
            for known_civ in diplomacy_manager.civ_info.get_known_civs() {
                let diplo_manager = known_civ.get_diplomacy_manager(&diplomacy_manager.civ_info).unwrap();
                if betrayed_friendship {
                    let amount = if known_civ == other_civ { -40.0 } else { -20.0 };
                    diplo_manager.add_modifier(
                        DiplomaticModifiers::BetrayedDeclarationOfFriendship,
                        amount * known_civ.get_personality().modifier_focus(PersonalityValue::Loyal, 0.3)
                    );
                }
                if betrayed_defensive_pact {
                    // Note: this stacks with Declaration of Friendship
                    let amount = if known_civ == other_civ { -20.0 } else { -10.0 };
                    diplo_manager.add_modifier(
                        DiplomaticModifiers::BetrayedDefensivePact,
                        amount * known_civ.get_personality().modifier_focus(PersonalityValue::Loyal, 0.3)
                    );
                }
                diplo_manager.remove_modifier(DiplomaticModifiers::DeclaredFriendshipWithOurAllies); // obviously this guy's declarations of friendship aren't worth much.
                diplo_manager.remove_modifier(DiplomaticModifiers::SignedDefensivePactWithOurAllies);
            }
        }

        if diplomacy_manager.has_flag(DiplomacyFlags::ResearchAgreement) {
            diplomacy_manager.remove_flag(DiplomacyFlags::ResearchAgreement);
            diplomacy_manager.total_of_science_during_ra = 0;
            other_civ_diplomacy.total_of_science_during_ra = 0;
        }
        other_civ_diplomacy.remove_flag(DiplomacyFlags::ResearchAgreement);

        // The other civ should keep any gifts we gave them
        // But we should not necessarily take away their gifts
        other_civ_diplomacy.remove_modifier(DiplomaticModifiers::GaveUsGifts);
    }

    /// Removes all defensive Pacts and trades. Notifies other civs.
    /// Note: Does not remove the flags and modifiers of the otherCiv if there is a defensive pact.
    /// This is so that we can apply more negative modifiers later.
    fn remove_defensive_pacts(diplomacy_manager: &mut DiplomacyManager) {
        let civ_at_war_with = diplomacy_manager.other_civ();
        for third_party_diplo_manager in diplomacy_manager.civ_info.diplomacy.values_mut() {
            if third_party_diplo_manager.diplomatic_status != DiplomaticStatus::DefensivePact {
                continue;
            }

            // Cancel the defensive pact functionality
            third_party_diplo_manager.diplomatic_status = DiplomaticStatus::Peace;
            third_party_diplo_manager.other_civ_diplomacy().diplomatic_status = DiplomaticStatus::Peace;

            // We already removed the trades and functionality
            // But we don't want to remove the flags yet so we can process BetrayedDefensivePact later
            if third_party_diplo_manager.other_civ() != civ_at_war_with {
                // Trades with defensive pact are now invalid
                let defensive_pact_offer = third_party_diplo_manager.trades
                    .iter()
                    .find(|trade| trade.our_offers.iter().any(|offer| offer.name == Constants::defensive_pact));

                if let Some(offer) = defensive_pact_offer {
                    third_party_diplo_manager.trades.retain(|t| t != offer);
                }

                let their_defensive_pact_offer = third_party_diplo_manager.other_civ_diplomacy().trades
                    .iter()
                    .find(|trade| trade.our_offers.iter().any(|offer| offer.name == Constants::defensive_pact));

                if let Some(offer) = their_defensive_pact_offer {
                    third_party_diplo_manager.other_civ_diplomacy().trades.retain(|t| t != offer);
                }

                third_party_diplo_manager.remove_flag(DiplomacyFlags::DefensivePact);
                third_party_diplo_manager.other_civ_diplomacy().remove_flag(DiplomacyFlags::DefensivePact);
            }

            for civ in third_party_diplo_manager.get_common_known_civs_with_spectators() {
                civ.add_notification(
                    format!("[{}] cancelled their Defensive Pact with [{}]!", diplomacy_manager.civ_info.civ_name, third_party_diplo_manager.other_civ_name),
                    NotificationCategory::Diplomacy,
                    diplomacy_manager.civ_info.civ_name.clone(),
                    NotificationIcon::Diplomacy,
                    third_party_diplo_manager.other_civ_name.clone(),
                );
            }

            third_party_diplo_manager.other_civ().add_notification(
                format!("[{}] cancelled their Defensive Pact with us!", diplomacy_manager.civ_info.civ_name),
                NotificationCategory::Diplomacy,
                diplomacy_manager.civ_info.civ_name.clone(),
                NotificationIcon::Diplomacy,
                third_party_diplo_manager.other_civ_name.clone(),
            );

            third_party_diplo_manager.civ_info.add_notification(
                format!("We have cancelled our Defensive Pact with [{}]!", third_party_diplo_manager.other_civ_name),
                NotificationCategory::Diplomacy,
                NotificationIcon::Diplomacy,
                third_party_diplo_manager.other_civ_name.clone(),
            );
        }
    }

    /// Goes through each DiplomacyManager with a defensive pact that is not already in the war.
    /// The civ that we are calling them in against should no longer have a defensive pact with us.
    fn call_in_defensive_pact_allies(diplomacy_manager: &mut DiplomacyManager) {
        let civ_at_war_with = diplomacy_manager.other_civ();
        for our_defensive_pact in diplomacy_manager.civ_info.diplomacy.values_mut()
            .filter(|our_dip_manager| {
                our_dip_manager.diplomatic_status == DiplomaticStatus::DefensivePact
                    && !our_dip_manager.other_civ().is_defeated()
                    && !our_dip_manager.other_civ().is_at_war_with(civ_at_war_with)
            }) {
            let ally = our_defensive_pact.other_civ();
            if !civ_at_war_with.knows(ally) {
                civ_at_war_with.diplomacy_functions.make_civilizations_meet(ally, true);
            }
            // Have the aggressor declare war on the ally.
            civ_at_war_with.get_diplomacy_manager(ally).unwrap().declare_war(
                DeclareWarReason::new(WarType::DefensivePactWar, Some(diplomacy_manager.civ_info.clone()))
            );
        }
    }

    /// Calls in city-state allies to join the war
    fn call_in_city_state_allies(diplomacy_manager: &mut DiplomacyManager) {
        let civ_at_war_with = diplomacy_manager.other_civ();
        for third_civ in diplomacy_manager.civ_info.get_known_civs()
            .iter()
            .filter(|it| it.is_city_state && it.get_ally_civ() == diplomacy_manager.civ_info.civ_name) {
            if !third_civ.is_at_war_with(civ_at_war_with) {
                if !third_civ.knows(civ_at_war_with) {
                    // Our city state ally has not met them yet, so they have to meet first
                    third_civ.diplomacy_functions.make_civilizations_meet(civ_at_war_with, true);
                }
                third_civ.get_diplomacy_manager(civ_at_war_with).unwrap().declare_war(
                    DeclareWarReason::new(WarType::CityStateAllianceWar, Some(diplomacy_manager.civ_info.clone()))
                );
            }
        }
    }
}