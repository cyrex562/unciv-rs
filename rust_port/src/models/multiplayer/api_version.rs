/// Represents different versions of the multiplayer API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    /// Version 1 of the API
    ApiV1,

    /// Version 2 of the API
    ApiV2,

    /// Unknown API version
    Unknown,
}