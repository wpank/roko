//! Passphrase login for showcase mode (S11 §4.3).
//!
//! Visitors share one passphrase, stored as an Argon2id PHC string in
//! `ROKO_SHOWCASE_PASSPHRASE_HASH`. Verification runs on a blocking thread behind a bounded queue
//! ([`PassphraseVerifier`]), so a login flood cannot exhaust CPU or memory. Every session request
//! carries the `X-Roko-CSRF: 1` header from the exact public origin.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};

use argon2::password_hash::{PasswordHash, PasswordVerifier as _};
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::http::header::ORIGIN;
use roko_core::config::schema::RokoConfig;
use roko_core::config::showcase::ShowcaseSessionConfig;

/// The header every showcase session request carries, with the value `1` (S11 §4.3).
pub const CSRF_HEADER: &str = "x-roko-csrf";

/// Verifications that may wait for a slot beyond the running ones (S11 §4.3).
pub const LOGIN_QUEUE_LIMIT: usize = 8;

/// The session cookie outside showcase mode.
const DEFAULT_SESSION_COOKIE: &str = "roko_session";

/// Passphrase verification under a bound: `concurrency` Argon2id checks at once, and at most
/// [`LOGIN_QUEUE_LIMIT`] more waiting for a slot.
pub struct PassphraseVerifier {
    slots: tokio::sync::Semaphore,
    in_flight: AtomicUsize,
    limit: usize,
}

/// A login admitted to verification. It holds its place until it is dropped.
pub struct Admission<'a> {
    verifier: &'a PassphraseVerifier,
}

impl PassphraseVerifier {
    /// A verifier that runs `concurrency` checks at once (at least one).
    pub fn new(concurrency: usize) -> Self {
        let concurrency = concurrency.max(1);
        Self {
            slots: tokio::sync::Semaphore::new(concurrency),
            in_flight: AtomicUsize::new(0),
            limit: concurrency + LOGIN_QUEUE_LIMIT,
        }
    }

    /// A place in verification, or `None` when every slot runs and the queue is full.
    pub fn admit(&self) -> Option<Admission<'_>> {
        let before = self.in_flight.fetch_add(1, Ordering::SeqCst);
        if before >= self.limit {
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Admission { verifier: self })
    }
}

impl Admission<'_> {
    /// Whether `passphrase` matches the PHC string `hash`, checked once a slot is free.
    pub async fn verify(self, hash: &str, passphrase: &str) -> bool {
        let Ok(_slot) = self.verifier.slots.acquire().await else {
            return false;
        };
        let hash = hash.to_string();
        let passphrase = passphrase.to_string();
        tokio::task::spawn_blocking(move || verify_phc(&hash, &passphrase))
            .await
            .unwrap_or(false)
    }
}

impl Drop for Admission<'_> {
    fn drop(&mut self) {
        self.verifier.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Whether `passphrase` matches the Argon2id PHC string `hash`. A malformed hash matches nothing.
///
/// The cost parameters come from the hash itself; the comparison is constant-time.
pub fn verify_phc(hash: &str, passphrase: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    argon2::Argon2::default()
        .verify_password(passphrase.as_bytes(), &parsed)
        .is_ok()
}

/// Why a showcase session request is refused before its credential is read: it lacks
/// `X-Roko-CSRF: 1`, or its `Origin` is not exactly `public_origin` (S11 §4.3).
pub fn check_csrf_and_origin(
    headers: &HeaderMap,
    public_origin: Option<&str>,
) -> Result<(), &'static str> {
    let csrf = headers.get(CSRF_HEADER).and_then(|value| value.to_str().ok());
    if csrf != Some("1") {
        return Err("csrf_required");
    }
    let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok());
    match (origin, public_origin) {
        (Some(origin), Some(expected)) if origin == expected.trim_end_matches('/') => Ok(()),
        _ => Err("origin_mismatch"),
    }
}

/// The session cookie's name: `[showcase.session] cookie_name` in showcase mode, `roko_session`
/// otherwise.
pub fn session_cookie_name(config: &RokoConfig) -> &str {
    if config.showcase.enabled {
        &config.showcase.session.cookie_name
    } else {
        DEFAULT_SESSION_COOKIE
    }
}

/// `Set-Cookie` for a new passphrase session: `HttpOnly`, `SameSite=Strict`, the absolute TTL
/// as `Max-Age`, and `Secure` unless `cookie_secure` is off for a local test.
pub fn set_session_cookie(session: &ShowcaseSessionConfig, session_id: &str) -> String {
    let mut cookie = format!(
        "{}={session_id}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        session.cookie_name, session.ttl_secs
    );
    if session.cookie_secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// `Set-Cookie` that deletes the passphrase session's cookie.
pub fn clear_session_cookie(session: &ShowcaseSessionConfig) -> String {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        session.cookie_name
    );
    if session.cookie_secure {
        cookie.push_str("; Secure");
    }
    cookie
}

/// The client's address: `Fly-Client-IP` when `trust_fly` holds (only behind Fly's proxy, which
/// sets it), else the socket peer; `None` when neither is known.
pub fn client_ip<B>(req: &axum::http::Request<B>, trust_fly: bool) -> Option<IpAddr> {
    let fly = trust_fly
        .then(|| req.headers().get("Fly-Client-IP"))
        .flatten()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok());
    fly.or_else(|| {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip())
    })
}

/// The network an address belongs to, for the audit log: an IPv4 `/24` or an IPv6 `/48`, never
/// the address itself.
pub fn ip_prefix(ip: Option<IpAddr>) -> String {
    match ip {
        Some(IpAddr::V4(ip)) => {
            let [a, b, c, _] = ip.octets();
            format!("{a}.{b}.{c}.0/24")
        }
        Some(IpAddr::V6(ip)) => {
            let [a, b, c, ..] = ip.segments();
            format!("{a:x}:{b:x}:{c:x}::/48")
        }
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full verifier refuses a new login until a place frees up.
    #[test]
    fn verifier_admits_its_slots_and_queue_then_refuses() {
        let verifier = PassphraseVerifier::new(2);
        let held: Vec<Admission<'_>> = std::iter::from_fn(|| verifier.admit()).collect();
        assert_eq!(held.len(), 2 + LOGIN_QUEUE_LIMIT);
        assert!(verifier.admit().is_none());
        drop(held);
        assert!(verifier.admit().is_some());
    }

    #[test]
    fn addresses_are_logged_as_networks() {
        assert_eq!(ip_prefix("203.0.113.77".parse().ok()), "203.0.113.0/24");
        assert_eq!(ip_prefix("2001:db8:7:1::9".parse().ok()), "2001:db8:7::/48");
        assert_eq!(ip_prefix(None), "unknown");
    }

    #[test]
    fn csrf_and_origin_are_both_required() {
        let mut headers = HeaderMap::new();
        let origin = Some("https://showcase.test");
        assert_eq!(check_csrf_and_origin(&headers, origin), Err("csrf_required"));
        headers.insert(CSRF_HEADER, "1".parse().expect("header value"));
        assert_eq!(check_csrf_and_origin(&headers, origin), Err("origin_mismatch"));
        headers.insert(ORIGIN, "https://evil.test".parse().expect("header value"));
        assert_eq!(check_csrf_and_origin(&headers, origin), Err("origin_mismatch"));
        headers.insert(ORIGIN, "https://showcase.test".parse().expect("header value"));
        assert_eq!(check_csrf_and_origin(&headers, origin), Ok(()));
        assert_eq!(check_csrf_and_origin(&headers, None), Err("origin_mismatch"));
    }
}
