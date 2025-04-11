use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Mutex;
use std::sync::Arc;

/// Implements the ability to work with GitHub's rate limit, recognize blocks from previous attempts, wait and retry.
/// See: https://docs.github.com/en/rest/reference/search#rate-limit
pub struct RateLimit {
    max_requests_per_interval: i32,
    interval_in_milliseconds: u64,
    max_wait_loop: i32,
    account: i32,         // used requests
    first_request: u64,   // timestamp window start (unix epoch millisecond)
}

impl RateLimit {
    /// Create a new RateLimit instance with default values
    pub fn new() -> Arc<Mutex<RateLimit>> {
        Arc::new(Mutex::new(RateLimit {
            max_requests_per_interval: 10,
            interval_in_milliseconds: 60000,
            max_wait_loop: 3,
            account: 0,
            first_request: 0,
        }))
    }

    /// Get current time in milliseconds
    fn millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }

    /// Calculate required wait in ms
    /// Returns estimated number of milliseconds to wait for the rate limit window to expire
    fn get_wait_length(&self) -> u64 {
        self.first_request + self.interval_in_milliseconds - self.millis()
    }

    /// Maintain and check a rate-limit
    /// Returns true if rate-limited, false if another request is allowed
    fn is_limit_reached(&mut self) -> bool {
        let now = self.millis();
        let elapsed = if self.first_request == 0 {
            self.interval_in_milliseconds
        } else {
            now - self.first_request
        };

        if elapsed >= self.interval_in_milliseconds {
            self.first_request = now;
            self.account = 1;
            return false;
        }

        if self.account >= self.max_requests_per_interval {
            return true;
        }

        self.account += 1;
        false
    }

    /// If rate limit in effect, sleep long enough to allow next request.
    ///
    /// Returns true if waiting did not clear is_limit_reached() (can only happen if the clock is broken),
    /// or the wait has been interrupted by Thread.interrupt()
    /// Returns false if we were below the limit or slept long enough to drop out of it.
    pub fn wait_for_limit(&mut self) -> bool {
        let mut loop_count = 0;
        while self.is_limit_reached() {
            let wait_length = self.get_wait_length();
            thread::sleep(Duration::from_millis(wait_length));

            loop_count += 1;
            if loop_count >= self.max_wait_loop {
                return true;
            }
        }
        false
    }

    /// HTTP responses should be passed to this so the actual rate limit window can be evaluated and used.
    /// The very first response and all 403 ones are good candidates if they can be expected to contain GitHub's rate limit headers.
    ///
    /// See: https://docs.github.com/en/rest/overview/resources-in-the-rest-api#rate-limiting
    pub fn notify_http_response(&mut self, response_code: i32, response_message: &str, headers: &[(String, String)]) {
        if response_message != "rate limit exceeded" && response_code != 200 {
            return;
        }

        let get_header_long = |name: &str, default: u64| -> u64 {
            headers.iter()
                .find(|(key, _)| key == name)
                .and_then(|(_, value)| value.parse::<u64>().ok())
                .unwrap_or(default)
        };

        let limit = get_header_long("X-RateLimit-Limit", self.max_requests_per_interval as u64) as i32;
        let remaining = get_header_long("X-RateLimit-Remaining", 0) as i32;
        let reset = get_header_long("X-RateLimit-Reset", 0);

        if limit != self.max_requests_per_interval {
            println!("GitHub API Limit reported via http ({}) not equal assumed value ({})",
                     limit, self.max_requests_per_interval);
        }

        self.account = self.max_requests_per_interval - remaining;

        if reset == 0 {
            return;
        }

        self.first_request = (reset + 1) * 1000 - self.interval_in_milliseconds;
    }
}

/// Extension trait to add rate limit functionality to HTTP clients
pub trait RateLimitExt {
    /// Wait for rate limit to reset if necessary
    fn wait_for_limit(&self) -> bool;

    /// Process HTTP response headers to update rate limit information
    fn notify_http_response(&self, response_code: i32, response_message: &str, headers: &[(String, String)]);
}

impl RateLimitExt for Arc<Mutex<RateLimit>> {
    fn wait_for_limit(&self) -> bool {
        if let Ok(mut rate_limit) = self.lock() {
            rate_limit.wait_for_limit()
        } else {
            false
        }
    }

    fn notify_http_response(&self, response_code: i32, response_message: &str, headers: &[(String, String)]) {
        if let Ok(mut rate_limit) = self.lock() {
            rate_limit.notify_http_response(response_code, response_message, headers);
        }
    }
}