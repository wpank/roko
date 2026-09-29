//! Environments for the child processes roko spawns.
//!
//! roko loads `~/.roko/.env` and `<workdir>/.roko/.env` into its own
//! environment at startup, and those files hold provider API keys. A child
//! inherits the whole environment unless told otherwise, so a verify command,
//! an agent-written test or the agent itself could read, and spend on, every
//! key. Two policies decide what a child sees:
//!
//! - **Gate commands** (authored verify steps and build/test gates) start from
//!   an empty environment and get only allowlisted variables: system basics,
//!   locale, toolchain and proxy settings, and `ROKO_*`. Credential-looking
//!   names and names roko loaded from a `.env` file are dropped even when the
//!   allowlist matches them. See [`gate_env`].
//! - **Provider CLIs** (Claude, Codex, Gemini, Cursor, Hermes, OpenClaw) keep
//!   their inherited environment minus the provider credentials they do not
//!   own. See [`CredentialScrub`].
//!
//! Both accept passthrough patterns from config (`[gates] env_passthrough`,
//! `[agent] env_passthrough`): an exact name (`DATABASE_URL`) or a prefix
//! ending in `*` (`AWS_*`). A passthrough match wins over every exclusion.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::agent::ProviderKind;
use crate::provider_catalog;

// ---- .env names ------------------------------------------------------------

/// Names of the variables roko loaded from its `.env` files at startup.
///
/// Only names are kept; the values stay in the process environment. Both
/// child policies treat every listed name as a secret by default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DotenvNames {
    listed: BTreeSet<String>,
    supplied: BTreeSet<String>,
}

impl DotenvNames {
    /// An empty record.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            listed: BTreeSet::new(),
            supplied: BTreeSet::new(),
        }
    }

    /// Record `name`, listed in a loaded `.env` file. `supplied_value` is true
    /// when the file provided the value roko now sees: the variable was unset
    /// before, or a project file replaced a different value.
    pub fn insert(&mut self, name: impl Into<String>, supplied_value: bool) {
        let name = name.into();
        if supplied_value {
            self.supplied.insert(name.clone());
        }
        self.listed.insert(name);
    }

    /// Whether any loaded `.env` file lists `name`.
    #[must_use]
    pub fn is_listed(&self, name: &str) -> bool {
        self.listed.contains(name)
    }

    /// Whether `name`'s current value came from a `.env` file rather than
    /// from the environment roko was started with.
    #[must_use]
    pub fn supplied_value(&self, name: &str) -> bool {
        self.supplied.contains(name)
    }
}

static STARTUP_DOTENV: OnceLock<DotenvNames> = OnceLock::new();

/// Record the `.env` names roko loaded at process startup. The first call
/// wins; later calls are ignored.
pub fn record_startup_dotenv(names: DotenvNames) {
    let _ = STARTUP_DOTENV.set(names);
}

/// The `.env` names recorded at startup; empty when nothing was recorded.
#[must_use]
pub fn startup_dotenv() -> &'static DotenvNames {
    static EMPTY: DotenvNames = DotenvNames::new();
    STARTUP_DOTENV.get().unwrap_or(&EMPTY)
}

// ---- Name classification ----------------------------------------------------

/// Whether `name` matches a passthrough `pattern`: an exact name, or a prefix
/// followed by `*` (`"AWS_*"`).
#[must_use]
pub fn matches_env_pattern(name: &str, pattern: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == pattern,
    }
}

fn matches_any(name: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| matches_env_pattern(name, pattern.trim()))
}

/// `_`-separated name segments that mark a variable as a credential.
const SECRET_SEGMENTS: &[&str] = &[
    "KEY",
    "KEYS",
    "APIKEY",
    "TOKEN",
    "AUTHTOKEN",
    "SECRET",
    "SECRETS",
    "PASSWORD",
    "PASSWD",
    "PASSPHRASE",
    "CREDENTIAL",
    "CREDENTIALS",
];

/// Whether `name` looks like it holds a credential: one of its
/// `_`-separated segments is `KEY`, `TOKEN`, `SECRET`, `PASSWORD` or a
/// relative, in any case (`OPENAI_API_KEY`, `CARGO_REGISTRY_TOKEN`,
/// `ROKO_SECRET_LLM_ANTHROPIC`, `npm_config__authToken`).
#[must_use]
pub fn is_secret_env_name(name: &str) -> bool {
    name.split('_').any(|segment| {
        SECRET_SEGMENTS
            .iter()
            .any(|secret| segment.eq_ignore_ascii_case(secret))
    })
}

// ---- Gate commands ---------------------------------------------------------

/// Variables a gate command inherits by exact name.
const GATE_ENV_NAMES: &[&str] = &[
    // System, shell and terminal.
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TERM",
    "COLORTERM",
    "TMPDIR",
    "TZ",
    "CI",
    "NO_COLOR",
    "FORCE_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    // Locale (plus `LC_*` below).
    "LANG",
    "LANGUAGE",
    // Toolchains and native builds.
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "VIRTUAL_ENV",
    "JAVA_HOME",
    "SDKROOT",
    "DEVELOPER_DIR",
    "MACOSX_DEPLOYMENT_TARGET",
    "CC",
    "CXX",
    "AR",
    "LD",
    "CFLAGS",
    "CXXFLAGS",
    "CPPFLAGS",
    "LDFLAGS",
    "LIBRARY_PATH",
    "CPATH",
    "LD_LIBRARY_PATH",
    "LIBCLANG_PATH",
    "LLVM_CONFIG_PATH",
    "PROTOC",
    "PROTOC_INCLUDE",
    // Network proxies.
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "all_proxy",
    // Windows equivalents of the system basics.
    "TEMP",
    "TMP",
    "USERPROFILE",
    "SYSTEMROOT",
    "COMSPEC",
    "PATHEXT",
    "WINDIR",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "SYSTEMDRIVE",
    "HOMEDRIVE",
    "HOMEPATH",
    "USERNAME",
];

/// Variables a gate command inherits by prefix.
const GATE_ENV_PREFIXES: &[&str] = &[
    "LC_",
    "XDG_",
    "CARGO_",
    "RUSTUP_",
    "RUSTC",
    "RUST_",
    "SCCACHE_",
    "NODE_",
    "NPM_",
    "npm_config_",
    "NVM_",
    "PNPM_",
    "YARN_",
    "GO",
    "CGO_",
    "PYTHON",
    "PKG_CONFIG",
    "CMAKE_",
    "OPENSSL_",
    "SSL_CERT_",
    "ROKO_",
];

/// Prefixes that look like a toolchain prefix but are not (`GO` vs `GOOGLE_*`).
const GATE_ENV_PREFIX_EXCEPTIONS: &[&str] = &["GOOGLE"];

/// Whether the default gate allowlist admits `name`, before the credential
/// and `.env` exclusions apply.
#[must_use]
pub fn gate_env_allowlisted(name: &str) -> bool {
    // Windows environment names are case-insensitive (`Path`, `SystemRoot`).
    let upper;
    let name = if cfg!(windows) {
        upper = name.to_ascii_uppercase();
        upper.as_str()
    } else {
        name
    };
    GATE_ENV_NAMES.contains(&name)
        || (GATE_ENV_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
            && !GATE_ENV_PREFIX_EXCEPTIONS
                .iter()
                .any(|prefix| name.starts_with(prefix)))
}

/// Whether a gate command inherits `name`: it matches `passthrough`, or it is
/// allowlisted, does not look like a credential, and was not loaded from a
/// `.env` file.
#[must_use]
pub fn gate_env_keeps(name: &str, passthrough: &[String], dotenv: &DotenvNames) -> bool {
    if matches_any(name, passthrough) {
        return true;
    }
    !dotenv.is_listed(name) && !is_secret_env_name(name) && gate_env_allowlisted(name)
}

/// The environment a gate command inherits from `parent` (roko's own
/// environment in production): the entries [`gate_env_keeps`] admits. The
/// caller clears the child's environment first and adds the gate's explicit
/// variables afterwards.
pub fn gate_env<V>(
    parent: impl IntoIterator<Item = (String, V)>,
    passthrough: &[String],
    dotenv: &DotenvNames,
) -> Vec<(String, V)> {
    parent
        .into_iter()
        .filter(|(name, _)| gate_env_keeps(name, passthrough, dotenv))
        .collect()
}

// ---- Provider CLIs ---------------------------------------------------------

/// LLM provider credentials roko knows by name, besides the keys in the
/// [provider catalog](crate::provider_catalog).
pub const PROVIDER_KEY_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "OPENAI_API_KEY",
    "CODEX_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "PERPLEXITY_API_KEY",
    "CEREBRAS_API_KEY",
    "MOONSHOT_API_KEY",
    "ZAI_API_KEY",
    "OPENROUTER_API_KEY",
    "GROQ_API_KEY",
    "MISTRAL_API_KEY",
    "DEEPSEEK_API_KEY",
    "XAI_API_KEY",
    "TOGETHER_API_KEY",
    "FIREWORKS_API_KEY",
    "CURSOR_API_KEY",
];

/// Whether `name` is a known LLM provider credential.
#[must_use]
pub fn is_provider_key_var(name: &str) -> bool {
    PROVIDER_KEY_VARS.contains(&name)
        || provider_catalog::catalog()
            .iter()
            .any(|entry| !entry.api_key_env.is_empty() && entry.api_key_env == name)
}

/// The credentials a provider CLI reads itself when the user runs it
/// directly. Hermes and OpenClaw route to many providers from their own
/// configuration, so every known key counts as theirs.
const fn native_credentials(kind: ProviderKind) -> &'static [&'static str] {
    match kind {
        ProviderKind::ClaudeCli => &[
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ],
        ProviderKind::CodexCli => &["OPENAI_API_KEY", "CODEX_API_KEY"],
        ProviderKind::GeminiCli => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        ProviderKind::CursorCli | ProviderKind::CursorAcp => &["CURSOR_API_KEY"],
        ProviderKind::Hermes | ProviderKind::OpenClaw => PROVIDER_KEY_VARS,
        ProviderKind::AnthropicApi
        | ProviderKind::OpenAiCompat
        | ProviderKind::PerplexityApi
        | ProviderKind::GeminiApi
        | ProviderKind::CerebrasApi => &[],
    }
}

/// Which inherited credentials a provider CLI subprocess loses.
///
/// A provider CLI keeps its inherited environment except for:
/// - known LLM provider keys ([`is_provider_key_var`]),
/// - every name roko loaded from a `.env` file, and
/// - roko's own credentials (`ROKO_*` names that look secret, such as a serve
///   admin key).
///
/// Three exemptions apply. Names matching a `keep` pattern stay (the
/// provider's `api_key_env`, `[agent] env_passthrough`, variables an MCP
/// config the agent receives refers to). The CLI's own credentials stay when
/// they came from the environment roko was started with rather than from
/// roko's `.env` files: a user's exported `ANTHROPIC_API_KEY` still reaches
/// `claude` exactly as it would without roko, while a key only roko loaded
/// does not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CredentialScrub {
    owner: Option<ProviderKind>,
    keep: Vec<String>,
}

impl CredentialScrub {
    /// The policy for a subprocess of provider `kind`.
    #[must_use]
    pub fn for_kind(kind: ProviderKind) -> Self {
        Self {
            owner: Some(kind),
            keep: Vec::new(),
        }
    }

    /// Keep variables matching `pattern` (an exact name or `PREFIX*`).
    #[must_use]
    pub fn keep(mut self, pattern: impl Into<String>) -> Self {
        let pattern = pattern.into();
        if !pattern.trim().is_empty() {
            self.keep.push(pattern);
        }
        self
    }

    /// Keep variables matching any of `patterns`.
    #[must_use]
    pub fn keep_all<I, S>(self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        patterns.into_iter().fold(self, Self::keep)
    }

    /// Whether the subprocess loses the inherited variable `name`.
    #[must_use]
    pub fn strips(&self, name: &str, dotenv: &DotenvNames) -> bool {
        let credential = is_provider_key_var(name)
            || dotenv.is_listed(name)
            || (name.starts_with("ROKO_") && is_secret_env_name(name));
        if !credential || matches_any(name, &self.keep) {
            return false;
        }
        let own_credential = self
            .owner
            .is_some_and(|kind| native_credentials(kind).contains(&name));
        !(own_credential && !dotenv.supplied_value(name))
    }

    /// The names in `inherited` this policy removes.
    pub fn names_to_strip<'a>(
        &self,
        inherited: impl IntoIterator<Item = &'a str>,
        dotenv: &DotenvNames,
    ) -> Vec<String> {
        inherited
            .into_iter()
            .filter(|name| self.strips(name, dotenv))
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    fn parent(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect()
    }

    fn kept_names(env: &[(String, String)]) -> Vec<&str> {
        env.iter().map(|(name, _)| name.as_str()).collect()
    }

    #[test]
    fn patterns_match_exact_names_and_prefixes() {
        assert!(matches_env_pattern("AWS_PROFILE", "AWS_*"));
        assert!(matches_env_pattern("DATABASE_URL", "DATABASE_URL"));
        assert!(!matches_env_pattern("DATABASE_URL_RO", "DATABASE_URL"));
        assert!(!matches_env_pattern("MY_AWS_PROFILE", "AWS_*"));
        assert!(matches_env_pattern("ANYTHING", "*"));
    }

    #[test]
    fn credential_names_are_recognised() {
        for name in [
            "OPENAI_API_KEY",
            "GITHUB_TOKEN",
            "CARGO_REGISTRY_TOKEN",
            "NPM_TOKEN",
            "npm_config__authToken",
            "ROKO_SECRET_LLM_ANTHROPIC",
            "ROKO_ATTEST_SIGNING_KEY_HEX",
            "DB_PASSWORD",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ] {
            assert!(is_secret_env_name(name), "{name} should look secret");
        }
        for name in [
            "PATH",
            "CARGO_HOME",
            "ROKO_BIN",
            "ROKO_MAX_TOKENS",
            "RUST_LOG",
            "GOPATH",
        ] {
            assert!(!is_secret_env_name(name), "{name} should not look secret");
        }
    }

    #[test]
    fn gate_env_keeps_allowlist_and_drops_everything_else() {
        let env = gate_env(
            parent(&[
                ("PATH", "/usr/bin"),
                ("HOME", "/home/dev"),
                ("LANG", "en_US.UTF-8"),
                ("LC_ALL", "C"),
                ("CARGO_HOME", "/home/dev/.cargo"),
                ("CARGO_TARGET_DIR", "/tmp/target"),
                ("RUSTC_WRAPPER", "sccache"),
                ("RUST_BACKTRACE", "1"),
                ("GOPATH", "/go"),
                ("npm_config_cache", "/tmp/npm"),
                ("ROKO_BIN", "/repo/target/debug/roko"),
                ("https_proxy", "http://proxy:3128"),
                ("OPENAI_API_KEY", "sk-test-not-real"),
                ("ANTHROPIC_API_KEY", "sk-ant-test-not-real"),
                ("GITHUB_TOKEN", "ghp_test"),
                ("SSH_AUTH_SOCK", "/tmp/agent.sock"),
                ("AWS_SECRET_ACCESS_KEY", "aws-test"),
                ("GOOGLE_CLOUD_PROJECT", "proj"),
                ("RANDOM_APP_SETTING", "x"),
            ]),
            &[],
            &DotenvNames::new(),
        );
        assert_eq!(
            kept_names(&env),
            vec![
                "PATH",
                "HOME",
                "LANG",
                "LC_ALL",
                "CARGO_HOME",
                "CARGO_TARGET_DIR",
                "RUSTC_WRAPPER",
                "RUST_BACKTRACE",
                "GOPATH",
                "npm_config_cache",
                "ROKO_BIN",
                "https_proxy",
            ]
        );
    }

    #[test]
    fn gate_env_drops_secret_looking_names_even_under_allowed_prefixes() {
        let env = gate_env(
            parent(&[
                ("CARGO_REGISTRY_TOKEN", "cio-test"),
                ("NPM_TOKEN", "npm-test"),
                ("ROKO_API_KEY", "roko-test"),
                ("ROKO_SERVER_AUTH_TOKEN", "serve-test"),
                ("ROKO_SECRET_LLM_ANTHROPIC", "sk-ant-test"),
                ("ROKO_LOG", "debug"),
            ]),
            &[],
            &DotenvNames::new(),
        );
        assert_eq!(kept_names(&env), vec!["ROKO_LOG"]);
    }

    #[test]
    fn gate_env_treats_dotenv_names_as_secrets() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("ROKO_LOG", true);
        dotenv.insert("RUST_LOG", false);
        let env = gate_env(
            parent(&[
                ("ROKO_LOG", "debug"),
                ("RUST_LOG", "info"),
                ("PATH", "/bin"),
            ]),
            &[],
            &dotenv,
        );
        assert_eq!(kept_names(&env), vec!["PATH"]);
    }

    #[test]
    fn gate_passthrough_admits_names_and_prefixes_even_when_secret() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("DATABASE_URL", true);
        let env = gate_env(
            parent(&[
                ("DATABASE_URL", "postgres://localhost/test"),
                ("AWS_PROFILE", "dev"),
                ("AWS_SECRET_ACCESS_KEY", "aws-test"),
                ("OPENAI_API_KEY", "sk-test-not-real"),
            ]),
            &patterns(&["DATABASE_URL", "AWS_*"]),
            &dotenv,
        );
        assert_eq!(
            kept_names(&env),
            vec!["DATABASE_URL", "AWS_PROFILE", "AWS_SECRET_ACCESS_KEY"]
        );
    }

    #[test]
    fn gate_env_passes_os_string_values_through() {
        let env = gate_env(
            vec![("PATH".to_string(), std::ffi::OsString::from("/bin"))],
            &[],
            &DotenvNames::new(),
        );
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn provider_keys_include_the_catalog() {
        assert!(is_provider_key_var("OPENAI_API_KEY"));
        assert!(is_provider_key_var("NVIDIA_API_KEY"));
        assert!(!is_provider_key_var("GITHUB_TOKEN"));
        assert!(!is_provider_key_var("PATH"));
    }

    #[test]
    fn scrub_strips_other_providers_keys_from_a_cli() {
        let scrub = CredentialScrub::for_kind(ProviderKind::ClaudeCli);
        let stripped = scrub.names_to_strip(
            [
                "PATH",
                "HOME",
                "OPENAI_API_KEY",
                "GEMINI_API_KEY",
                "GITHUB_TOKEN",
                "ROKO_SERVE_AUTH_API_KEY",
                "ROKO_LOG",
            ],
            &DotenvNames::new(),
        );
        assert_eq!(
            stripped,
            vec![
                "OPENAI_API_KEY",
                "GEMINI_API_KEY",
                "ROKO_SERVE_AUTH_API_KEY"
            ]
        );
    }

    #[test]
    fn scrub_keeps_a_cli_credential_from_the_users_shell() {
        let scrub = CredentialScrub::for_kind(ProviderKind::ClaudeCli);
        // Exported in the shell roko started from: `claude` would see it anyway.
        assert!(!scrub.strips("ANTHROPIC_API_KEY", &DotenvNames::new()));
        // Also listed in `~/.roko/.env`, but the shell value won.
        let mut dotenv = DotenvNames::new();
        dotenv.insert("ANTHROPIC_API_KEY", false);
        assert!(!scrub.strips("ANTHROPIC_API_KEY", &dotenv));
    }

    #[test]
    fn scrub_strips_a_cli_credential_only_roko_loaded() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("ANTHROPIC_API_KEY", true);
        let scrub = CredentialScrub::for_kind(ProviderKind::ClaudeCli);
        assert!(scrub.strips("ANTHROPIC_API_KEY", &dotenv));
        // Naming it as the provider's `api_key_env` opts back in.
        let scrub = scrub.keep("ANTHROPIC_API_KEY");
        assert!(!scrub.strips("ANTHROPIC_API_KEY", &dotenv));
    }

    #[test]
    fn scrub_strips_every_dotenv_name_unless_kept() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("AWS_PROFILE", true);
        dotenv.insert("CUSTOM_SETTING", true);
        let scrub = CredentialScrub::for_kind(ProviderKind::CodexCli);
        assert!(scrub.strips("AWS_PROFILE", &dotenv));
        assert!(scrub.strips("CUSTOM_SETTING", &dotenv));
        let scrub = scrub.keep_all(["AWS_*"]);
        assert!(!scrub.strips("AWS_PROFILE", &dotenv));
        assert!(scrub.strips("CUSTOM_SETTING", &dotenv));
    }

    #[test]
    fn generic_scrub_has_no_own_credentials() {
        let scrub = CredentialScrub::default();
        assert!(scrub.strips("ANTHROPIC_API_KEY", &DotenvNames::new()));
        assert!(scrub.strips("OPENAI_API_KEY", &DotenvNames::new()));
        assert!(!scrub.strips("PATH", &DotenvNames::new()));
    }

    #[test]
    fn harness_scrub_keeps_shell_keys_and_strips_dotenv_keys() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("OPENROUTER_API_KEY", true);
        let scrub = CredentialScrub::for_kind(ProviderKind::Hermes);
        assert!(!scrub.strips("OPENAI_API_KEY", &dotenv));
        assert!(scrub.strips("OPENROUTER_API_KEY", &dotenv));
    }
}
