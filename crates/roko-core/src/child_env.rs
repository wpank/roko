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
//! - **Provider CLIs** (Claude, Codex, Gemini, Cursor, Hermes, OpenClaw) and
//!   MCP servers keep their inherited environment minus every credential they
//!   do not own: provider keys, names that look like a credential
//!   (`GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`) and names roko loaded from a
//!   `.env` file. See [`CredentialScrub`].
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
//! **Check commands** (every command roko runs as a check: task verify steps
//! and rungs, whole-plan and post-merge regression checks, red-on-base and
//! audit re-runs) get the gate policy plus more, through one builder,
//! [`apply_check_env`] (bug-3c4600). The gate policy keeps `HOME` and `USER`,
//! so `gh` would find the person's login there and act as them: a check gets
//! `GH_CONFIG_DIR` pointing at an empty directory instead. GitHub tokens and
//! the provider keys roko loaded from a `.env` file reach it only when a
//! passthrough list names them exactly, and `[gates] strip_env` removes more.
//! A check also builds in its own checkout (bug-be2a53): an inherited
//! `CARGO_TARGET_DIR` that names a directory outside the command's is left
//! out ([`CARGO_BUILD_DIR_VARS`]), because cargo takes another checkout's
//! build of a crate in a shared target dir for this checkout's whenever this
//! checkout's copy of the crate is older.
//!
//! Keeping keys out of a child's environment is moot while the child can read
//! the files they come from, so this module also lists those files: see
//! [`key_file_paths`] and [`is_key_file`].

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;

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
/// - every name that looks like a credential ([`is_secret_env_name`]), known
///   to roko or not: `GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`,
///   `STRIPE_SECRET_KEY`, and roko's own `ROKO_*` credentials such as a serve
///   admin key, and
/// - every name roko loaded from a `.env` file.
///
/// Two exemptions apply. Names matching a `keep` pattern stay (the
/// provider's `api_key_env`, `[agent] env_passthrough`, variables an MCP
/// config the agent receives refers to). The CLI's own credentials stay when
/// they came from the environment roko was started with rather than from
/// roko's `.env` files: a user's exported `ANTHROPIC_API_KEY` still reaches
/// `claude` exactly as it would without roko, while a key only roko loaded
/// does not.
///
/// A CLI that reads another credential itself, such as Claude Code on
/// Bedrock or Vertex reading `AWS_*` or `GOOGLE_APPLICATION_CREDENTIALS`,
/// gets it when `[agent] env_passthrough` names it.
///
/// The default policy, with no owner, is the one MCP servers get. The names
/// a policy strips are logged at debug, each once per process.
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
        let credential =
            is_provider_key_var(name) || is_secret_env_name(name) || dotenv.is_listed(name);
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

// ---- Gate command limits ----------------------------------------------------

/// How hard a gate command may push the machine: `[gates] cargo_jobs` and
/// `[gates] nice` (bug-b3849c). A run's checks share the machine with the
/// person using it, and a whole-workspace build of a large Cargo workspace
/// otherwise takes every core for tens of minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateLimits {
    /// What `CARGO_BUILD_JOBS` (the jobs a cargo build runs at once) and
    /// `RUST_TEST_THREADS` (the tests a test binary runs at once) say, unless
    /// the command already sets them. `None`: no cap.
    pub cargo_jobs: Option<usize>,
    /// The niceness a gate command runs at: one roko would start less nice
    /// is raised to it. 0 leaves the priority alone.
    pub nice: i32,
}

impl GateLimits {
    /// The default niceness of a gate command.
    pub const DEFAULT_NICE: i32 = 10;

    /// The default job cap: half the CPUs, at least 2.
    #[must_use]
    pub fn default_cargo_jobs() -> usize {
        let cpus = std::thread::available_parallelism().map_or(2, std::num::NonZeroUsize::get);
        (cpus / 2).max(2)
    }
}

impl Default for GateLimits {
    fn default() -> Self {
        Self {
            cargo_jobs: Some(Self::default_cargo_jobs()),
            nice: Self::DEFAULT_NICE,
        }
    }
}

static GATE_LIMITS: Mutex<Option<GateLimits>> = Mutex::new(None);

/// Make `limits` the limits of the gate commands this process starts from
/// now on. A plan run sets its workspace's `[gates]` limits before anything
/// runs.
pub fn set_gate_limits(limits: GateLimits) {
    *GATE_LIMITS.lock() = Some(limits);
}

/// The limits gate commands start with: the ones [`set_gate_limits`] set
/// last, else [`GateLimits::default`].
#[must_use]
pub fn gate_limits() -> GateLimits {
    GATE_LIMITS.lock().unwrap_or_default()
}

/// Give `cmd`, whose environment [`apply_gate_env`] set, the job cap of
/// `limits`: `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS`, each unless `cmd`
/// already sets (or removes) it, so a value in roko's own environment, or
/// one a gate sets, wins. The niceness is the gate runner's to apply: it
/// takes a hook in the child.
pub fn apply_gate_job_cap(cmd: &mut Command, limits: &GateLimits) {
    let Some(jobs) = limits.cargo_jobs else {
        return;
    };
    for name in ["CARGO_BUILD_JOBS", "RUST_TEST_THREADS"] {
        if !cmd.get_envs().any(|(set, _)| set == name) {
            cmd.env(name, jobs.to_string());
        }
    }
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
        let stripped = self.names_to_strip(inherited.iter().map(String::as_str), startup_dotenv());
        log_stripped(stripped.iter().map(String::as_str));
        for name in stripped {
            cmd.env_remove(name);
        }
    }
}

/// Log at debug the inherited variables a [`CredentialScrub`] kept out of a
/// child, each name once per process. Names only, never values.
fn log_stripped<'a>(names: impl IntoIterator<Item = &'a str>) {
    static LOGGED: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
    let fresh: Vec<&str> = {
        let mut logged = LOGGED.get_or_init(|| Mutex::new(BTreeSet::new())).lock();
        names
            .into_iter()
            .filter(|name| logged.insert((*name).to_string()))
            .collect()
    };
    if !fresh.is_empty() {
        tracing::debug!(
            names = %fresh.join(","),
            "inherited credentials kept out of a child process"
        );
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
    let (stripped, kept): (Vec<_>, Vec<_>) = inherited
        .into_iter()
        .partition(|(name, _)| scrub.strips(name, dotenv));
    log_stripped(stripped.iter().map(|(name, _)| name.as_str()));
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

// ---- Check commands --------------------------------------------------------

/// GitHub tokens, which `gh` and other tools read. A check command inherits
/// one only when a passthrough list names it exactly.
pub const GITHUB_TOKEN_VARS: &[&str] = &[
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
];

/// Where `gh` keeps its login (`hosts.yml`; the token itself may sit in the
/// system keychain, which `gh` reads only for a host listed there). A check
/// command gets an empty directory here ([`check_gh_config_dir`]), so `gh`
/// finds no login, unless a passthrough list names `GH_CONFIG_DIR` exactly.
pub const GH_CONFIG_DIR: &str = "GH_CONFIG_DIR";

/// The variables that tell cargo where a build keeps its output and the
/// fingerprints that decide what is fresh: `CARGO_TARGET_DIR`, and the
/// environment forms of the `build.target-dir` and `build.build-dir` config
/// values. A check command inherits one only when it names a directory inside
/// the command's own directory ([`build_dir_is_own`]), or when a passthrough
/// list names it exactly (bug-be2a53).
pub const CARGO_BUILD_DIR_VARS: &[&str] = &[
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR",
];

/// Why a check command does not inherit a variable of roko's environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckEnvRemoval {
    /// `[gates] strip_env` names it.
    Strip,
    /// A GitHub token, or `GH_CONFIG_DIR`.
    GithubCredential,
    /// A provider key roko loaded from a `.env` file.
    DotenvProviderKey,
    /// Another name a loaded `.env` file lists.
    Dotenv,
    /// A name that looks like a credential ([`is_secret_env_name`]).
    Credential,
    /// One of [`CARGO_BUILD_DIR_VARS`] naming a directory outside the
    /// command's own: other checkouts build there too (bug-be2a53).
    SharedBuildDir,
    /// Not on the gate allowlist ([`gate_env_allowlisted`]).
    NotAllowlisted,
}

impl CheckEnvRemoval {
    /// The class, as the run's log names it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Strip => "named in [gates] strip_env",
            Self::GithubCredential => "GitHub credentials",
            Self::DotenvProviderKey => "provider keys roko loaded from .env",
            Self::Dotenv => "other names roko loaded from .env",
            Self::Credential => "names that look like credentials",
            Self::SharedBuildDir => "cargo build dirs outside the check's directory",
            Self::NotAllowlisted => "not on the allowlist",
        }
    }
}

/// Why a check command does not inherit `name`, or `None` when it does.
///
/// `strip` (`[gates] strip_env`) wins over everything. GitHub credentials
/// ([`GITHUB_TOKEN_VARS`], [`GH_CONFIG_DIR`]) and the provider keys roko
/// loaded from a `.env` file are inherited only when `passthrough` names them
/// exactly: a `PREFIX*` pattern does not admit them. Everything else follows
/// the gate policy ([`gate_env_keeps`]).
#[must_use]
pub fn check_env_removal(
    name: &str,
    passthrough: &[String],
    dotenv: &DotenvNames,
    strip: &[String],
) -> Option<CheckEnvRemoval> {
    if matches_any(name, strip) {
        return Some(CheckEnvRemoval::Strip);
    }
    let named = passthrough.iter().any(|pattern| pattern.trim() == name);
    let github = GITHUB_TOKEN_VARS
        .iter()
        .chain([&GH_CONFIG_DIR])
        .any(|var| var.eq_ignore_ascii_case(name));
    if github && !named {
        return Some(CheckEnvRemoval::GithubCredential);
    }
    if is_provider_key_var(name) && dotenv.supplied_value(name) && !named {
        return Some(CheckEnvRemoval::DotenvProviderKey);
    }
    if gate_env_keeps(name, passthrough, dotenv) {
        return None;
    }
    Some(if dotenv.is_listed(name) {
        CheckEnvRemoval::Dotenv
    } else if is_secret_env_name(name) {
        CheckEnvRemoval::Credential
    } else {
        CheckEnvRemoval::NotAllowlisted
    })
}

/// The variables of `parent` a check command inherits, and the names it does
/// not, each with why ([`check_env_removal`]).
pub fn check_env<V>(
    parent: impl IntoIterator<Item = (String, V)>,
    passthrough: &[String],
    dotenv: &DotenvNames,
    strip: &[String],
) -> (Vec<(String, V)>, Vec<(String, CheckEnvRemoval)>) {
    let mut kept = Vec::new();
    let mut removed = Vec::new();
    for (name, value) in parent {
        match check_env_removal(&name, passthrough, dotenv, strip) {
            None => kept.push((name, value)),
            Some(reason) => removed.push((name, reason)),
        }
    }
    (kept, removed)
}

/// Whether `name` is one of [`CARGO_BUILD_DIR_VARS`], in any case on Windows,
/// whose environment names are case-insensitive.
fn is_cargo_build_dir_var(name: &str) -> bool {
    CARGO_BUILD_DIR_VARS.iter().any(|var| {
        if cfg!(windows) {
            var.eq_ignore_ascii_case(name)
        } else {
            *var == name
        }
    })
}

/// Whether `value`, the value of `name`, one of [`CARGO_BUILD_DIR_VARS`],
/// names a directory that belongs to the checkout a command runs in: one
/// inside `dir`, the command's directory, where no other checkout builds
/// (bug-be2a53).
///
/// Cargo names a workspace crate's build alike in every checkout of a
/// repository, and takes it as fresh whenever the checkout's sources are
/// older than it. In a build dir two checkouts share, one checkout's build
/// of a crate then stands in for the other's, and a gate passes code that
/// does not compile.
///
/// A relative value is relative to the command's directory, as cargo takes
/// it, and stays inside unless it climbs out with `..`. An absolute one is
/// compared with both paths resolved as far as they exist, symlinks
/// included. A `CARGO_BUILD_BUILD_DIR` template is the checkout's own when it
/// starts at `{workspace-root}` or holds `{workspace-path-hash}`, a hash of
/// the checkout's manifest path; any other template is not. An empty value
/// names no directory.
#[must_use]
pub fn build_dir_is_own(name: &str, value: &OsStr, dir: &Path) -> bool {
    if value.is_empty() {
        return true;
    }
    if name.eq_ignore_ascii_case("CARGO_BUILD_BUILD_DIR") {
        let Some(text) = value.to_str() else {
            return false;
        };
        if text.contains("{workspace-path-hash}") {
            return true;
        }
        if let Some(rest) = text.strip_prefix("{workspace-root}") {
            let rest = rest.trim_start_matches(['/', '\\']);
            return !rest.contains('{') && stays_inside(Path::new(rest));
        }
        if text.contains('{') {
            return false;
        }
    }
    let path = Path::new(value);
    if path.is_relative() {
        return stays_inside(path);
    }
    match (resolve_existing(dir), resolve_existing(path)) {
        (Some(dir), Some(path)) => path.starts_with(dir),
        _ => false,
    }
}

/// Whether the relative path `path` stays inside the directory it is taken
/// from: no `..`, root or prefix component.
fn stays_inside(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

/// `path` with its symlinks resolved as far as it exists: its longest
/// existing ancestor canonicalized, the rest appended. `None` when no
/// ancestor exists or the part that does not exist climbs with `..`.
fn resolve_existing(path: &Path) -> Option<PathBuf> {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(mut resolved) = std::fs::canonicalize(existing) {
            resolved.extend(rest.iter().rev());
            return Some(resolved);
        }
        rest.push(existing.file_name()?);
        existing = existing.parent()?;
    }
}

/// Move the [`CARGO_BUILD_DIR_VARS`] of `kept` that a command running in
/// `dir` does not inherit to `removed` (bug-be2a53): each one naming a
/// directory outside `dir` ([`build_dir_is_own`]), and each one when `dir` is
/// not known, unless `passthrough` names it exactly. Cargo then builds in
/// the checkout's own `target/`.
fn drop_shared_build_dirs(
    kept: &mut Vec<(String, OsString)>,
    removed: &mut Vec<(String, CheckEnvRemoval)>,
    dir: Option<&Path>,
    passthrough: &[String],
) {
    kept.retain(|(name, value)| {
        let shared = is_cargo_build_dir_var(name)
            && !passthrough.iter().any(|pattern| pattern.trim() == name)
            && !dir.is_some_and(|dir| build_dir_is_own(name, value, dir));
        if shared {
            removed.push((name.clone(), CheckEnvRemoval::SharedBuildDir));
        }
        !shared
    });
}

/// The directory `cmd` runs in, when one was set on it: absolute, taken from
/// roko's own working directory when relative.
fn command_dir(cmd: &Command) -> Option<PathBuf> {
    let dir = cmd.get_current_dir()?;
    if dir.is_absolute() {
        return Some(dir.to_path_buf());
    }
    std::env::current_dir().ok().map(|cwd| cwd.join(dir))
}

/// What `[gates]` says about every check command of a run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckEnvSettings {
    /// `[gates] env_passthrough`: names or `PREFIX*` patterns a check inherits
    /// beyond the allowlist, on top of the ones its caller passes.
    pub passthrough: Vec<String>,
    /// `[gates] strip_env`: names or `PREFIX*` patterns a check never
    /// inherits, even when allowlisted or passed through.
    pub strip: Vec<String>,
}

/// The check settings of the run going on, its empty `GH_CONFIG_DIR`, and
/// whether its checks' environment was logged.
struct CheckEnvState {
    settings: CheckEnvSettings,
    gh_config_dir: Option<PathBuf>,
    logged: bool,
}

static CHECK_ENV: Mutex<CheckEnvState> = Mutex::new(CheckEnvState {
    settings: CheckEnvSettings {
        passthrough: Vec::new(),
        strip: Vec::new(),
    },
    gh_config_dir: None,
    logged: false,
});

/// Start a run's checks. From now on they follow `settings`, the next one
/// logs what its environment leaves out, and they get a fresh empty
/// `GH_CONFIG_DIR`, removed when the returned guard drops at the end of the
/// run. A plan run calls this before anything runs; checks outside a run
/// follow the defaults.
#[must_use = "dropping the guard ends the run's checks"]
pub fn begin_check_run(settings: CheckEnvSettings) -> CheckRun {
    let previous = {
        let mut state = CHECK_ENV.lock();
        state.settings = settings;
        state.logged = false;
        state.gh_config_dir.take()
    };
    remove_if_empty(previous.as_deref());
    CheckRun { _run: () }
}

/// The checks of one run ([`begin_check_run`]). Dropping it removes their
/// empty `GH_CONFIG_DIR`; a check still running elsewhere then finds no
/// directory there, which `gh` reads as no login, and the next check gets a
/// new one.
#[derive(Debug)]
pub struct CheckRun {
    _run: (),
}

impl Drop for CheckRun {
    fn drop(&mut self) {
        let dir = CHECK_ENV.lock().gh_config_dir.take();
        remove_if_empty(dir.as_deref());
    }
}

/// Remove `dir` if it is an empty directory; leave anything else alone.
fn remove_if_empty(dir: Option<&Path>) {
    if let Some(dir) = dir {
        // `remove_dir` refuses a directory that holds anything.
        let _ = std::fs::remove_dir(dir);
    }
}

/// The empty directory check commands get as `GH_CONFIG_DIR`: the run's,
/// made on first use and made again when it is gone or a check left
/// something in it. It is read-only, so a check cannot log `gh` in there.
///
/// # Errors
///
/// When no new directory can be made in the temp directory.
pub fn check_gh_config_dir() -> io::Result<PathBuf> {
    let mut state = CHECK_ENV.lock();
    if let Some(dir) = &state.gh_config_dir
        && is_empty_dir(dir)
    {
        return Ok(dir.clone());
    }
    let dir = make_empty_dir()?;
    state.gh_config_dir = Some(dir.clone());
    Ok(dir)
}

/// Whether `dir` is a directory, not a link to one, with nothing in it.
fn is_empty_dir(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir).is_ok_and(|meta| meta.is_dir())
        && std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}

/// Make a new read-only directory in the temp directory.
fn make_empty_dir() -> io::Result<PathBuf> {
    let parent = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let mut taken = None;
    for attempt in 0..8 {
        let name = format!("roko-check-gh-{}-{stamp:x}-{attempt}", std::process::id());
        let dir = parent.join(name);
        #[cfg(unix)]
        let created = {
            use std::os::unix::fs::DirBuilderExt as _;
            std::fs::DirBuilder::new().mode(0o500).create(&dir)
        };
        #[cfg(not(unix))]
        let created = std::fs::create_dir(&dir);
        match created {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => taken = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(taken.unwrap_or_else(|| io::Error::other("every name was taken")))
}

/// Give `cmd` the environment of a check command (bug-3c4600), in place of
/// the one it would inherit:
///
/// - the variables of `inherited` that [`check_env`] keeps: the gate
///   allowlist plus `passthrough` and the run's `[gates] env_passthrough`,
///   minus `[gates] strip_env`, and minus GitHub tokens and the provider keys
///   roko loaded from a `.env` file unless a passthrough list names them
///   exactly;
/// - `GH_CONFIG_DIR` pointing at an empty directory ([`check_gh_config_dir`]),
///   so `gh` finds no login although the command keeps `HOME` and `USER`,
///   unless a passthrough list names `GH_CONFIG_DIR` exactly;
/// - none of [`CARGO_BUILD_DIR_VARS`] that names a directory outside the one
///   `cmd` runs in ([`build_dir_is_own`]), unless a passthrough list names it
///   exactly, so cargo builds in the checkout's own `target/` and no other
///   checkout's build of a crate passes for this one's (bug-be2a53). Set the
///   command's directory first: without one, each of them is left out;
/// - the variables already set on `cmd`.
///
/// `inherited` is roko's own environment ([`process_env`]) in production;
/// tests pass their own. The first check of a run logs which classes of
/// variables it left out: counts at info, names at debug, never values.
///
/// # Errors
///
/// When the empty `GH_CONFIG_DIR` cannot be made. `cmd` is then left with an
/// empty environment, and the caller must not run it: a check whose
/// environment cannot be set up does not run.
pub fn apply_check_env(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (String, OsString)>,
    passthrough: &[String],
) -> io::Result<()> {
    let settings = CHECK_ENV.lock().settings.clone();
    let mut patterns = passthrough.to_vec();
    patterns.extend(settings.passthrough.iter().cloned());
    let own_gh_config = patterns
        .iter()
        .any(|pattern| pattern.trim() == GH_CONFIG_DIR);
    let gh_config_dir = if own_gh_config {
        None
    } else {
        match check_gh_config_dir() {
            Ok(dir) => Some(dir),
            Err(error) => {
                cmd.env_clear();
                return Err(io::Error::new(
                    error.kind(),
                    format!(
                        "could not make an empty GH_CONFIG_DIR in {}: {error}",
                        std::env::temp_dir().display()
                    ),
                ));
            }
        }
    };
    let removed = apply_check_env_with(
        cmd,
        inherited,
        &patterns,
        startup_dotenv(),
        &settings.strip,
        gh_config_dir.as_deref(),
    );
    log_check_env(&removed, gh_config_dir.as_deref());
    Ok(())
}

/// [`apply_check_env`] with every input given: `gh_config_dir` is the empty
/// directory, or `None` to keep the inherited `GH_CONFIG_DIR` (a passthrough
/// list names it). Returns the names left out, each with why.
fn apply_check_env_with(
    cmd: &mut Command,
    inherited: impl IntoIterator<Item = (String, OsString)>,
    passthrough: &[String],
    dotenv: &DotenvNames,
    strip: &[String],
    gh_config_dir: Option<&Path>,
) -> Vec<(String, CheckEnvRemoval)> {
    let explicit = explicit_env(cmd);
    let (mut kept, mut removed) = check_env(inherited, passthrough, dotenv, strip);
    drop_shared_build_dirs(
        &mut kept,
        &mut removed,
        command_dir(cmd).as_deref(),
        passthrough,
    );
    cmd.env_clear();
    cmd.envs(kept);
    if let Some(dir) = gh_config_dir {
        cmd.env(GH_CONFIG_DIR, dir);
    }
    restore_env(cmd, explicit);
    removed
}

/// Log, once per run ([`begin_check_run`]), which classes of variables check
/// commands leave out and where `gh` looks for a login: counts at info, names
/// at debug, never values.
fn log_check_env(removed: &[(String, CheckEnvRemoval)], gh_config_dir: Option<&Path>) {
    {
        let mut state = CHECK_ENV.lock();
        if state.logged {
            return;
        }
        state.logged = true;
    }
    let mut classes: BTreeMap<CheckEnvRemoval, Vec<&str>> = BTreeMap::new();
    for (name, reason) in removed {
        classes.entry(*reason).or_default().push(name);
    }
    let summary = classes
        .iter()
        .map(|(reason, names)| format!("{} {}", names.len(), reason.label()))
        .collect::<Vec<_>>()
        .join(", ");
    let gh_config = gh_config_dir.map_or_else(
        || "inherited: [gates] env_passthrough names GH_CONFIG_DIR".to_string(),
        |dir| format!("an empty directory, {}", dir.display()),
    );
    tracing::info!(
        left_out = %summary,
        gh_config_dir = %gh_config,
        "check commands run without the person's credentials or a cargo build dir other \
         checkouts share; a project whose own checks need one of these variables gets it by \
         naming it exactly in [gates] env_passthrough"
    );
    for (reason, names) in &classes {
        tracing::debug!(
            class = reason.label(),
            names = %names.join(","),
            "left out of check commands"
        );
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
                "GITHUB_TOKEN",
                "ROKO_SERVE_AUTH_API_KEY"
            ]
        );
    }

    #[test]
    fn credential_scrub_strips_unknown_secret_names() {
        let scrub = CredentialScrub::for_kind(ProviderKind::ClaudeCli);
        let dotenv = DotenvNames::new();
        // Exported secrets whose names roko does not know as provider keys.
        for name in [
            "AWS_SECRET_ACCESS_KEY",
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "STRIPE_SECRET_KEY",
        ] {
            assert!(scrub.strips(name, &dotenv), "{name} should be stripped");
        }
        for name in ["PATH", "HOME"] {
            assert!(!scrub.strips(name, &dotenv), "{name} should be kept");
        }
        // Claude's own key, exported in the shell roko started from.
        assert!(!scrub.strips("ANTHROPIC_API_KEY", &dotenv));
        // A CLI that reads a cloud credential itself gets it through a keep
        // pattern (`[agent] env_passthrough = ["AWS_*"]`).
        let scrub = scrub.keep("AWS_*");
        assert!(!scrub.strips("AWS_SECRET_ACCESS_KEY", &dotenv));
        assert!(scrub.strips("GITHUB_TOKEN", &dotenv));
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
        // Provider keys, roko's own credentials and a shell `GITHUB_TOKEN`
        // go, other variables stay, and a value set on the command wins.
        assert_eq!(
            command_env(&cmd),
            vec![
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

    /// Tests that use the run's check state ([`begin_check_run`]) take this
    /// lock, so that one's run does not remove another's directory.
    static CHECK_STATE: Mutex<()> = Mutex::new(());

    fn removals(removed: &[(String, CheckEnvRemoval)]) -> Vec<(&str, CheckEnvRemoval)> {
        removed
            .iter()
            .map(|(name, reason)| (name.as_str(), *reason))
            .collect()
    }

    /// bug-3c4600: a check keeps `HOME` and `USER`, but no GitHub token, and
    /// no `GH_CONFIG_DIR` of its own, unless a passthrough list names one
    /// exactly: a pattern does not.
    #[test]
    fn check_env_drops_github_credentials_unless_named_exactly() {
        let parent = parent(&[
            ("PATH", "/usr/bin"),
            ("HOME", "/home/dev"),
            ("USER", "dev"),
            ("GH_TOKEN", "gho_test"),
            ("GITHUB_TOKEN", "ghp_test"),
            ("GH_ENTERPRISE_TOKEN", "ghe_test"),
            ("GITHUB_ENTERPRISE_TOKEN", "ghe_test"),
            ("GH_CONFIG_DIR", "/home/dev/.config/gh"),
        ]);
        let wide = patterns(&["GH_*", "GITHUB_*", "*"]);
        let (kept, removed) = check_env(parent.clone(), &wide, &DotenvNames::new(), &[]);
        assert_eq!(kept_names(&kept), vec!["PATH", "HOME", "USER"]);
        assert_eq!(
            removals(&removed),
            [
                "GH_TOKEN",
                "GITHUB_TOKEN",
                "GH_ENTERPRISE_TOKEN",
                "GITHUB_ENTERPRISE_TOKEN",
                "GH_CONFIG_DIR",
            ]
            .map(|name| (name, CheckEnvRemoval::GithubCredential))
        );

        let named = patterns(&["GH_TOKEN", "GH_CONFIG_DIR"]);
        let (kept, _) = check_env(parent, &named, &DotenvNames::new(), &[]);
        assert_eq!(
            kept_names(&kept),
            vec!["PATH", "HOME", "USER", "GH_TOKEN", "GH_CONFIG_DIR"]
        );
    }

    /// The provider keys roko loaded from a `.env` file reach a check only by
    /// exact name; a key the person exported keeps the gate policy's rule.
    #[test]
    fn check_env_drops_provider_keys_roko_loaded_unless_named_exactly() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("OPENAI_API_KEY", true);
        dotenv.insert("CEREBRAS_API_KEY", true);
        dotenv.insert("DATABASE_URL", true);
        let parent = parent(&[
            ("OPENAI_API_KEY", "sk-test-not-real"),
            ("CEREBRAS_API_KEY", "csk-test-not-real"),
            ("DATABASE_URL", "postgres://db"),
            ("ANTHROPIC_API_KEY", "sk-ant-exported-not-real"),
            ("PATH", "/bin"),
        ]);

        let (kept, removed) = check_env(parent.clone(), &[], &dotenv, &[]);
        assert_eq!(kept_names(&kept), vec!["PATH"]);
        assert_eq!(
            removals(&removed),
            vec![
                ("OPENAI_API_KEY", CheckEnvRemoval::DotenvProviderKey),
                ("CEREBRAS_API_KEY", CheckEnvRemoval::DotenvProviderKey),
                ("DATABASE_URL", CheckEnvRemoval::Dotenv),
                ("ANTHROPIC_API_KEY", CheckEnvRemoval::Credential),
            ]
        );

        // A pattern admits other `.env` names and exported keys, as the gate
        // policy does, but not a provider key roko loaded.
        let wide = patterns(&["*"]);
        let (kept, _) = check_env(parent.clone(), &wide, &dotenv, &[]);
        assert_eq!(
            kept_names(&kept),
            vec!["DATABASE_URL", "ANTHROPIC_API_KEY", "PATH"]
        );
        let named = patterns(&["OPENAI_API_KEY"]);
        let (kept, _) = check_env(parent, &named, &dotenv, &[]);
        assert_eq!(kept_names(&kept), vec!["OPENAI_API_KEY", "PATH"]);
    }

    /// `[gates] strip_env` removes allowlisted and passed-through names alike.
    #[test]
    fn strip_env_wins_over_the_allowlist_and_passthrough() {
        let parent = parent(&[
            ("PATH", "/bin"),
            ("RUSTC_WRAPPER", "sccache"),
            ("CARGO_REGISTRIES_CORP_INDEX", "https://corp/index"),
            ("CARGO_HOME", "/home/dev/.cargo"),
            ("DATABASE_URL", "postgres://db"),
        ]);
        let (kept, removed) = check_env(
            parent,
            &patterns(&["DATABASE_URL"]),
            &DotenvNames::new(),
            &patterns(&["RUSTC_WRAPPER", "CARGO_REGISTRIES_*", "DATABASE_URL"]),
        );
        assert_eq!(kept_names(&kept), vec!["PATH", "CARGO_HOME"]);
        assert_eq!(
            removals(&removed),
            vec![
                ("RUSTC_WRAPPER", CheckEnvRemoval::Strip),
                ("CARGO_REGISTRIES_CORP_INDEX", CheckEnvRemoval::Strip),
                ("DATABASE_URL", CheckEnvRemoval::Strip),
            ]
        );
    }

    /// On a command: `GH_CONFIG_DIR` points at the empty directory, no token
    /// or `.env`-loaded key is left, a passed-through name is, and so are the
    /// variables the caller set.
    #[test]
    fn check_env_on_a_command() {
        let mut dotenv = DotenvNames::new();
        dotenv.insert("OPENAI_API_KEY", true);
        let empty = tempfile::tempdir().expect("tempdir");
        let mut cmd = Command::new("true");
        cmd.env("CARGO_TARGET_DIR", "/tmp/target");
        let removed = apply_check_env_with(
            &mut cmd,
            os_parent(&[
                ("PATH", "/usr/bin:/bin"),
                ("HOME", "/home/dev"),
                ("GH_TOKEN", "gho_test"),
                ("GITHUB_TOKEN", "ghp_test"),
                ("GH_CONFIG_DIR", "/home/dev/.config/gh"),
                ("OPENAI_API_KEY", "sk-test-not-real"),
                ("DATABASE_URL", "postgres://db"),
            ]),
            &patterns(&["DATABASE_URL"]),
            &dotenv,
            &[],
            Some(empty.path()),
        );
        assert_eq!(
            command_env(&cmd),
            vec![
                set("CARGO_TARGET_DIR", "/tmp/target"),
                set("DATABASE_URL", "postgres://db"),
                set("GH_CONFIG_DIR", &empty.path().to_string_lossy()),
                set("HOME", "/home/dev"),
                set("PATH", "/usr/bin:/bin"),
            ]
        );
        assert_eq!(removed.len(), 4, "{removed:?}");

        // A passthrough list that names GH_CONFIG_DIR keeps the person's.
        let mut cmd = Command::new("true");
        apply_check_env_with(
            &mut cmd,
            os_parent(&[("GH_CONFIG_DIR", "/home/dev/.config/gh")]),
            &patterns(&["GH_CONFIG_DIR"]),
            &DotenvNames::new(),
            &[],
            None,
        );
        assert_eq!(
            command_env(&cmd),
            vec![set("GH_CONFIG_DIR", "/home/dev/.config/gh")]
        );
    }

    /// bug-be2a53: a cargo build dir is a check's own when it lies inside the
    /// check's directory; not when it lies outside, climbs out of it, leads
    /// out through a symlink, or is a template no checkout owns.
    #[test]
    fn a_build_dir_is_the_checks_own_only_inside_its_directory() {
        let repo = tempfile::tempdir().expect("tempdir");
        let checkout = repo.path().join(".roko").join("worktrees").join("a");
        std::fs::create_dir_all(&checkout).expect("checkout");
        let own = |name: &str, value: &Path| build_dir_is_own(name, value.as_os_str(), &checkout);
        let target = "CARGO_TARGET_DIR";
        assert!(
            !own(target, &repo.path().join("target")),
            "the main checkout's target dir"
        );
        assert!(
            own(target, &checkout.join("target")),
            "its own, not built yet"
        );
        assert!(own(target, Path::new("target")));
        assert!(own(target, Path::new("./build/target")));
        assert!(!own(target, Path::new("../target")));
        assert!(!own(target, Path::new("out/../../target")));
        assert!(!own(target, &checkout.join("..").join("b").join("target")));
        assert!(
            own(target, Path::new("")),
            "an empty value names no directory"
        );
        #[cfg(unix)]
        {
            let shared = repo.path().join("shared");
            std::fs::create_dir(&shared).expect("shared dir");
            std::os::unix::fs::symlink(&shared, checkout.join("link")).expect("a link out");
            assert!(!own(target, &checkout.join("link").join("target")));
            let alias = repo.path().join("alias");
            std::os::unix::fs::symlink(&checkout, &alias).expect("a link to the checkout");
            assert!(
                own(target, &alias.join("target")),
                "the checkout by another path"
            );
        }
        let build = "CARGO_BUILD_BUILD_DIR";
        assert!(own(build, Path::new("{workspace-root}/target/build")));
        assert!(own(
            build,
            Path::new("{cargo-cache-home}/build/{workspace-path-hash}")
        ));
        assert!(!own(build, Path::new("{cargo-cache-home}/build")));
        assert!(!own(build, Path::new("{workspace-root}/../build")));
        assert!(!own(build, &repo.path().join("build")));
    }

    /// bug-be2a53: on a command, a check keeps the cargo build dirs inside
    /// its directory and loses the rest, and all of them when its directory
    /// is not set yet. An exact passthrough name keeps one wherever it
    /// points, a pattern does not, and one the caller sets stays.
    #[test]
    fn check_env_keeps_only_the_build_dirs_of_the_checks_directory() {
        let repo = tempfile::tempdir().expect("tempdir");
        let checkout = repo.path().join("checkout");
        std::fs::create_dir(&checkout).expect("checkout");
        let shared = repo.path().join("target");
        let shared = shared.to_str().expect("a UTF-8 path");
        let inherited = || {
            os_parent(&[
                ("PATH", "/bin"),
                ("CARGO_TARGET_DIR", shared),
                ("CARGO_BUILD_TARGET_DIR", "target"),
                ("CARGO_BUILD_BUILD_DIR", "{cargo-cache-home}/build"),
            ])
        };
        let none = DotenvNames::new();

        let mut cmd = Command::new("true");
        cmd.current_dir(&checkout);
        let removed = apply_check_env_with(&mut cmd, inherited(), &[], &none, &[], None);
        assert_eq!(
            command_env(&cmd),
            vec![set("CARGO_BUILD_TARGET_DIR", "target"), set("PATH", "/bin")]
        );
        assert_eq!(
            removals(&removed),
            vec![
                ("CARGO_TARGET_DIR", CheckEnvRemoval::SharedBuildDir),
                ("CARGO_BUILD_BUILD_DIR", CheckEnvRemoval::SharedBuildDir),
            ]
        );

        let mut unplaced = Command::new("true");
        apply_check_env_with(&mut unplaced, inherited(), &[], &none, &[], None);
        assert_eq!(command_env(&unplaced), vec![set("PATH", "/bin")]);

        let mut cmd = Command::new("true");
        cmd.current_dir(&checkout)
            .env("CARGO_BUILD_BUILD_DIR", "/own/build");
        let named = patterns(&["CARGO_TARGET_DIR", "CARGO_*"]);
        apply_check_env_with(&mut cmd, inherited(), &named, &none, &[], None);
        assert_eq!(
            command_env(&cmd),
            vec![
                set("CARGO_BUILD_BUILD_DIR", "/own/build"),
                set("CARGO_BUILD_TARGET_DIR", "target"),
                set("CARGO_TARGET_DIR", shared),
                set("PATH", "/bin"),
            ]
        );
    }

    /// The directory is empty and read-only; one a check wrote into, or one
    /// that is gone, is replaced; a run's directory goes with the run.
    #[test]
    fn checks_get_an_empty_gh_config_dir_per_run() {
        let _state = CHECK_STATE.lock();
        let run = begin_check_run(CheckEnvSettings::default());
        let dir = check_gh_config_dir().expect("an empty directory");
        assert!(is_empty_dir(&dir), "{}", dir.display());
        assert_eq!(check_gh_config_dir().expect("the same directory"), dir);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&dir)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o222, 0, "writable: {mode:o}");
            // A check that makes it writable and logs in there.
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        }
        std::fs::write(dir.join("hosts.yml"), "github.com:\n").expect("a login");
        let fresh = check_gh_config_dir().expect("a new directory");
        assert_ne!(fresh, dir);
        assert!(is_empty_dir(&fresh));
        std::fs::remove_dir_all(&dir).expect("clean up the written one");

        drop(run);
        assert!(!fresh.exists(), "the run's directory stays after the run");
        let next = begin_check_run(CheckEnvSettings::default());
        let after = check_gh_config_dir().expect("a directory for the next run");
        assert!(is_empty_dir(&after));
        drop(next);
        assert!(!after.exists());
    }

    /// The first check of a run logs what it left out; later ones do not,
    /// until the next run begins.
    #[test]
    fn a_run_logs_its_check_environment_once() {
        let _state = CHECK_STATE.lock();
        let run = begin_check_run(CheckEnvSettings {
            passthrough: patterns(&["DATABASE_URL"]),
            strip: patterns(&["RUSTC_WRAPPER"]),
        });
        let mut cmd = Command::new("true");
        apply_check_env(
            &mut cmd,
            os_parent(&[
                ("PATH", "/bin"),
                ("DATABASE_URL", "postgres://db"),
                ("RUSTC_WRAPPER", "sccache"),
                ("GITHUB_TOKEN", "ghp_test"),
            ]),
            &[],
        )
        .expect("the check environment");
        // The run's settings apply without the caller passing them.
        let env = command_env(&cmd);
        let names: Vec<&str> = env.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["DATABASE_URL", "GH_CONFIG_DIR", "PATH"]);
        assert!(CHECK_ENV.lock().logged, "the first check logs");
        drop(run);
        let next = begin_check_run(CheckEnvSettings::default());
        assert!(!CHECK_ENV.lock().logged, "a new run logs again");
        drop(next);
    }

    /// Where [`a_check_without_its_environment_does_not_run`] tells its child
    /// test process to look.
    const FAIL_CLOSED_CHILD: &str = "ROKO_CHECK_ENV_FAIL_CLOSED_CHILD";

    /// A check whose empty `GH_CONFIG_DIR` cannot be made gets an error and
    /// an empty environment: it must not run. The temp directory is set for
    /// a child test process, away from other tests.
    #[test]
    fn a_check_without_its_environment_does_not_run() {
        let missing = tempfile::tempdir().expect("tempdir").path().join("gone");
        let output = Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "child_env::tests::check_env_fail_closed_child",
                "--nocapture",
            ])
            .env(FAIL_CLOSED_CHILD, "1")
            .env("TMPDIR", &missing)
            .env("TMP", &missing)
            .env("TEMP", &missing)
            .output()
            .expect("run the child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains("1 passed"),
            "the child test did not run: {stdout}"
        );
    }

    /// Run by [`a_check_without_its_environment_does_not_run`] only.
    #[test]
    fn check_env_fail_closed_child() {
        if std::env::var_os(FAIL_CLOSED_CHILD).is_none() {
            return;
        }
        let mut cmd = Command::new("true");
        cmd.env("CARGO_TARGET_DIR", "/tmp/target");
        let error = apply_check_env(&mut cmd, os_parent(&[("PATH", "/bin")]), &[])
            .expect_err("no temp directory, no check environment");
        assert!(error.to_string().contains("GH_CONFIG_DIR"), "{error}");
        assert_eq!(command_env(&cmd), Vec::new(), "the environment was cleared");
    }
}
