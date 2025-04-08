pub mod declare_war_plan_evaluator;
pub mod motivation_to_attack_automation;
pub mod declare_war_target_automation;
pub mod diplomacy_automation;
pub mod next_turn_automation;
pub mod religion_automation;
pub mod trade_automation;
pub mod use_gold_automation;

pub use declare_war_plan_evaluator::DeclareWarPlanEvaluator;
pub use motivation_to_attack_automation::MotivationToAttackAutomation;
pub use declare_war_target_automation::DeclareWarTargetAutomation;
pub use diplomacy_automation::DiplomacyAutomation;
pub use next_turn_automation::NextTurnAutomation;
pub use religion_automation::ReligionAutomation;
pub use trade_automation::TradeAutomation;
pub use use_gold_automation::UseGoldAutomation;