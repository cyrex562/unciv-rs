/// Trait for objects that have a name that can be modified
///
/// This is a mutable name because unit tests set it (see `createRulesetObject` in TestGame.kt)
/// As of 2023-08-08 no core code modifies a name!
/// The main source of names are RuleSet json files, and Json deserialization can set an immutable name just fine
pub trait INamed {
    /// Get the name of the object
    fn name(&self) -> &str;

    /// Set the name of the object
    fn set_name(&mut self, name: String);
}