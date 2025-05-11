/// Trait for game resources that have a name
pub trait GameResource {
    /// Get the name of the resource
    fn name(&self) -> &str;
}