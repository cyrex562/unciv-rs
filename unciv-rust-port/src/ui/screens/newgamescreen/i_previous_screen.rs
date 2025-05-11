use crate::models::metadata::GameSetupInfo;
use crate::models::ruleset::Ruleset;

/// Trait to implement for all screens using GameOptionsTable and PlayerPickerTable
/// for universal usage of those two tables.
pub trait IPreviousScreen {
    /// Get the game setup info
    fn game_setup_info(&self) -> &GameSetupInfo;

    /// Get the stage width
    fn stage_width(&self) -> f32;

    /// Get the ruleset
    fn ruleset(&self) -> &Ruleset;

    /// Note: Having `fn set_right_side_button_enabled(enabled: bool)` part of this trait gives a warning:
    /// "Names of the parameter #1 conflict in the following members of supertypes: 'public abstract fun setRightSideButtonEnabled(boolean: Boolean): Unit defined in com.unciv.ui.screens.IPreviousScreen, public final fun setRightSideButtonEnabled(bool: Boolean): Unit defined in com.unciv.ui.screens.PickerScreen'. This may cause problems when calling this function with named arguments."
}