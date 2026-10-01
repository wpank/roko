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
//!
//! The commands agents run through roko's own tools (`bash`, `run_tests`, ACP's
//! `bash`) follow the gate policy, as verify steps do. [`apply_gate_env`],
//! [`CredentialScrub::apply`] and [`apply_credential_scrub_from`] put either
//! policy on a [`Command`].
//!
//! Keeping keys out of a child's environment is moot while the child can read
//! the files they come from, so this module also lists those files: see
//! [`key_file_paths`] and [`is_key_file`].

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::agent::ProviderKind;
use crate::config::loader::config_text_holds_secrets;
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

// ---- Commands --------------------------------------------------------------

/// roko's own environment: what a child inherits from in production.
/// Variables whose names are not Unicode are left out.
#[must_use]
pub fn process_env() -> Vec<(String, OsString)> {
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value)))
        .collect()
}

/// Give `cmd` the variables of `inherited` that [`gate_env`] admits, in place
/// of the environment it would inherit. Variables already set on `cmd` stay.
///
/// Gate commands start this way, and so do the commands agents run through
/// roko's own tools: both run code an agent wrote. `inherited` is roko's own
/// environment ([`process_env`]) in production; tests pass their own.
/// `passthrough` is `[gates] env_passthrough` for gates and
/// `[agent] env_passthrough` for tool commands.
pub fn apply_gate_env(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (String, OsString)>,
    passthrough: &[String],
) {
    let explicit = explicit_env(cmd);
    cmd.env_clear();
    cmd.envs(gate_env(inherited, passthrough, startup_dotenv()));
    restore_env(cmd, explicit);
}

impl CredentialScrub {
    /// Remove from `cmd` the variables of roko's own environment this policy
    /// strips. Variables set explicitly on `cmd` stay.
    pub fn apply(&self, cmd: &mut Command) {
        let explicit: BTreeSet<OsString> = cmd
            .get_envs()
            .filter(|(_, value)| value.is_some())
            .map(|(name, _)| name.to_os_string())
            .collect();
        let inherited: Vec<String> = std::env::vars_os()
            .filter_map(|(name, _)| name.into_string().ok())
            .filter(|name| !explicit.contains(OsStr::new(name)))
            .collect();
        for name in self.names_to_strip(inherited.iter().map(String::as_str), startup_dotenv()) {
            cmd.env_remove(name);
        }
    }
}

/// Give `cmd` the variables of `inherited` that `scrub` keeps, in place of
/// the environment it would inherit. Variables already set on `cmd` stay,
/// a credential too: the caller chose to pass it.
///
/// `inherited` is roko's own environment ([`process_env`]) in production;
/// tests pass their own.
pub fn apply_credential_scrub_from(
    cmd: &mut Command,
    scrub: &CredentialScrub,
    inherited: impl IntoIterator<Item = (String, OsString)>,
) {
    let explicit = explicit_env(cmd);
    let dotenv = startup_dotenv();
    let kept = inherited
        .into_iter()
        .filter(|(name, _)| !scrub.strips(name, dotenv));
    cmd.env_clear();
    cmd.envs(kept);
    restore_env(cmd, explicit);
}

/// The variables set or removed on `cmd` so far, which `env_clear` forgets.
fn explicit_env(cmd: &Command) -> Vec<(OsString, Option<OsString>)> {
    cmd.get_envs()
        .map(|(name, value)| (name.to_os_string(), value.map(OsStr::to_os_string)))
        .collect()
}

/// Set or remove again the variables [`explicit_env`] recorded.
fn restore_env(cmd: &mut Command, explicit: Vec<(OsString, Option<OsString>)>) {
    for (name, value) in explicit {
        match value {
            Some(value) => cmd.env(name, value),
            None => cmd.env_remove(name),
        };
    }
}

// ---- Key files -------------------------------------------------------------

/// Files in a `.roko` directory that hold provider keys or roko credentials:
/// `.env` (`~/.roko/.env` and `<workdir>/.roko/.env`, loaded at startup),
/// `secrets.toml` (`roko config secrets`), `credentials.json`
/// (`roko login`) and `config.toml` (`~/.roko/config.toml`, the global
/// config). The global config counts whether or not it holds a secret yet:
/// it can hold `serve.auth.api_key` and provider `extra_headers`, and a check
/// of its contents could not be repeated in the Claude CLI permission rules.
pub const KEY_FILE_NAMES: &[&str] = &[".env", "secrets.toml", "credentials.json", "config.toml"];

/// The files that hold provider keys and roko credentials for home directory
/// `home` and workdir `workdir`: each of [`KEY_FILE_NAMES`] in `~/.roko` and
/// in `<workdir>/.roko`. Agents must read none of them; [`is_key_file`] is
/// the check.
#[must_use]
pub fn key_file_paths(home: Option<&Path>, workdir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for dir in home.into_iter().chain([workdir]) {
        let roko = dir.join(".roko");
        paths.extend(KEY_FILE_NAMES.iter().map(|name| roko.join(name)));
    }
    paths
}

/// Whether agents must be kept away from `path`: it is one of
/// [`KEY_FILE_NAMES`] in a `.roko` directory, `~/.roko`, the workdir's or
/// any other checkout's.
///
/// The rest of a `.roko` directory, `~/.roko` included, stays readable. When
/// `HOME` is the workdir, as in many containers and CI jobs, `~/.roko` is
/// the workdir's `.roko`, and agents need its plans, state and the plan
/// worktrees under `.roko/worktrees`.
#[must_use]
pub fn is_key_file(path: &Path) -> bool {
    let mut components = path.components().rev();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(name)), Some(Component::Normal(dir)))
            if dir == ".roko" && name.to_str().is_some_and(|file| KEY_FILE_NAMES.contains(&file))
    )
}

/// Whether agents must be kept away from `path`, a roko config file that
/// holds a secret such as `serve.auth.api_key`.
///
/// A secret is a literal in a secret-named field, a provider header or an
/// agent variable ([`crate::config::loader::secret_fields`], through
/// [`config_text_holds_secrets`]). Besides `~/.roko/config.toml`, a key file
/// whatever it holds, roko reads its config from the project `roko.toml` (in
/// the workdir or an ancestor), from the file `ROKO_CONFIG` names, and from
/// the legacy `~/.config/roko/config.toml`. Agents may read those while they
/// hold no secret; a secret kept in `ROKO__*` variables in `.roko/.env` (for
/// example `ROKO__SERVE__AUTH__API_KEY`) leaves them readable. The loader
/// refuses such a file ([`crate::config::LoadConfigError::SecretInConfig`]),
/// so a secret sits there only if it is added while roko runs, and this check
/// covers the file tools, not a command that reads the whole project.
#[must_use]
pub fn is_config_with_secrets(path: &Path) -> bool {
    let roko_config = std::env::var_os("ROKO_CONFIG").map(PathBuf::from);
    config_holds_secrets(path, roko_config.as_deref())
}

/// [`is_config_with_secrets`], with `roko_config` the file `ROKO_CONFIG` names.
fn config_holds_secrets(path: &Path, roko_config: Option<&Path>) -> bool {
    let name = path.file_name().and_then(|name| name.to_str());
    let dir = path.parent().and_then(Path::file_name);
    let is_config = name == Some("roko.toml")
        || (name == Some("config.toml") && dir.is_some_and(|dir| dir == "roko"))
        || roko_config.is_some_and(|config| same_file(config, path));
    // Only a regular file is read: opening a FIFO named roko.toml would block.
    is_config
        && std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
        && std::fs::read_to_string(path).is_ok_and(|text| config_text_holds_secrets(&text))
}

/// Whether `a` and `b` name the same file, as given or resolved.
fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
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

    fn os_parent(items: &[(&str, &str)]) -> Vec<(String, OsString)> {
        items
            .iter()
            .map(|(name, value)| ((*name).to_string(), OsString::from(*value)))
            .collect()
    }

    fn command_env(cmd: &Command) -> Vec<(String, Option<String>)> {
        let mut env: Vec<_> = cmd
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect();
        env.sort();
        env
    }

    fn set(name: &str, value: &str) -> (String, Option<String>) {
        (name.to_string(), Some(value.to_string()))
    }

    #[test]
    fn gate_env_on_a_command_replaces_the_inherited_env() {
        let mut cmd = Command::new("true");
        cmd.env("CARGO_BUILD_JOBS", "2");
        apply_gate_env(
            &mut cmd,
            os_parent(&[
                ("PATH", "/usr/bin:/bin"),
                ("CARGO_HOME", "/home/dev/.cargo"),
                ("OPENAI_API_KEY", "sk-test-not-real"),
                ("MY_SECRET_TOKEN", "tok-test-not-real"),
                ("DATABASE_URL", "postgres://db"),
            ]),
            &patterns(&["DATABASE_URL"]),
        );
        assert_eq!(
            command_env(&cmd),
            vec![
                set("CARGO_BUILD_JOBS", "2"),
                set("CARGO_HOME", "/home/dev/.cargo"),
                set("DATABASE_URL", "postgres://db"),
                set("PATH", "/usr/bin:/bin"),
            ]
        );
    }

    #[test]
    fn credential_scrub_on_a_command_keeps_other_variables_and_explicit_ones() {
        let mut cmd = Command::new("true");
        cmd.env("MCP_SERVER_SETTING", "from-config");
        cmd.env("PERPLEXITY_API_KEY", "named-in-the-config");
        apply_credential_scrub_from(
            &mut cmd,
            &CredentialScrub::default(),
            os_parent(&[
                ("PATH", "/usr/bin:/bin"),
                ("GITHUB_TOKEN", "ghp_test"),
                ("OPENAI_API_KEY", "sk-test-not-real"),
                ("ANTHROPIC_API_KEY", "sk-ant-test-not-real"),
                ("ROKO_SERVE_AUTH_API_KEY", "serve-test-not-real"),
                ("PERPLEXITY_API_KEY", "pplx-inherited"),
            ]),
        );
        // A blocklist: a shell `GITHUB_TOKEN` stays, provider keys and
        // roko's own credentials go, and a value set on the command wins.
        assert_eq!(
            command_env(&cmd),
            vec![
                set("GITHUB_TOKEN", "ghp_test"),
                set("MCP_SERVER_SETTING", "from-config"),
                set("PATH", "/usr/bin:/bin"),
                set("PERPLEXITY_API_KEY", "named-in-the-config"),
            ]
        );
    }

    #[test]
    fn key_files_cover_both_roko_dirs() {
        let home = Path::new("/home/dev");
        let workdir = Path::new("/work/repo");
        let paths = key_file_paths(Some(home), workdir);
        for expected in [
            "/home/dev/.roko/.env",
            "/home/dev/.roko/credentials.json",
            "/work/repo/.roko/.env",
            "/work/repo/.roko/secrets.toml",
        ] {
            assert!(
                paths.contains(&PathBuf::from(expected)),
                "{expected} missing from {paths:?}"
            );
        }
        for path in &paths {
            assert!(is_key_file(path), "{}", path.display());
        }
        assert_eq!(key_file_paths(None, workdir).len(), KEY_FILE_NAMES.len());

        // The key files in any other checkout's .roko, and relative paths.
        for path in ["/elsewhere/repo/.roko/.env", ".roko/secrets.toml"] {
            assert!(is_key_file(Path::new(path)), "{path}");
        }
        // The global config, which can hold serve.auth.api_key.
        assert!(is_key_file(Path::new("/home/dev/.roko/config.toml")));
        // The rest of a .roko directory, ~/.roko's too, and look-alikes.
        for path in [
            "/work/repo/.roko/state/graph/p/checkpoint.json",
            "/home/dev/.roko/logs/daemon.log",
            "/work/repo/.roko/config/config.toml",
            "/work/repo/.env",
            "/work/repo/.roko-old/.env",
            "/work/repo/.roko/.env/nested",
            "/home/dev/.rokorc",
        ] {
            assert!(!is_key_file(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn key_file_policy_when_home_is_workdir() {
        // Containers and CI jobs often run with HOME set to the project, so
        // ~/.roko and the workdir's .roko are one directory. Its key files
        // are refused; its plans, state and plan worktrees are not.
        let workdir = Path::new("/app");
        let paths = key_file_paths(Some(workdir), workdir);
        assert_eq!(paths.len(), 2 * KEY_FILE_NAMES.len());
        for name in KEY_FILE_NAMES {
            let path = workdir.join(".roko").join(name);
            assert!(paths.contains(&path), "{} missing", path.display());
            assert!(is_key_file(&path), "{}", path.display());
        }
        for path in [
            "/app/.roko",
            "/app/.roko/state/graph/p/checkpoint.json",
            "/app/.roko/prd/drafts/x.md",
            "/app/.roko/worktrees/p-t1/src/lib.rs",
            "/app/.roko/worktrees/p-t1/config.toml",
        ] {
            assert!(!is_key_file(Path::new(path)), "{path}");
        }
        // A plan worktree's own .roko is still covered.
        assert!(is_key_file(Path::new(
            "/app/.roko/worktrees/p-t1/.roko/.env"
        )));
    }

    #[test]
    fn no_agent_readable_config_file_holds_the_serve_api_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let write = |path: &Path, text: &str| std::fs::write(path, text).expect("write config");
        let serve_key = "[serve.auth]\nenabled = true\napi_key = \"sk-serve-test\"\n";

        // Every file roko reads its config from is off limits while it
        // holds the key: the project roko.toml, the file ROKO_CONFIG names,
        // the legacy ~/.config/roko/config.toml, and ~/.roko/config.toml,
        // a key file whatever it holds.
        let project = dir.path().join("roko.toml");
        write(&project, serve_key);
        assert!(config_holds_secrets(&project, None));
        let named = dir.path().join("deploy.toml");
        write(&named, serve_key);
        assert!(!config_holds_secrets(&named, None), "not a config file");
        assert!(config_holds_secrets(&named, Some(named.as_path())));
        let legacy = dir.path().join("roko").join("config.toml");
        std::fs::create_dir_all(dir.path().join("roko")).expect("mkdir");
        write(&legacy, serve_key);
        assert!(config_holds_secrets(&legacy, None));
        assert!(is_key_file(&dir.path().join(".roko").join("config.toml")));

        // The other credentials a config can hold, and one that won't parse.
        for text in [
            "[server]\nauth_token = \"t\"\n",
            "[deploy]\nrailway_api_token = \"t\"\n",
            "[webhooks.github]\nsecret = \"s\"\n",
            "[chain]\nwallet_key = \"0xabc\"\n",
            "[platforms.chat]\nkind = \"discord\"\ntoken = \"literal\"\n",
            "[providers.x.extra_headers]\nX-Org = \"org-1\"\n",
            "[agent]\nenv = [[\"OPENAI_API_KEY\", \"sk-x\"]]\n",
            "[serve.auth\napi_key = \"sk-serve-test\"\n",
        ] {
            write(&project, text);
            assert!(config_holds_secrets(&project, None), "{text}");
        }
        // Without a secret, agents keep reading the project config.
        for text in [
            "[serve.auth]\nenabled = true\napi_key = \"\"\n",
            "[providers.x]\napi_key_env = \"X_API_KEY\"\n",
            "[platforms.chat]\nkind = \"discord\"\ntoken = { env = \"DISCORD_TOKEN\" }\n",
            "[agent]\nmax_tokens_per_turn = 4096\n",
            "[providers.x.extra_headers]\nAuthorization = \"Bearer ${X_API_KEY}\"\n",
            "[providers.x.extra_headers]\ntoken_file = \"/run/secrets/x\"\n",
            "[agent]\nenv = [[\"RUST_LOG\", \"debug\"]]\n",
            "",
        ] {
            write(&project, text);
            assert!(!config_holds_secrets(&project, None), "{text}");
        }
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
