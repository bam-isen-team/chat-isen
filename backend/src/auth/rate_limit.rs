//! In-process rate limits for authentication endpoints.

use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::error::ErrorResponse;

/// Duration of one fixed rate-limit window.
const WINDOW: Duration = Duration::from_secs(60);
/// Registration attempts allowed from one IP during a window.
const REGISTER_LIMIT: u32 = 5;
/// Login attempts allowed from one IP during a window.
const LOGIN_LIMIT: u32 = 10;
/// Refresh attempts allowed from one IP during a window.
const REFRESH_LIMIT: u32 = 20;

/// Authentication endpoint buckets with independent request quotas.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum AuthEndpoint {
    /// New-account registration.
    Register,
    /// Credential verification and session creation.
    Login,
    /// Refresh-token rotation.
    Refresh,
}

impl AuthEndpoint {
    /// Map eligible POST requests to their rate-limit bucket.
    fn from_request(request: &Request) -> Option<Self> {
        if request.method() != Method::POST {
            return None;
        }

        match request.uri().path() {
            "/auth/register" => Some(Self::Register),
            "/auth/login" => Some(Self::Login),
            "/auth/refresh" => Some(Self::Refresh),
            _ => None,
        }
    }

    /// Return the quota configured for this endpoint.
    fn limit(self) -> u32 {
        match self {
            Self::Register => REGISTER_LIMIT,
            Self::Login => LOGIN_LIMIT,
            Self::Refresh => REFRESH_LIMIT,
        }
    }
}

/// Request count and start time for one IP-and-endpoint bucket.
#[derive(Clone, Copy)]
struct WindowCounter {
    /// Monotonic start time of the current fixed window.
    started_at: Instant,
    /// Number of accepted requests in the current window.
    requests: u32,
}

/// Fixed-window counters shared by the auth routes in this backend process.
#[derive(Clone, Default)]
pub(crate) struct AuthRateLimiter {
    counters: Arc<Mutex<HashMap<(IpAddr, AuthEndpoint), WindowCounter>>>,
}

impl AuthRateLimiter {
    /// Record one request using the current monotonic time.
    fn check(&self, ip: IpAddr, endpoint: AuthEndpoint) -> Result<(), Duration> {
        self.check_at(ip, endpoint, Instant::now())
    }

    /// Record a request at `now`, returning remaining wait time when over quota.
    fn check_at(&self, ip: IpAddr, endpoint: AuthEndpoint, now: Instant) -> Result<(), Duration> {
        let mut counters = self
            .counters
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Expired buckets can be discarded on every check to keep old client
        // addresses from accumulating indefinitely in this process.
        counters.retain(|_, counter| now.saturating_duration_since(counter.started_at) < WINDOW);

        let counter = counters.entry((ip, endpoint)).or_insert(WindowCounter {
            started_at: now,
            requests: 0,
        });
        let elapsed = now.saturating_duration_since(counter.started_at);

        if elapsed >= WINDOW {
            counter.started_at = now;
            counter.requests = 0;
        }

        // The first `limit` requests pass; the next one receives a retry delay.
        if counter.requests >= endpoint.limit() {
            return Err(WINDOW.saturating_sub(elapsed));
        }

        counter.requests += 1;
        Ok(())
    }
}

/// Apply the configured per-IP quota to matching authentication POST requests.
pub(crate) async fn limit_auth_requests(
    State(limiter): State<AuthRateLimiter>,
    request: Request,
    next: Next,
) -> Response {
    let Some(endpoint) = AuthEndpoint::from_request(&request) else {
        return next.run(request).await;
    };

    // The production listener installs ConnectInfo. The localhost fallback
    // gives direct in-process Router calls a stable bucket when no socket exists.
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

    match limiter.check(peer_ip, endpoint) {
        Ok(()) => next.run(request).await,
        Err(retry_after) => rate_limited_response(retry_after),
    }
}

/// Build the standard 429 JSON response and its cache/retry headers.
fn rate_limited_response(retry_after: Duration) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorResponse {
            error: "too many requests".to_owned(),
        }),
    )
        .into_response();
    // Retry-After uses whole seconds, so round up to avoid telling clients to
    // retry before the bucket has actually expired.
    let retry_after_secs = retry_after
        .as_secs()
        .saturating_add(u64::from(retry_after.subsec_nanos() > 0))
        .max(1)
        .to_string();
    let retry_after_header =
        HeaderValue::from_str(&retry_after_secs).unwrap_or_else(|_| HeaderValue::from_static("60"));
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, retry_after_header);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
/// Unit tests for endpoint quotas, bucket separation, and retry headers.
mod tests {
    use super::{
        AuthEndpoint, AuthRateLimiter, LOGIN_LIMIT, REFRESH_LIMIT, REGISTER_LIMIT, WINDOW,
        rate_limited_response,
    };
    use axum::http::{StatusCode, header};
    use rstest::rstest;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::{Duration, Instant};

    /// Build a documentation-range IPv4 address for isolated test buckets.
    fn test_ip(last_octet: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, last_octet))
    }

    /// Allow exactly the configured quota, then reject the next request.
    #[rstest]
    #[case(AuthEndpoint::Register, REGISTER_LIMIT)]
    #[case(AuthEndpoint::Login, LOGIN_LIMIT)]
    #[case(AuthEndpoint::Refresh, REFRESH_LIMIT)]
    fn blocks_only_after_the_endpoint_limit(#[case] endpoint: AuthEndpoint, #[case] limit: u32) {
        let limiter = AuthRateLimiter::default();
        let now = Instant::now();

        for _ in 0..limit {
            assert_eq!(limiter.check_at(test_ip(1), endpoint, now), Ok(()));
        }
        assert_eq!(limiter.check_at(test_ip(1), endpoint, now), Err(WINDOW));
    }

    /// Keep counts independent across client IPs and endpoint types.
    #[rstest]
    fn buckets_are_separate_by_ip_and_endpoint() {
        let limiter = AuthRateLimiter::default();
        let now = Instant::now();

        for _ in 0..REGISTER_LIMIT {
            assert_eq!(
                limiter.check_at(test_ip(1), AuthEndpoint::Register, now),
                Ok(())
            );
        }

        assert_eq!(
            limiter.check_at(test_ip(1), AuthEndpoint::Register, now),
            Err(WINDOW)
        );
        assert_eq!(
            limiter.check_at(test_ip(1), AuthEndpoint::Login, now),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(test_ip(2), AuthEndpoint::Register, now),
            Ok(())
        );
    }

    /// Start a fresh counter after the fixed window expires.
    #[rstest]
    fn counter_resets_after_the_window() {
        let limiter = AuthRateLimiter::default();
        let now = Instant::now();

        for _ in 0..=REGISTER_LIMIT {
            let _ = limiter.check_at(test_ip(1), AuthEndpoint::Register, now);
        }

        assert_eq!(
            limiter.check_at(test_ip(1), AuthEndpoint::Register, now + WINDOW),
            Ok(())
        );
    }

    /// Return 429 and round Retry-After up to a whole second.
    #[rstest]
    fn throttle_response_is_429_with_rounded_retry_after() {
        let response = rate_limited_response(Duration::from_millis(1_500));

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "2");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
}
