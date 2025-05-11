/// This struct is used to store the features of the server.
///
/// We use version numbers instead of simple boolean
/// to allow for future expansion and backwards compatibility.
///
/// Everything is optional, so if a feature is not present, it is assumed to be 0.
/// Dropbox does not support anything of this, so it will always be 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServerFeatureSet {
    /// The version of the authentication system
    pub auth_version: i32,
}

impl ServerFeatureSet {
    /// Create a new ServerFeatureSet
    ///
    /// # Parameters
    ///
    /// * `auth_version` - The version of the authentication system
    ///
    /// # Returns
    ///
    /// A new ServerFeatureSet
    pub fn new(auth_version: i32) -> Self {
        Self { auth_version }
    }
}