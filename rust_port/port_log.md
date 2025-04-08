# Port Log

This file logs the porting of Kotlin files to Rust.

## Entries

1. `core/src/com/unciv/json/DurationSerializer.kt`, `src/serializers/duration.rs` — Ported the DurationSerializer from Kotlin to Rust, implementing serialization and deserialization of Duration using serde traits.
2. `core/src/com/unciv/json/LastSeenImprovement.kt`, `src/serializers/last_seen_improvement.rs` — Ported the LastSeenImprovement from Kotlin to Rust, implementing a HashMap wrapper with Vector2 keys and special serialization logic. Added backward compatibility for old format.
3. `core/src/com/unciv/json/UncivJson.kt`, `src/json/mod.rs` — Ported the UncivJson from Kotlin to Rust, implementing a thread-safe JSON serializer/deserializer with support for file I/O and custom serialization.
4. `core/src/com/unciv/logic/automation/city/ConstructionAutomation.kt`, `src/automation/city/construction_automation.rs` — Ported the ConstructionAutomation from Kotlin to Rust, implementing city construction automation with support for buildings, units, and other improvements.