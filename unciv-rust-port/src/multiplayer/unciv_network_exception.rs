use std::error::Error;
use std::fmt;

/// Subclass of UncivShowableException indicating network errors (timeout, connection refused and so on)
#[derive(Debug)]
pub struct UncivNetworkException {
    message: String,
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl UncivNetworkException {
    /// Creates a new UncivNetworkException with a default message and the given cause
    pub fn new(cause: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            message: "An unexpected network error occurred.".to_string(),
            cause: Some(cause),
        }
    }

    /// Creates a new UncivNetworkException with a custom message and optional cause
    pub fn with_message(message: String, cause: Option<Box<dyn Error + Send + Sync>>) -> Self {
        Self { message, cause }
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the underlying cause of the error, if any
    pub fn cause(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

impl fmt::Display for UncivNetworkException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.message, cause)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl Error for UncivNetworkException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

impl From<reqwest::Error> for UncivNetworkException {
    fn from(error: reqwest::Error) -> Self {
        UncivNetworkException::new(Box::new(error))
    }
}

impl From<std::io::Error> for UncivNetworkException {
    fn from(error: std::io::Error) -> Self {
        UncivNetworkException::new(Box::new(error))
    }
}

impl From<serde_json::Error> for UncivNetworkException {
    fn from(error: serde_json::Error) -> Self {
        UncivNetworkException::new(Box::new(error))
    }
}

impl From<uuid::Error> for UncivNetworkException {
    fn from(error: uuid::Error) -> Self {
        UncivNetworkException::new(Box::new(error))
    }
}

impl From<String> for UncivNetworkException {
    fn from(message: String) -> Self {
        UncivNetworkException::with_message(message, None)
    }
}

impl From<&str> for UncivNetworkException {
    fn from(message: &str) -> Self {
        UncivNetworkException::with_message(message.to_string(), None)
    }
}