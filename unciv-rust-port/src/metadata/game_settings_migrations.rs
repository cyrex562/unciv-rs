use std::time::Duration;
use serde_json::Value as JsonValue;
use crate::metadata::game_settings::GameSettings;

/// Current version of the game settings
const CURRENT_VERSION: i32 = 2;

/// Performs migrations on the game settings based on the provided JSON data
pub fn do_migrations(settings: &mut GameSettings, json: &JsonValue) {
    if settings.version.is_none() {
        migrate_multiplayer_settings(settings, json);
        settings.version = Some(1);
    }
}

/// Checks if migration is necessary based on the current version
pub fn is_migration_necessary(settings: &GameSettings) -> bool {
    settings.version != Some(CURRENT_VERSION)
}

/// Migrates multiplayer settings from the old format to the new format
fn migrate_multiplayer_settings(settings: &mut GameSettings, json: &JsonValue) {
    // Migrate user ID
    if let Some(user_id) = json.get("userId").and_then(|v| v.as_str()) {
        settings.multiplayer.user_id = user_id.to_string();
    }

    // Migrate server
    if let Some(server) = json.get("multiplayerServer").and_then(|v| v.as_str()) {
        settings.multiplayer.server = server.to_string();
    }

    // Migrate turn checker enabled
    if let Some(enabled) = json.get("multiplayerTurnCheckerEnabled").and_then(|v| v.as_bool()) {
        settings.multiplayer.turn_checker_enabled = enabled;
    }

    // Migrate turn checker persistent notification
    if let Some(notification) = json.get("multiplayerTurnCheckerPersistentNotificationEnabled").and_then(|v| v.as_bool()) {
        settings.multiplayer.turn_checker_persistent_notification_enabled = notification;
    }

    // Migrate turn checker delay
    if let Some(delay_minutes) = json.get("multiplayerTurnCheckerDelayInMinutes").and_then(|v| v.as_i64()) {
        settings.multiplayer.turn_checker_delay = Duration::from_secs((delay_minutes * 60) as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_migration_necessary() {
        let mut settings = GameSettings::default();
        assert!(is_migration_necessary(&settings));

        settings.version = Some(CURRENT_VERSION);
        assert!(!is_migration_necessary(&settings));
    }

    #[test]
    fn test_migrate_multiplayer_settings() {
        let mut settings = GameSettings::default();
        let json = json!({
            "userId": "test_user",
            "multiplayerServer": "test_server",
            "multiplayerTurnCheckerEnabled": true,
            "multiplayerTurnCheckerPersistentNotificationEnabled": false,
            "multiplayerTurnCheckerDelayInMinutes": 5
        });

        migrate_multiplayer_settings(&mut settings, &json);

        assert_eq!(settings.multiplayer.user_id, "test_user");
        assert_eq!(settings.multiplayer.server, "test_server");
        assert!(settings.multiplayer.turn_checker_enabled);
        assert!(!settings.multiplayer.turn_checker_persistent_notification_enabled);
        assert_eq!(settings.multiplayer.turn_checker_delay, Duration::from_secs(300));
    }

    #[test]
    fn test_do_migrations() {
        let mut settings = GameSettings::default();
        let json = json!({
            "userId": "test_user",
            "multiplayerServer": "test_server"
        });

        do_migrations(&mut settings, &json);

        assert_eq!(settings.version, Some(1));
        assert_eq!(settings.multiplayer.user_id, "test_user");
        assert_eq!(settings.multiplayer.server, "test_server");
    }
}