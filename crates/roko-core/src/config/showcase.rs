//! `[showcase]` in `roko.toml` (S11 §4.7): the passphrase-gated public showcase that `roko serve`
//! runs on Fly.
//!
//! It is off by default. With `enabled = true`, serve refuses to start half configured: auth
//! must enforce, `public_origin` must be set, `ROKO_SHOWCASE_PASSPHRASE_HASH` must hold an
//! Argon2id PHC string, and Privy must be off (`roko_serve::validate_showcase_mode`). The
//! session, login, caps, idle and models tables are read by the showcase routes. Their defaults
//! follow D22 (sessions and lockout) and D37 (the idle timer); `caps.total_usd` is open question
//! D41.

use serde::{Deserialize, Serialize};

/// Where the verified replay bundles live, relative to the workspace.
pub const DEFAULT_BUNDLE_ROOT: &str = ".roko/showcase/bundles";

/// The file whose presence keeps an idle showcase running, relative to the workspace.
pub const DEFAULT_HOLD_FILE: &str = ".roko/showcase/hold";

/// The environment variable that holds the passphrase's Argon2id PHC string (S11 §4.3). It is a
/// deploy secret, never a `roko.toml` key.
pub const PASSPHRASE_HASH_ENV: &str = "ROKO_SHOWCASE_PASSPHRASE_HASH";

/// `[showcase]` in `roko.toml` (S11 §4.7). Every key is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseConfig {
    /// Showcase mode: serve checks its startup rules and serves the showcase.
    pub enabled: bool,
    /// The origin visitors load the showcase from, such as `https://roko-showcase.fly.dev`.
    /// Login requires an exact `Origin` match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_origin: Option<String>,
    /// The live slices (run console, audit draws). Off: F1 is replay only.
    pub live_enabled: bool,
    /// Where the verified replay bundles live.
    pub bundle_root: String,
    /// Whether the portal is mounted at `/`; when it is not, `/` redirects to `/demo/`.
    pub portal_mounted: bool,
    /// The session cookie and its lifetimes.
    pub session: ShowcaseSessionConfig,
    /// Passphrase login: verification and lockout.
    pub login: ShowcaseLoginConfig,
    /// The global spending caps of the live slices.
    pub caps: ShowcaseCapsConfig,
    /// The idle timer.
    pub idle: ShowcaseIdleConfig,
    /// The models a live slice may reach, with their prices.
    pub models: ShowcaseModelsConfig,
}

impl Default for ShowcaseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            public_origin: None,
            live_enabled: false,
            bundle_root: DEFAULT_BUNDLE_ROOT.to_string(),
            portal_mounted: false,
            session: ShowcaseSessionConfig::default(),
            login: ShowcaseLoginConfig::default(),
            caps: ShowcaseCapsConfig::default(),
            idle: ShowcaseIdleConfig::default(),
            models: ShowcaseModelsConfig::default(),
        }
    }
}

/// `[showcase.session]`: the session cookie (S11 §4.3, D22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseSessionConfig {
    /// The cookie's name: `__Host-roko_session` in a deploy, `roko_session` in local tests.
    pub cookie_name: String,
    /// Whether the cookie is `Secure`. Local tests over plain HTTP turn it off.
    pub cookie_secure: bool,
    /// Absolute lifetime of a session: 72 hours.
    pub ttl_secs: u64,
    /// Idle (sliding) lifetime of a session: 12 hours.
    pub idle_ttl_secs: u64,
    /// Most sessions kept; the oldest go first.
    pub max_sessions: u32,
}

impl Default for ShowcaseSessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "__Host-roko_session".to_string(),
            cookie_secure: true,
            ttl_secs: 72 * 3600,
            idle_ttl_secs: 12 * 3600,
            max_sessions: 200,
        }
    }
}

/// `[showcase.login]`: passphrase verification and lockout (S11 §4.3, D22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseLoginConfig {
    /// Failures from one IP within `per_ip_window_secs` that block it.
    pub per_ip_max_failures: u32,
    /// The window per-IP failures are counted in.
    pub per_ip_window_secs: u64,
    /// How long a blocked IP stays blocked; each repeat doubles it.
    pub per_ip_block_secs: u64,
    /// The longest an IP block grows to.
    pub block_backoff_max_secs: u64,
    /// Failures from every IP within `global_window_secs` that disable login for everyone.
    pub global_max_failures: u32,
    /// The window global failures are counted in.
    pub global_window_secs: u64,
    /// How long login stays disabled after a global lockout.
    pub global_block_secs: u64,
    /// Argon2id verifications that may run at once.
    pub verify_concurrency: u32,
    /// Take the client IP from `Fly-Client-IP` instead of the socket peer. Only behind Fly's
    /// proxy, which sets it: anywhere else a client could.
    pub trust_fly_client_ip: bool,
}

impl Default for ShowcaseLoginConfig {
    fn default() -> Self {
        Self {
            per_ip_max_failures: 5,
            per_ip_window_secs: 900,
            per_ip_block_secs: 900,
            block_backoff_max_secs: 86_400,
            global_max_failures: 50,
            global_window_secs: 3_600,
            global_block_secs: 1_800,
            verify_concurrency: 2,
            trust_fly_client_ip: false,
        }
    }
}

/// `[showcase.caps]`: the global spending caps of the live slices (S11 §4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseCapsConfig {
    /// Spend per UTC day, USD.
    pub daily_usd: f64,
    /// Spend over the showcase's life, USD (open question D41).
    pub total_usd: f64,
    /// Spend per live run, USD.
    pub per_run_usd: f64,
    /// Live runs at once.
    pub max_concurrent_runs: u32,
    /// Tasks in one live run.
    pub max_tasks_per_run: u32,
    /// Wall time of one live run.
    pub max_run_wall_secs: u64,
    /// Live runs one session may start per day.
    pub runs_per_session_per_day: u32,
}

impl Default for ShowcaseCapsConfig {
    fn default() -> Self {
        Self {
            daily_usd: 5.0,
            total_usd: 50.0,
            per_run_usd: 1.0,
            max_concurrent_runs: 1,
            max_tasks_per_run: 10,
            max_run_wall_secs: 1_800,
            runs_per_session_per_day: 4,
        }
    }
}

/// `[showcase.idle]`: the idle timer (S11 §4.6, D37).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseIdleConfig {
    /// Seconds with no request before serve exits, so the Machine stops: 20 minutes.
    pub exit_after_secs: u64,
    /// The longest an SSE stream stays open with no event.
    pub sse_max_idle_secs: u64,
    /// While this file exists, the idle timer leaves serve running.
    pub hold_file: String,
}

impl Default for ShowcaseIdleConfig {
    fn default() -> Self {
        Self {
            exit_after_secs: 1_200,
            sse_max_idle_secs: 1_800,
            hold_file: DEFAULT_HOLD_FILE.to_string(),
        }
    }
}

/// `[showcase.models]`: the models a live slice may reach (S11 §4.4). A model missing from
/// `allow` is refused as unpriced.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShowcaseModelsConfig {
    /// The dated price snapshot the rows copy, such as `prices-2026-09-28`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_snapshot_id: Option<String>,
    /// The allowlist, with each model's prices.
    pub allow: Vec<ShowcaseModelPrice>,
}

/// One allowlisted model and its prices, in USD per million tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowcaseModelPrice {
    /// `<provider>/<model>`, as the provider registry names it.
    pub id: String,
    /// Input tokens.
    pub input_per_m: f64,
    /// Cached input tokens, when the provider discounts them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_per_m: Option<f64>,
    /// Output tokens.
    pub output_per_m: f64,
}

impl ShowcaseConfig {
    /// Every problem with the section, as (key, problem). The startup rules that read
    /// `[serve.auth]` or the environment are serve's, checked when showcase mode starts.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        if let Some(origin) = &self.public_origin
            && !is_origin(origin)
        {
            problems.push((
                "public_origin",
                format!("public_origin ({origin}) must be an origin, such as https://host"),
            ));
        }
        if self.bundle_root.trim().is_empty() {
            problems.push((
                "bundle_root",
                "bundle_root must name a directory".to_string(),
            ));
        }

        let session = &self.session;
        if session.cookie_name.trim().is_empty() {
            problems.push(("session.cookie_name", "cookie_name must be set".to_string()));
        }
        if session.cookie_name.starts_with("__Host-") && !session.cookie_secure {
            problems.push((
                "session.cookie_secure",
                "a __Host- cookie must be Secure: browsers drop it otherwise".to_string(),
            ));
        }
        if session.idle_ttl_secs == 0 || session.idle_ttl_secs > session.ttl_secs {
            problems.push((
                "session.idle_ttl_secs",
                "idle_ttl_secs must be at least 1 and at most ttl_secs".to_string(),
            ));
        }
        if session.max_sessions == 0 {
            problems.push((
                "session.max_sessions",
                "max_sessions must be at least 1".to_string(),
            ));
        }

        let login = &self.login;
        for (key, value) in [
            ("login.per_ip_max_failures", login.per_ip_max_failures),
            ("login.global_max_failures", login.global_max_failures),
            ("login.verify_concurrency", login.verify_concurrency),
        ] {
            if value == 0 {
                problems.push((key, format!("{key} must be at least 1")));
            }
        }
        if login.per_ip_block_secs > login.block_backoff_max_secs {
            problems.push((
                "login.block_backoff_max_secs",
                "block_backoff_max_secs must be at least per_ip_block_secs".to_string(),
            ));
        }

        let caps = &self.caps;
        let spend = [caps.per_run_usd, caps.daily_usd, caps.total_usd];
        if spend.into_iter().any(bad_usd) {
            problems.push((
                "caps",
                "the caps must be finite and not negative".to_string(),
            ));
        } else if caps.per_run_usd > caps.daily_usd || caps.daily_usd > caps.total_usd {
            problems.push((
                "caps",
                "the caps must nest: per_run_usd <= daily_usd <= total_usd".to_string(),
            ));
        }
        if caps.max_concurrent_runs == 0 {
            problems.push((
                "caps.max_concurrent_runs",
                "max_concurrent_runs must be at least 1".to_string(),
            ));
        }

        if self.idle.exit_after_secs == 0 {
            problems.push((
                "idle.exit_after_secs",
                "exit_after_secs must be at least 1".to_string(),
            ));
        }

        let mut ids = std::collections::BTreeSet::new();
        for row in &self.models.allow {
            let prices = [row.input_per_m, row.output_per_m];
            if prices.into_iter().chain(row.cached_per_m).any(bad_usd) {
                problems.push((
                    "models.allow",
                    format!("{}: prices must be finite and not negative", row.id),
                ));
            }
            if row.id.trim().is_empty() || !ids.insert(row.id.as_str()) {
                problems.push((
                    "models.allow",
                    format!("model ids must be set and distinct: {:?}", row.id),
                ));
            }
        }
        problems
    }
}

/// Whether `value` is an origin: `http://` or `https://`, then a host with no path.
fn is_origin(value: &str) -> bool {
    let host = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or_default();
    !host.is_empty() && !host.contains('/')
}

/// Whether `usd` is no amount of money: not finite, or negative.
fn bad_usd(usd: f64) -> bool {
    !usd.is_finite() || usd < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{LoadOptions, load_config_validated_with_options};
    use crate::config::schema::RokoConfig;

    /// `docker/showcase.roko.toml`'s `[showcase]` table (S11 §4.7).
    const FULL_TABLE: &str = r#"
[showcase]
enabled = true
public_origin = "https://roko-showcase.fly.dev"
live_enabled = false
bundle_root = "/data/.roko/showcase/bundles"
portal_mounted = false
session = { cookie_name = "__Host-roko_session", cookie_secure = true, ttl_secs = 259200, idle_ttl_secs = 43200, max_sessions = 200 }
login = { per_ip_max_failures = 5, per_ip_window_secs = 900, per_ip_block_secs = 900, block_backoff_max_secs = 86400, global_max_failures = 50, global_window_secs = 3600, global_block_secs = 1800, verify_concurrency = 2, trust_fly_client_ip = true }
caps = { daily_usd = 5.0, total_usd = 50.0, per_run_usd = 1.0, max_concurrent_runs = 1, max_tasks_per_run = 10, max_run_wall_secs = 1800, runs_per_session_per_day = 4 }
idle = { exit_after_secs = 1200, sse_max_idle_secs = 1800, hold_file = "/data/.roko/showcase/hold" }

[showcase.models]
price_snapshot_id = "prices-2026-09-28"
allow = [
  { id = "cerebras/gpt-oss-120b", input_per_m = 0.35, output_per_m = 0.75 },
  { id = "zai/glm-4.7", input_per_m = 0.60, cached_per_m = 0.11, output_per_m = 2.20 },
]
"#;

    fn price(id: &str, input: f64, cached: Option<f64>, output: f64) -> ShowcaseModelPrice {
        ShowcaseModelPrice {
            id: id.to_string(),
            input_per_m: input,
            cached_per_m: cached,
            output_per_m: output,
        }
    }

    fn full_table() -> ShowcaseConfig {
        ShowcaseConfig {
            enabled: true,
            public_origin: Some("https://roko-showcase.fly.dev".to_string()),
            live_enabled: false,
            bundle_root: "/data/.roko/showcase/bundles".to_string(),
            portal_mounted: false,
            session: ShowcaseSessionConfig::default(),
            login: ShowcaseLoginConfig {
                trust_fly_client_ip: true,
                ..ShowcaseLoginConfig::default()
            },
            caps: ShowcaseCapsConfig::default(),
            idle: ShowcaseIdleConfig {
                hold_file: "/data/.roko/showcase/hold".to_string(),
                ..ShowcaseIdleConfig::default()
            },
            models: ShowcaseModelsConfig {
                price_snapshot_id: Some("prices-2026-09-28".to_string()),
                allow: vec![
                    price("cerebras/gpt-oss-120b", 0.35, None, 0.75),
                    price("zai/glm-4.7", 0.60, Some(0.11), 2.20),
                ],
            },
        }
    }

    /// 9320: the full table loads intact through the loader, which strips the keys its schema
    /// tree lacks: every value arrives, no unknown-key diagnostic fires, and no invariant
    /// objects. The section is off by default and round-trips through TOML.
    #[test]
    fn showcase_table_survives_loading() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("roko.toml"), FULL_TABLE).expect("write roko.toml");
        let options = LoadOptions {
            merge_global: false,
            apply_env_overrides: false,
            apply_hierarchical_env: false,
            strict_validation: false,
        };

        let loaded = load_config_validated_with_options(dir.path(), &options).expect("load");

        let showcase: Vec<_> = loaded
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.key.starts_with("showcase"))
            .collect();
        assert!(showcase.is_empty(), "{showcase:?}");
        assert_eq!(loaded.config().showcase, full_table());
        let value: toml::Value = toml::from_str(FULL_TABLE).expect("parse the table");
        let unknown = crate::config::loader::validate_known_config_paths(&value);
        assert!(unknown.is_empty(), "{unknown:?}");

        let config = ShowcaseConfig::default();
        assert!(!config.enabled);
        assert!(config.problems().is_empty());
        assert_eq!(RokoConfig::default().showcase, config);
        for section in [config, full_table()] {
            let text = toml::to_string(&section).expect("serialize the section");
            let back: ShowcaseConfig = toml::from_str(&text).expect("parse it back");
            assert_eq!(back, section);
        }
        assert!(toml::from_str::<ShowcaseConfig>("enable = true").is_err());
    }

    /// The section's checks, through `validate_invariants` too.
    #[test]
    fn showcase_problems_name_their_keys() {
        let mut bad = full_table();
        bad.public_origin = Some("https://roko-showcase.fly.dev/demo".to_string());
        bad.session.cookie_secure = false;
        bad.login.verify_concurrency = 0;
        bad.caps.per_run_usd = 10.0;
        bad.models.allow.push(price("zai/glm-4.7", -1.0, None, 2.20));
        let keys: Vec<&str> = bad.problems().into_iter().map(|(key, _)| key).collect();
        let expected = [
            "public_origin",
            "session.cookie_secure",
            "login.verify_concurrency",
            "caps",
            "models.allow",
            "models.allow",
        ];
        assert_eq!(keys, expected);
        assert!(is_origin("http://127.0.0.1:6677"));
        assert!(!is_origin("roko-showcase.fly.dev"));

        let mut roko = RokoConfig::default();
        roko.showcase.session.max_sessions = 0;
        let results = crate::config::validate_invariants(&roko);
        assert!(
            results
                .iter()
                .any(|result| result.config_path == "showcase.session.max_sessions")
        );
    }
}
