use std::time::SystemTime;
use std::error::Error;
use std::fmt;

/// Exception thrown when a file storage operation conflicts with an existing file
#[derive(Debug)]
pub struct FileStorageConflictException;

impl fmt::Display for FileStorageConflictException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File storage operation conflicts with existing file")
    }
}

impl Error for FileStorageConflictException {}

/// Exception thrown when a file storage rate limit is reached
#[derive(Debug)]
pub struct FileStorageRateLimitReached {
    limit_remaining_seconds: i32,
}

impl FileStorageRateLimitReached {
    pub fn new(limit_remaining_seconds: i32) -> Self {
        Self {
            limit_remaining_seconds,
        }
    }
}

impl fmt::Display for FileStorageRateLimitReached {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Server limit reached! Please wait for [{}] seconds",
            self.limit_remaining_seconds
        )
    }
}

impl Error for FileStorageRateLimitReached {}

/// Exception thrown when a file is not found on the multiplayer server
#[derive(Debug)]
pub struct MultiplayerFileNotFoundException {
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl MultiplayerFileNotFoundException {
    pub fn new() -> Self {
        Self { cause: None }
    }

    pub fn with_cause(cause: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            cause: Some(cause),
        }
    }
}

impl fmt::Display for MultiplayerFileNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File could not be found on the multiplayer server")
    }
}

impl Error for MultiplayerFileNotFoundException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

/// Exception thrown when authentication fails
#[derive(Debug)]
pub struct MultiplayerAuthException {
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl MultiplayerAuthException {
    pub fn new() -> Self {
        Self { cause: None }
    }

    pub fn with_cause(cause: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            cause: Some(cause),
        }
    }
}

impl fmt::Display for MultiplayerAuthException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Authentication failed")
    }
}

impl Error for MultiplayerAuthException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

/// Trait for file metadata
pub trait FileMetaData {
    /// Get the last modification time of the file
    fn get_last_modified(&self) -> SystemTime;
}

/// Trait for file storage operations
pub trait FileStorage {
    /// Save data to a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - Authentication failed (MultiplayerAuthException)
    fn save_file_data(&self, file_name: &str, data: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Load data from a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - The file can't be found (MultiplayerFileNotFoundException)
    fn load_file_data(&self, file_name: &str) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// Get metadata for a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - The file can't be found (MultiplayerFileNotFoundException)
    fn get_file_meta_data(&self, file_name: &str) -> Result<Box<dyn FileMetaData>, Box<dyn Error + Send + Sync>>;

    /// Delete a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - The file can't be found (MultiplayerFileNotFoundException)
    /// - Authentication failed (MultiplayerAuthException)
    fn delete_file(&self, file_name: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Authenticate a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - Authentication failed (MultiplayerAuthException)
    fn authenticate(&self, user_id: &str, password: &str) -> Result<bool, Box<dyn Error + Send + Sync>>;

    /// Set a new password
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file storage backend can't handle any additional actions for a time (FileStorageRateLimitReached)
    /// - Authentication failed (MultiplayerAuthException)
    fn set_password(&self, new_password: &str) -> Result<bool, Box<dyn Error + Send + Sync>>;
}