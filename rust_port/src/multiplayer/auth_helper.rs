use std::sync::atomic::{AtomicPtr, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use log::debug;

/// Authentication helper which doesn't support multiple cookies, but just does the job correctly
///
/// It also stores the username and password as well as the timestamp of the last successful login.
/// Do not use HttpCookies since the url-encoded cookie values break the authentication flow.
pub struct AuthHelper {
    /// Value of the last received session cookie (pair of cookie value and max age)
    cookie: AtomicPtr<Option<(String, Option<i32>)>>,

    /// Credentials used during the last successful login
    last_successful_credentials: AtomicPtr<Option<(String, String)>>,

    /// Timestamp of the last successful login
    last_successful_authentication: AtomicPtr<Option<Instant>>,
}

impl AuthHelper {
    /// Creates a new AuthHelper instance
    pub fn new() -> Self {
        Self {
            cookie: AtomicPtr::new(Box::into_raw(Box::new(None))),
            last_successful_credentials: AtomicPtr::new(Box::into_raw(Box::new(None))),
            last_successful_authentication: AtomicPtr::new(Box::into_raw(Box::new(None))),
        }
    }

    /// Set the session cookie, update the last refresh timestamp and the last successful credentials
    pub fn set_cookie(&self, value: String, max_age: Option<i32>, credentials: Option<(String, String)>) {
        let cookie_ptr = Box::into_raw(Box::new(Some((value, max_age))));
        let old_ptr = self.cookie.swap(cookie_ptr, Ordering::SeqCst);
        unsafe { Box::from_raw(old_ptr) }; // Drop the old value

        let auth_ptr = Box::into_raw(Box::new(Some(Instant::now())));
        let old_auth_ptr = self.last_successful_authentication.swap(auth_ptr, Ordering::SeqCst);
        unsafe { Box::from_raw(old_auth_ptr) }; // Drop the old value

        let cred_ptr = Box::into_raw(Box::new(credentials));
        let old_cred_ptr = self.last_successful_credentials.swap(cred_ptr, Ordering::SeqCst);
        unsafe { Box::from_raw(old_cred_ptr) }; // Drop the old value
    }

    /// Drop the session cookie and credentials, so that authenticating won't be possible until re-login
    pub fn unset(&self) {
        let cookie_ptr = Box::into_raw(Box::new(None::<(String, Option<i32>)>));
        let old_ptr = self.cookie.swap(cookie_ptr, Ordering::SeqCst);
        unsafe { Box::from_raw(old_ptr) }; // Drop the old value

        let cred_ptr = Box::into_raw(Box::new(None::<(String, String)>));
        let old_cred_ptr = self.last_successful_credentials.swap(cred_ptr, Ordering::SeqCst);
        unsafe { Box::from_raw(old_cred_ptr) }; // Drop the old value
    }

    /// Add authentication to the request headers by adding the Cookie header
    pub fn add(&self, headers: &mut HeaderMap) {
        let cookie_value = unsafe { &*self.cookie.load(Ordering::SeqCst) };
        let last_auth = unsafe { &*self.last_successful_authentication.load(Ordering::SeqCst) };

        if let Some((value, max_age)) = cookie_value {
            if let Some(last_auth_time) = last_auth {
                if let Some(max_age_secs) = max_age {
                    let expiry_time = last_auth_time + Duration::from_secs(*max_age_secs as u64);
                    if expiry_time < Instant::now() {
                        debug!("Session cookie might have already expired");
                    }
                }
            }

            // Using raw cookie encoding ensures that valid base64 characters are not re-url-encoded
            let cookie_header = format!("SESSION={}", value);
            headers.insert(COOKIE, HeaderValue::from_str(&cookie_header).unwrap_or_default());
        } else {
            debug!("Session cookie is not available");
        }
    }

    /// Get the last successful credentials
    pub fn get_last_successful_credentials(&self) -> Option<(String, String)> {
        let cred_ptr = unsafe { &*self.last_successful_credentials.load(Ordering::SeqCst) };
        cred_ptr.clone()
    }
}

impl Clone for AuthHelper {
    fn clone(&self) -> Self {
        let cookie_value = unsafe { &*self.cookie.load(Ordering::SeqCst) };
        let cred_value = unsafe { &*self.last_successful_credentials.load(Ordering::SeqCst) };
        let auth_value = unsafe { &*self.last_successful_authentication.load(Ordering::SeqCst) };

        let new_helper = Self::new();

        if let Some((value, max_age)) = cookie_value {
            new_helper.set_cookie(value.clone(), *max_age, cred_value.clone());
        }

        new_helper
    }
}

impl Drop for AuthHelper {
    fn drop(&mut self) {
        // Clean up the allocated memory
        unsafe {
            Box::from_raw(self.cookie.load(Ordering::SeqCst));
            Box::from_raw(self.last_successful_credentials.load(Ordering::SeqCst));
            Box::from_raw(self.last_successful_authentication.load(Ordering::SeqCst));
        }
    }
}