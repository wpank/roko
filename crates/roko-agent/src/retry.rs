//! Retry policy helpers for provider-backed agent requests.

use rand::Rng;

use crate::provider::ProviderError;

/// Canonical error classes used by retry policy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Provider rejected the request because of rate limiting.
    RateLimit,
    /// Provider rejected the request because authentication failed.
    AuthFailure,
    /// Request timed out before completing.
    Timeout,
    /// Provider returned a transient 5xx-style failure.
    ServerError,
    /// Provider rejected the request for content-policy reasons.
    ContentPolicy,
    /// Request exceeded the model context window.
    ContextOverflow,
    /// Model or deployment slug does not exist.
    ModelNotFound,
    /// Fallback when the exact failure type is not otherwise classified.
    Unknown,
}

impl From<&ProviderError> for ErrorClass {
    fn from(error: &ProviderError) -> Self {
        match error {
            ProviderError::RateLimit { .. } => Self::RateLimit,
            ProviderError::AuthFailure => Self::AuthFailure,
            ProviderError::Timeout => Self::Timeout,
            ProviderError::ServerError(_) => Self::ServerError,
            ProviderError::ContentPolicy => Self::ContentPolicy,
            ProviderError::ContextOverflow => Self::ContextOverflow,
            ProviderError::ModelNotFound => Self::ModelNotFound,
            ProviderError::Other(_) => Self::Unknown,
        }
    }
}

impl std::fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimit => f.write_str("rate_limit"),
            Self::AuthFailure => f.write_str("auth_failure"),
            Self::Timeout => f.write_str("timeout"),
            Self::ServerError => f.write_str("server_error"),
            Self::ContentPolicy => f.write_str("content_policy"),
            Self::ContextOverflow => f.write_str("context_overflow"),
            Self::ModelNotFound => f.write_str("model_not_found"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

/// Retry policy with exponential backoff and ±25% jitter.
///
/// The delay for attempt `n` (0-indexed) is computed as:
///
/// ```text
/// base_ms = base_delay_ms * 2^n   (capped at max_delay_ms)
/// delay   = base_ms * rand(0.75 ..= 1.25)
/// ```
///
/// The ±25% window prevents thundering-herd reconvergence without spreading
/// delays as widely as full-jitter, so callers stay near their expected
/// backoff while still desynchronizing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of failed attempts that should still trigger retry handling.
    pub max_attempts: u32,
    /// Base backoff delay in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum backoff delay in milliseconds (hard ceiling after jitter).
    pub max_delay_ms: u64,
    /// Error classes considered retryable by default.
    pub retryable_errors: Vec<ErrorClass>,
}

impl RetryPolicy {
    /// Build a policy tuned for provider rate-limit errors.
    ///
    /// Uses the rate-limit-specific defaults from `roko_core::defaults`:
    /// - 2 s base delay (doubles each attempt: 2 s, 4 s, 8 s, ...)
    /// - 5 max attempts (`DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS`)
    /// - 30 s ceiling
    ///
    /// Jitter is ±25% around the exponential delay, capped at `max_delay_ms`.
    #[must_use]
    pub fn for_rate_limit() -> Self {
        use roko_core::defaults::{
            DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS, DEFAULT_RATE_LIMIT_RETRY_BASE_DELAY_MS,
            DEFAULT_RATE_LIMIT_RETRY_MAX_BACKOFF_MS,
        };
        Self {
            max_attempts: DEFAULT_RATE_LIMIT_RETRY_ATTEMPTS, // 5
            base_delay_ms: DEFAULT_RATE_LIMIT_RETRY_BASE_DELAY_MS, // 2_000
            max_delay_ms: DEFAULT_RATE_LIMIT_RETRY_MAX_BACKOFF_MS, // 30_000
            retryable_errors: vec![
                ErrorClass::RateLimit,
                ErrorClass::ServerError,
                ErrorClass::Timeout,
            ],
        }
    }

    /// Returns an exponential-backoff delay with ±25% jitter for the given attempt.
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let mut rng = rand::thread_rng();
        self.delay_for_attempt_with_rng(attempt, &mut rng)
    }

    /// Compute the delay for `attempt` using the supplied RNG (testable variant).
    ///
    /// Algorithm:
    /// 1. Exponential: `base_delay_ms * 2^attempt`, saturating, capped at `max_delay_ms`.
    /// 2. ±25% jitter: multiply by a uniform random factor in `[0.75, 1.25]`.
    /// 3. Re-cap the jittered result at `max_delay_ms`.
    ///
    /// Returns 0 when `max_delay_ms` is 0 (test/no-op policies), otherwise
    /// guarantees a minimum of 1 ms.
    fn delay_for_attempt_with_rng<R: Rng + ?Sized>(&self, attempt: u32, rng: &mut R) -> u64 {
        // Fast path: zero-delay policies are used by tests that skip sleeping.
        if self.max_delay_ms == 0 {
            return 0;
        }
        let exp_delay = self.base_delay_ms.saturating_mul(1u64 << attempt.min(10));
        let capped = exp_delay.min(self.max_delay_ms);
        // ±25% jitter: factor in [0.75, 1.25].
        let factor: f64 = rng.gen_range(0.75_f64..=1.25_f64);
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation
        )]
        let jittered = ((capped as f64) * factor) as u64;
        jittered.clamp(1, self.max_delay_ms)
    }

    /// Returns whether the given provider error should be retried.
    #[must_use]
    pub fn should_retry(&self, error: &ProviderError, attempt: u32) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }

        match error {
            ProviderError::RateLimit { .. } => true,
            ProviderError::AuthFailure => false,
            ProviderError::ContentPolicy => false,
            ProviderError::Timeout => true,
            ProviderError::ServerError(_) => true,
            ProviderError::ContextOverflow => false,
            _ => attempt < 2,
        }
    }

    /// Returns the provider-specified retry delay when present, otherwise uses
    /// exponential backoff with ±25% jitter.
    #[must_use]
    pub fn delay_with_retry_after(&self, attempt: u32, retry_after_ms: Option<u64>) -> u64 {
        retry_after_ms.unwrap_or_else(|| self.delay_for_attempt(attempt))
    }

    /// Compute the backoff delay for a rate-limit error at the given attempt.
    ///
    /// Prefers `retry_after_ms` from the provider response when available.
    /// Otherwise falls back to exponential backoff with ±25% jitter
    /// (`base_delay_ms * 2^attempt`, capped at `max_delay_ms`).
    #[must_use]
    pub fn rate_limit_delay(&self, attempt: u32, retry_after_ms: Option<u64>) -> u64 {
        if let Some(provider_hint) = retry_after_ms {
            return provider_hint;
        }
        self.delay_for_attempt(attempt)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        use roko_core::defaults::{
            DEFAULT_RETRY_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY_MS, DEFAULT_RETRY_MAX_BACKOFF_MS,
        };
        Self {
            max_attempts: DEFAULT_RETRY_ATTEMPTS,
            base_delay_ms: DEFAULT_RETRY_BASE_DELAY_MS,
            max_delay_ms: DEFAULT_RETRY_MAX_BACKOFF_MS,
            retryable_errors: vec![
                ErrorClass::RateLimit,
                ErrorClass::Timeout,
                ErrorClass::ServerError,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// ±25% jitter around `base * 2^attempt` — verify bounds and spread.
    #[test]
    fn jitter_25pct_stays_within_bounds() {
        let policy = RetryPolicy {
            base_delay_ms: 1_000,
            max_delay_ms: 30_000,
            ..RetryPolicy::default()
        };
        // attempt 0: base = 1000, jitter window [750, 1250] capped at 30_000
        let lo = 750u64;
        let hi = 1_250u64;
        let mut rng = StdRng::seed_from_u64(7);
        let mut unique = std::collections::BTreeSet::new();

        for _ in 0..200 {
            let delay = policy.delay_for_attempt_with_rng(0, &mut rng);
            assert!(delay >= lo, "delay {delay} below ±25% floor {lo}");
            assert!(delay <= hi, "delay {delay} above ±25% ceiling {hi}");
            unique.insert(delay);
        }

        assert!(
            unique.len() >= 50,
            "expected spread across the ±25% window, got {} unique delays",
            unique.len()
        );
    }

    /// The cap at `max_delay_ms` applies after jitter, so delays never exceed it.
    #[test]
    fn jitter_never_exceeds_max_delay_ms() {
        let policy = RetryPolicy {
            base_delay_ms: 1_000,
            max_delay_ms: 30_000,
            ..RetryPolicy::default()
        };
        let mut rng = StdRng::seed_from_u64(13);
        // At attempt 10 the uncapped exponential would overflow; verify we stay at max.
        for _ in 0..100 {
            let delay = policy.delay_for_attempt_with_rng(10, &mut rng);
            assert!(
                delay <= policy.max_delay_ms,
                "delay {delay} exceeded max_delay_ms {}",
                policy.max_delay_ms
            );
        }
    }

    #[test]
    fn delay_with_retry_after_prefers_provider_hint() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_with_retry_after(2, Some(4_200)), 4_200);
    }

    #[test]
    fn should_retry_matches_provider_error_classification() {
        let policy = RetryPolicy::default();

        assert!(policy.should_retry(&ProviderError::Timeout, 0));
        assert!(policy.should_retry(&ProviderError::ServerError(503), 1));
        assert!(policy.should_retry(
            &ProviderError::RateLimit {
                retry_after_ms: Some(2_000)
            },
            2
        ));
        assert!(!policy.should_retry(&ProviderError::AuthFailure, 0));
        assert!(!policy.should_retry(&ProviderError::ContentPolicy, 0));
        assert!(!policy.should_retry(&ProviderError::ContextOverflow, 0));
        assert!(policy.should_retry(&ProviderError::Other("unknown".into()), 1));
        assert!(!policy.should_retry(&ProviderError::Other("unknown".into()), 2));
    }

    #[test]
    fn for_rate_limit_uses_higher_base_delay() {
        let policy = RetryPolicy::for_rate_limit();
        assert_eq!(policy.base_delay_ms, 2_000);
        assert_eq!(policy.max_attempts, 5);
        // attempt 0: base = 2000, ±25% window = [1500, 2500]
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let delay = policy.delay_for_attempt_with_rng(0, &mut rng);
            assert!(
                delay >= 1_500,
                "rate limit delay {delay} below ±25% floor 1500"
            );
            assert!(
                delay <= 2_500,
                "rate limit delay {delay} above ±25% ceiling 2500"
            );
        }
    }

    #[test]
    fn for_rate_limit_exponential_growth() {
        let policy = RetryPolicy::for_rate_limit();
        let mut rng = StdRng::seed_from_u64(99);
        // attempt 1: base * 2 = 4000, ±25% -> [3000, 5000]
        for _ in 0..50 {
            let delay = policy.delay_for_attempt_with_rng(1, &mut rng);
            assert!(
                delay >= 3_000,
                "attempt 1 delay {delay} below ±25% floor 3000"
            );
            assert!(
                delay <= 5_000,
                "attempt 1 delay {delay} above ±25% ceiling 5000"
            );
        }
        // attempt 2: base * 4 = 8000, ±25% -> [6000, 10000]
        for _ in 0..50 {
            let delay = policy.delay_for_attempt_with_rng(2, &mut rng);
            assert!(
                delay >= 6_000,
                "attempt 2 delay {delay} below ±25% floor 6000"
            );
            assert!(
                delay <= 10_000,
                "attempt 2 delay {delay} above ±25% ceiling 10000"
            );
        }
    }

    #[test]
    fn rate_limit_delay_respects_provider_hint() {
        let policy = RetryPolicy::for_rate_limit();
        // Provider says wait 5000ms
        assert_eq!(policy.rate_limit_delay(0, Some(5_000)), 5_000);
        // Provider says wait 200ms; we respect it as-is (no floor enforced)
        assert_eq!(policy.rate_limit_delay(0, Some(200)), 200);
        // No provider hint -> falls back to jittered exponential: [1500, 2500]
        let delay = policy.rate_limit_delay(0, None);
        assert!(delay >= 1_500, "no-hint delay {delay} below 1500");
        assert!(delay <= 2_500, "no-hint delay {delay} above 2500");
    }

    /// Verify the doubling sequence produces the right exponential base values.
    #[test]
    fn exponential_sequence_doubles() {
        // With no jitter (factor=1.0 always), we'd get exact doubling.
        // Use many samples and verify the average is near the expected base.
        let policy = RetryPolicy {
            base_delay_ms: 1_000,
            max_delay_ms: 30_000,
            ..RetryPolicy::default()
        };
        let mut rng = StdRng::seed_from_u64(55);
        let samples = 500u64;

        // attempt 0: expected base = 1000, ±25% -> [750, 1250]
        let sum0: u64 = (0..samples)
            .map(|_| policy.delay_for_attempt_with_rng(0, &mut rng))
            .sum();
        let avg0 = sum0 / samples;
        assert!(
            avg0 >= 750 && avg0 <= 1_250,
            "attempt 0 avg {avg0} out of range"
        );

        // attempt 1: expected base = 2000, ±25% -> [1500, 2500]
        let sum1: u64 = (0..samples)
            .map(|_| policy.delay_for_attempt_with_rng(1, &mut rng))
            .sum();
        let avg1 = sum1 / samples;
        assert!(
            avg1 >= 1_500 && avg1 <= 2_500,
            "attempt 1 avg {avg1} out of range"
        );

        // attempt 2: expected base = 4000, ±25% -> [3000, 5000]
        let sum2: u64 = (0..samples)
            .map(|_| policy.delay_for_attempt_with_rng(2, &mut rng))
            .sum();
        let avg2 = sum2 / samples;
        assert!(
            avg2 >= 3_000 && avg2 <= 5_000,
            "attempt 2 avg {avg2} out of range"
        );

        // avg should roughly double each attempt
        assert!(
            avg1 > avg0,
            "avg should grow: attempt 1 ({avg1}) > attempt 0 ({avg0})"
        );
        assert!(
            avg2 > avg1,
            "avg should grow: attempt 2 ({avg2}) > attempt 1 ({avg1})"
        );
    }
}
