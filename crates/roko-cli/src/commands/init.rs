//! `roko init` template rendering.
//!
//! The model ladder `roko init` writes climbs only models whose provider this
//! machine reaches, an API key or an agent CLI (bug-4e336b), and its gate
//! rungs check the project it finds in the directory (gap-1625c0).

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use roko_cli::config::command_on_path;
use roko_core::config::routing::{LadderRung, LadderStart};
use roko_core::config::schema::RokoConfig;
use roko_core::config::{GateRungConfig, TimeoutConfig};
use roko_core::{Language, detect_from_files};

/// Environment variable that holds the Anthropic API key.
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";

/// `agent.default_model` of a workspace whose ladder has no rung.
const UNCONFIGURED_DEFAULT_MODEL: &str = "claude-sonnet-4-6";

/// The Claude provider a machine has: the shorthand [`write_init_config`]
/// takes for the machine it renders for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitProvider {
    /// `claude` is on `PATH`.
    ClaudeCli,
    /// `claude` is missing but `ANTHROPIC_API_KEY` is set.
    AnthropicApi,
    /// Neither is available.
    Unconfigured,
}

/// What this machine runs models on: the API keys set and the agent CLIs on
/// `PATH`. roko loads `~/.roko/.env` before any command runs, so a key kept
/// there counts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InitMachine {
    keys: BTreeSet<String>,
    commands: BTreeSet<String>,
}

impl InitMachine {
    /// The keys and CLIs of the providers `roko init` knows, as set now.
    #[must_use]
    pub fn detect() -> Self {
        let mut machine = Self::default();
        for provider in PROVIDERS {
            match provider.access {
                Access::Key(name) => {
                    if std::env::var(name).is_ok_and(|value| !value.trim().is_empty()) {
                        machine.keys.insert(name.to_string());
                    }
                }
                Access::Command(command) => {
                    if command_on_path(command) {
                        machine.commands.insert(command.to_string());
                    }
                }
            }
        }
        machine
    }

    /// This machine with the API key variable `name` set.
    #[must_use]
    pub fn with_key(mut self, name: &str) -> Self {
        self.keys.insert(name.to_string());
        self
    }

    /// This machine with `command` on `PATH`.
    #[must_use]
    pub fn with_command(mut self, command: &str) -> Self {
        self.commands.insert(command.to_string());
        self
    }

    fn reaches(&self, provider: &ProviderSpec) -> bool {
        match provider.access {
            Access::Key(name) => self.keys.contains(name),
            Access::Command(command) => self.commands.contains(command),
        }
    }
}

impl From<InitProvider> for InitMachine {
    fn from(provider: InitProvider) -> Self {
        match provider {
            InitProvider::ClaudeCli => Self::default().with_command("claude"),
            InitProvider::AnthropicApi => Self::default().with_key(ANTHROPIC_API_KEY_ENV),
            InitProvider::Unconfigured => Self::default(),
        }
    }
}

// ─── Providers and models ─────────────────────────────────────────────────

/// How roko reaches a provider.
#[derive(Clone, Copy, Debug)]
enum Access {
    /// An API, called with the key this variable holds.
    Key(&'static str),
    /// An agent CLI, run as this command.
    Command(&'static str),
}

/// A provider `roko init` configures when this machine reaches it.
#[derive(Debug)]
struct ProviderSpec {
    /// Its `[providers.<id>]` key.
    id: &'static str,
    kind: &'static str,
    base_url: Option<&'static str>,
    access: Access,
}

impl ProviderSpec {
    /// Whether roko's own tool loop runs its models, as a chat host's run
    /// needs: only that loop can hold an outbound effect for approval.
    const fn is_api(&self) -> bool {
        matches!(self.access, Access::Key(_))
    }

    /// What makes this machine reach it, for a hint.
    fn needs(&self) -> String {
        match self.access {
            Access::Key(name) => name.to_string(),
            Access::Command(command) => format!("`{command}` on PATH"),
        }
    }
}

const CLAUDE_CLI: ProviderSpec = ProviderSpec {
    id: "claude_cli",
    kind: "claude_cli",
    base_url: None,
    access: Access::Command("claude"),
};

const ANTHROPIC: ProviderSpec = ProviderSpec {
    id: "anthropic",
    kind: "anthropic_api",
    base_url: Some("https://api.anthropic.com"),
    access: Access::Key(ANTHROPIC_API_KEY_ENV),
};

const CEREBRAS: ProviderSpec = ProviderSpec {
    id: "cerebras",
    kind: "openai_compat",
    base_url: Some("https://api.cerebras.ai/v1"),
    access: Access::Key("CEREBRAS_API_KEY"),
};

const ZAI: ProviderSpec = ProviderSpec {
    id: "zai",
    kind: "openai_compat",
    base_url: Some("https://api.z.ai/api/paas/v4"),
    access: Access::Key("ZAI_API_KEY"),
};

const MOONSHOT: ProviderSpec = ProviderSpec {
    id: "moonshot",
    kind: "openai_compat",
    base_url: Some("https://api.moonshot.ai/v1"),
    access: Access::Key("MOONSHOT_API_KEY"),
};

const OPENAI: ProviderSpec = ProviderSpec {
    id: "openai",
    kind: "openai_compat",
    base_url: Some("https://api.openai.com/v1"),
    access: Access::Key("OPENAI_API_KEY"),
};

const OPENROUTER: ProviderSpec = ProviderSpec {
    id: "openrouter",
    kind: "openai_compat",
    base_url: Some("https://openrouter.ai/api/v1"),
    access: Access::Key("OPENROUTER_API_KEY"),
};

/// The providers `roko init` configures when this machine reaches them.
const PROVIDERS: &[&ProviderSpec] = &[
    &CLAUDE_CLI,
    &ANTHROPIC,
    &CEREBRAS,
    &ZAI,
    &MOONSHOT,
    &OPENAI,
    &OPENROUTER,
];

/// A model `roko init` can put on a ladder rung.
#[derive(Debug)]
struct ModelSpec {
    /// Its `[models.<key>]` key, which the rung names.
    key: &'static str,
    provider: &'static ProviderSpec,
    slug: &'static str,
    /// The rung it takes.
    rung: &'static str,
    /// The other lines of its table.
    fields: &'static [&'static str],
}

/// D11's rung names, cheapest first.
const RUNGS: [&str; 4] = ["cheap", "mid", "strong", "top"];

const CLAUDE_FIELDS: &[&str] = &[
    "context_window = 200000",
    "tool_format = \"anthropic_blocks\"",
    "max_tools = 32",
];

/// The models each rung can take, most preferred first: DECISIONS D11's
/// cascade (gpt-oss-120b, GLM-4.7, gpt-5.4-mini, then Sonnet) from its own
/// provider, else the same or a close model from another. Rates are the
/// 2026-09-28 price snapshot's (`config/prices/2026-09-28.toml`), or
/// OpenRouter's model list on 2026-10-15; Claude's are built in.
const MODELS: &[ModelSpec] = &[
    ModelSpec {
        key: "gpt-oss-120b",
        provider: &CEREBRAS,
        slug: "gpt-oss-120b",
        rung: "cheap",
        fields: &[
            "context_window = 128000",
            "max_output = 8192",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.35",
            "cost_output_per_m = 0.75",
            "cost_cache_read_per_m = 0.35",
        ],
    },
    ModelSpec {
        key: "gpt-oss-120b-openrouter",
        provider: &OPENROUTER,
        slug: "openai/gpt-oss-120b",
        rung: "cheap",
        fields: &[
            "context_window = 128000",
            "max_output = 8192",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.15",
            "cost_output_per_m = 0.6",
        ],
    },
    ModelSpec {
        key: "glm-4-7",
        provider: &ZAI,
        slug: "glm-4.7",
        rung: "mid",
        fields: &[
            "context_window = 128000",
            "max_output = 16384",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.6",
            "cost_output_per_m = 2.2",
            "cost_cache_read_per_m = 0.11",
        ],
    },
    ModelSpec {
        key: "kimi-k2-6",
        provider: &MOONSHOT,
        slug: "kimi-k2.6",
        rung: "mid",
        fields: &[
            "context_window = 256000",
            "max_output = 65535",
            "supports_thinking = true",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.95",
            "cost_output_per_m = 4.0",
            "cost_cache_read_per_m = 0.16",
        ],
    },
    ModelSpec {
        key: "glm-4-7-openrouter",
        provider: &OPENROUTER,
        slug: "z-ai/glm-4.7",
        rung: "mid",
        fields: &[
            "context_window = 128000",
            "max_output = 16384",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.6",
            "cost_output_per_m = 2.2",
        ],
    },
    ModelSpec {
        key: "gpt-5-4-mini",
        provider: &OPENAI,
        slug: "gpt-5.4-mini",
        rung: "strong",
        fields: &[
            "context_window = 200000",
            "max_output = 16384",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.75",
            "cost_output_per_m = 4.5",
            "cost_cache_read_per_m = 0.075",
        ],
    },
    ModelSpec {
        key: "gpt-5-4-mini-openrouter",
        provider: &OPENROUTER,
        slug: "openai/gpt-5.4-mini",
        rung: "strong",
        fields: &[
            "context_window = 200000",
            "max_output = 16384",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 0.75",
            "cost_output_per_m = 4.5",
        ],
    },
    ModelSpec {
        key: "claude-haiku-4-5",
        provider: &ANTHROPIC,
        slug: "claude-haiku-4-5",
        rung: "strong",
        fields: CLAUDE_FIELDS,
    },
    ModelSpec {
        key: "claude-sonnet-4-6",
        provider: &CLAUDE_CLI,
        slug: "claude-sonnet-4-6",
        rung: "top",
        fields: CLAUDE_FIELDS,
    },
    ModelSpec {
        key: "claude-sonnet-4-6",
        provider: &ANTHROPIC,
        slug: "claude-sonnet-4-6",
        rung: "top",
        fields: CLAUDE_FIELDS,
    },
    ModelSpec {
        key: "claude-sonnet-4-6-openrouter",
        provider: &OPENROUTER,
        slug: "anthropic/claude-sonnet-4.6",
        rung: "top",
        fields: &[
            "context_window = 200000",
            "tool_format = \"openai_json\"",
            "cost_input_per_m = 3.0",
            "cost_output_per_m = 15.0",
        ],
    },
];

/// The models `roko init` writes and the ladder that climbs them.
#[derive(Clone, Debug, Default)]
pub struct InitModels {
    providers: Vec<&'static ProviderSpec>,
    models: Vec<&'static ModelSpec>,
    /// The ladder's rungs, cheapest first.
    pub rungs: Vec<LadderRung>,
    /// `agent.default_model`.
    pub default_model: String,
    /// What the person should know about the ladder.
    pub notes: Vec<String>,
}

impl InitModels {
    /// The ladder for `machine`, or the one model `pin` (`--model`) names.
    /// A pinned model is looked up among the models `roko init` knows, by
    /// key or slug, then among `global`'s (`~/.roko/config.toml`).
    pub fn plan(
        machine: &InitMachine,
        pin: Option<&str>,
        global: Option<&RokoConfig>,
    ) -> Result<Self> {
        let mut plan = match pin.map(str::trim).filter(|pin| !pin.is_empty()) {
            Some(pin) => Self::pinned(machine, pin, global)?,
            None => Self::ladder(machine),
        };
        plan.default_model = plan.rungs.last().map_or_else(
            || UNCONFIGURED_DEFAULT_MODEL.to_string(),
            |rung| rung.model.clone(),
        );
        Ok(plan)
    }

    /// Each D11 rung this machine can fill, with the first model it reaches.
    fn ladder(machine: &InitMachine) -> Self {
        let mut plan = Self::default();
        let mut missing = Vec::new();
        for rung in RUNGS {
            let candidates = MODELS.iter().filter(|model| model.rung == rung);
            match candidates
                .clone()
                .find(|model| machine.reaches(model.provider))
            {
                Some(model) => plan.add(rung, model),
                None => {
                    let mut needs: Vec<String> =
                        candidates.map(|model| model.provider.needs()).collect();
                    needs.dedup();
                    missing.push(format!("{rung} needs {}", needs.join(" or ")));
                }
            }
        }
        if !missing.is_empty() {
            plan.notes.push(format!(
                "rungs left off, as no provider reaches them: {}",
                missing.join("; ")
            ));
        }
        if !plan.providers.iter().any(|provider| provider.is_api()) {
            plan.notes.push(
                "no API model on the ladder: a run a chat host such as Hermes starts needs one, \
                 because only roko's own tool loop can hold its outbound effects for approval. \
                 Put an API key in ~/.roko/.env (OPENAI_API_KEY, OPENROUTER_API_KEY, \
                 ANTHROPIC_API_KEY, CEREBRAS_API_KEY, ZAI_API_KEY or MOONSHOT_API_KEY; `roko \
                 setup` creates the file) before running roko init"
                    .to_string(),
            );
        }
        plan
    }

    /// One rung, `top`, on the model `pin` names.
    fn pinned(machine: &InitMachine, pin: &str, global: Option<&RokoConfig>) -> Result<Self> {
        let mut plan = Self::default();
        let known: Vec<&'static ModelSpec> = MODELS
            .iter()
            .filter(|model| model.key == pin || model.slug == pin)
            .collect();
        let reached = known
            .iter()
            .copied()
            .find(|model| machine.reaches(model.provider));
        if let Some(model) = reached.or_else(|| known.first().copied()) {
            plan.add("top", model);
            let provider = model.provider;
            if !machine.reaches(provider) {
                plan.notes.push(format!(
                    "{} runs on `{}`, which needs {}: it cannot run until that is set",
                    model.key,
                    provider.id,
                    provider.needs()
                ));
            } else if !provider.is_api() {
                plan.notes.push(format!(
                    "{} runs on `{}`, an agent CLI: a run a chat host such as Hermes starts \
                     needs an API model",
                    model.key, provider.id
                ));
            }
        } else if let Some(key) = global.and_then(|global| global_model_key(global, pin)) {
            plan.notes.push(format!(
                "{key} is defined in ~/.roko/config.toml, so this workspace depends on that file"
            ));
            plan.rungs.push(LadderRung {
                name: "top".to_string(),
                model: key,
            });
        } else {
            let known: BTreeSet<&str> = MODELS.iter().map(|model| model.key).collect();
            bail!(
                "--model {pin}: no model roko init knows, and none in ~/.roko/config.toml, has \
                 that key or slug; known models: {}",
                known.into_iter().collect::<Vec<_>>().join(", ")
            );
        }
        plan.notes.push(format!(
            "--model (or ROKO_MODEL) pins every task to {}: the ladder has that one rung",
            plan.rungs[0].model
        ));
        Ok(plan)
    }

    fn add(&mut self, rung: &str, model: &'static ModelSpec) {
        let provider = model.provider;
        if !self.providers.iter().any(|known| known.id == provider.id) {
            self.providers.push(provider);
        }
        self.models.push(model);
        self.rungs.push(LadderRung {
            name: rung.to_string(),
            model: model.key.to_string(),
        });
    }

    /// The start rung of each tier: D11's, else the nearest rung above it,
    /// else the highest rung, as the ladder itself resolves a rung that
    /// cannot run; named, so the loader has no missing rung to warn about.
    fn start(&self) -> LadderStart {
        let present: Vec<usize> = self
            .rungs
            .iter()
            .filter_map(|rung| RUNGS.iter().position(|name| *name == rung.name))
            .collect();
        let pick = |name: Option<String>| {
            let wanted = RUNGS
                .iter()
                .position(|rung| Some(*rung) == name.as_deref())
                .unwrap_or(0);
            present
                .iter()
                .copied()
                .find(|&index| index >= wanted)
                .or_else(|| present.last().copied())
                .map(|index| RUNGS[index].to_string())
        };
        let d11 = LadderStart::d11();
        LadderStart {
            mechanical: pick(d11.mechanical),
            focused: pick(d11.focused),
            integrative: pick(d11.integrative),
            architectural: pick(d11.architectural),
        }
    }

    /// One line per rung, `cheap: gpt-oss-120b (cerebras)`.
    #[must_use]
    pub fn describe_rungs(&self) -> Vec<String> {
        self.rungs
            .iter()
            .map(|rung| {
                let provider = self
                    .models
                    .iter()
                    .find(|model| model.key == rung.model)
                    .map_or("~/.roko/config.toml", |model| model.provider.id);
                format!("{}: {} ({provider})", rung.name, rung.model)
            })
            .collect()
    }
}

/// The key of the model `global` defines under the key or slug `pin`.
fn global_model_key(global: &RokoConfig, pin: &str) -> Option<String> {
    global
        .models
        .iter()
        .find(|(key, model)| key.as_str() == pin || model.slug == pin)
        .map(|(key, _)| key.clone())
}

// ─── Project detection and gate rungs ─────────────────────────────────────

/// A kind of project `roko init` writes gate rungs for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InitProfile {
    /// Cargo.
    Rust,
    /// npm, pnpm, yarn or bun.
    TypeScript,
    /// The Go toolchain.
    Go,
    /// Python, with pytest or unittest.
    Python,
    /// Foundry.
    Solidity,
    /// Nothing `roko init` knows: no rungs.
    #[default]
    General,
}

impl InitProfile {
    /// The profiles `--profile` takes, by name.
    pub const NAMES: &'static str = "rust, typescript, go, python, solidity, general";

    /// The profile's name, as `--profile` takes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Python => "python",
            Self::Solidity => "solidity",
            Self::General => "general",
        }
    }

    /// The profile `--profile <name>` names.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "rust" | "cargo" => Self::Rust,
            "typescript" | "ts" | "javascript" | "js" | "node" | "nodejs" => Self::TypeScript,
            "go" | "golang" => Self::Go,
            "python" | "py" => Self::Python,
            "solidity" | "foundry" | "forge" => Self::Solidity,
            "general" | "none" | "blank" => Self::General,
            _ => return None,
        })
    }

    /// The profile of the project `scan` read: its manifest, by roko-core's
    /// detector, else Python for Python sources or tests with no manifest.
    fn detect(scan: &ProjectScan) -> Self {
        let names: Vec<&str> = scan.names.iter().map(String::as_str).collect();
        match detect_from_files(&names).language {
            Language::Rust => Self::Rust,
            Language::TypeScript => Self::TypeScript,
            Language::Go => Self::Go,
            Language::Python => Self::Python,
            Language::Solidity => Self::Solidity,
            _ if scan.looks_like_python() => Self::Python,
            _ => Self::General,
        }
    }
}

/// Most test files `roko init` reads to tell pytest tests from unittest ones.
const MAX_TEST_FILES: usize = 200;

/// Most bytes of a test file it reads.
const TEST_FILE_PREFIX: u64 = 64 * 1024;

/// Directories never searched for Python files.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "venv",
    "env",
    "site-packages",
    "__pycache__",
    "build",
    "dist",
    "target",
];

/// What `roko init` read of a project.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
struct ProjectScan {
    /// Names of the files and directories at its root.
    names: BTreeSet<String>,
    /// Whether the root has Python sources: a `*.py` file or a package.
    python_sources: bool,
    /// The directory its Python tests are in, `tests`, `test` or `.`.
    python_test_dir: Option<String>,
    /// Python test files that need pytest: they import it, use fixtures from
    /// a `conftest.py`, or define test functions outside any unittest case.
    pytest_tests: usize,
    /// Whether the project configures mypy.
    mypy_configured: bool,
    /// `package.json`'s `scripts`, and whether it has dependencies.
    npm_scripts: BTreeMap<String, String>,
    npm_dependencies: bool,
    /// Whether `node_modules` holds TypeScript's `tsc`.
    typescript_installed: bool,
}

impl ProjectScan {
    fn read(root: &Path) -> Self {
        let mut scan = Self::default();
        for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let path = entry.path();
            // A `*.py` file, or a package directory.
            if (name.ends_with(".py") && path.is_file())
                || (path.join("__init__.py").is_file() && !name.starts_with('.'))
            {
                scan.python_sources = true;
            }
            scan.names.insert(name);
        }
        scan.read_python_tests(root);
        scan.mypy_configured = read_text(&root.join("pyproject.toml")).contains("[tool.mypy")
            || read_text(&root.join("setup.cfg")).contains("[mypy")
            || scan.names.contains("mypy.ini")
            || scan.names.contains(".mypy.ini");
        scan.typescript_installed = root.join("node_modules/.bin/tsc").is_file();
        if let Ok(text) = std::fs::read_to_string(root.join("package.json"))
            && let Ok(package) = serde_json::from_str::<serde_json::Value>(&text)
        {
            if let Some(scripts) = package.get("scripts").and_then(|value| value.as_object()) {
                for (name, command) in scripts {
                    if let Some(command) = command.as_str() {
                        scan.npm_scripts.insert(name.clone(), command.to_string());
                    }
                }
            }
            scan.npm_dependencies = ["dependencies", "devDependencies"].iter().any(|key| {
                package
                    .get(*key)
                    .and_then(|value| value.as_object())
                    .is_some_and(|deps| !deps.is_empty())
            });
        }
        scan
    }

    /// Find the Python test files, in `tests/` or `test/`, else at the root,
    /// and tell pytest tests from unittest ones by what they use.
    fn read_python_tests(&mut self, root: &Path) {
        let conftest = root.join("conftest.py").is_file();
        for dir in ["tests", "test", "."] {
            let mut files = Vec::new();
            if dir == "." {
                for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
                    let path = entry.path();
                    if path.is_file() && is_python_test_file(&path) {
                        files.push(path);
                    }
                }
            } else if root.join(dir).is_dir() {
                collect_python_tests(&root.join(dir), 0, &mut files);
            }
            if files.is_empty() {
                continue;
            }
            files.sort();
            files.truncate(MAX_TEST_FILES);
            let conftest = conftest || root.join(dir).join("conftest.py").is_file();
            self.pytest_tests = files
                .iter()
                .filter(|file| test_file_needs_pytest(&read_prefix(file), conftest))
                .count();
            self.python_test_dir = Some(dir.to_string());
            return;
        }
    }

    /// Whether a project with no manifest roko-core knows is Python's:
    /// packaging files, sources or tests.
    fn looks_like_python(&self) -> bool {
        ["requirements.txt", "setup.cfg", "Pipfile", "tox.ini"]
            .iter()
            .any(|name| self.names.contains(*name))
            || self.python_sources
            || self.python_test_dir.is_some()
    }
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn read_prefix(path: &Path) -> String {
    use std::io::Read as _;
    let mut text = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(TEST_FILE_PREFIX).read_to_end(&mut text);
    }
    String::from_utf8_lossy(&text).into_owned()
}

/// `test_*.py` or `*_test.py`, the files pytest collects.
fn is_python_test_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name.ends_with(".py") && (name.starts_with("test_") || name.ends_with("_test.py"))
}

fn collect_python_tests(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    if depth > 4 || files.len() >= MAX_TEST_FILES {
        return;
    }
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && !SKIPPED_DIRS.contains(&name.as_ref()) {
                collect_python_tests(&path, depth + 1, files);
            }
        } else if is_python_test_file(&path) {
            files.push(path);
        }
    }
}

/// Whether a test file needs pytest to run: it imports pytest, a
/// `conftest.py` beside it supplies fixtures, or it defines test functions
/// and no unittest case.
fn test_file_needs_pytest(text: &str, conftest: bool) -> bool {
    let imports_pytest = text
        .lines()
        .map(str::trim_start)
        .any(|line| line.starts_with("import pytest") || line.starts_with("from pytest"));
    let unittest = text.contains("unittest");
    let test_functions = text.lines().any(|line| line.starts_with("def test"));
    imports_pytest || (conftest && !unittest) || (test_functions && !unittest)
}

/// The tools this machine has that a profile's checks run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toolbox {
    commands: BTreeSet<String>,
    python_modules: BTreeSet<String>,
}

impl Toolbox {
    /// The commands `profile`'s checks run that are on `PATH`, and for
    /// Python the modules `python3` imports, asked in `root` as a rung runs.
    #[must_use]
    pub fn detect(root: &Path, profile: InitProfile) -> Self {
        let candidates: &[&str] = match profile {
            InitProfile::Rust => &["cargo"],
            InitProfile::TypeScript => &["npm", "pnpm", "yarn", "bun", "npx"],
            InitProfile::Go => &["go"],
            InitProfile::Python => &["python3", "ruff", "mypy"],
            InitProfile::Solidity => &["forge"],
            InitProfile::General => &[],
        };
        let mut tools = Self::default();
        for command in candidates {
            if command_on_path(command) {
                tools.commands.insert((*command).to_string());
            }
        }
        if profile == InitProfile::Python && tools.has("python3") {
            tools.python_modules = python_modules(root, &["pytest", "ruff", "mypy"]);
        }
        tools
    }

    /// This toolbox with `command` on `PATH`.
    #[must_use]
    pub fn with_command(mut self, command: &str) -> Self {
        self.commands.insert(command.to_string());
        self
    }

    /// This toolbox with `python3` importing `module`.
    #[must_use]
    pub fn with_python_module(mut self, module: &str) -> Self {
        self.python_modules.insert(module.to_string());
        self
    }

    fn has(&self, command: &str) -> bool {
        self.commands.contains(command)
    }
}

/// The modules among `modules` that `python3` finds from `root`, without
/// importing them. The probe inherits no provider key.
fn python_modules(root: &Path, modules: &[&str]) -> BTreeSet<String> {
    let script = format!(
        "import importlib.util as util\nfor name in {modules:?}:\n    if util.find_spec(name):\n        \
         print(name)"
    );
    let mut probe = std::process::Command::new("python3");
    probe
        .arg("-c")
        .arg(script)
        .current_dir(root)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    roko_core::child_env::CredentialScrub::default().apply(&mut probe);
    probe
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| modules.contains(line))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The project `roko init` found and the gate rungs it writes for it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectGates {
    /// The project's profile.
    pub profile: InitProfile,
    /// Whether `--profile` named it rather than the files found.
    pub chosen: bool,
    /// The rungs `roko init` writes, in the order they run.
    pub rungs: Vec<GateRungConfig>,
    /// What the person should know: a check left out and why.
    pub notes: Vec<String>,
}

impl ProjectGates {
    /// The rungs for the project at `root`: of `profile` (`--profile`) when
    /// it names one, else of the kind its files show. Each rung runs a tool
    /// this machine has, and none is left that would fail the unchanged
    /// project for a reason `roko init` can see, such as a test runner that
    /// is not installed.
    #[must_use]
    pub fn detect(root: &Path, profile: Option<&str>) -> Self {
        let scan = ProjectScan::read(root);
        let name = profile.map(str::trim).filter(|name| !name.is_empty());
        let chosen = name.and_then(InitProfile::parse);
        let profile = chosen.unwrap_or_else(|| InitProfile::detect(&scan));
        let tools = Toolbox::detect(root, profile);
        let mut gates = Self::for_profile(profile, &scan, &tools);
        gates.chosen = chosen.is_some();
        if chosen == Some(InitProfile::General) {
            gates.notes = vec!["--profile general writes no gate rungs".to_string()];
        }
        if let Some(name) = name.filter(|_| chosen.is_none()) {
            gates.notes.insert(
                0,
                format!(
                    "--profile {name} is not one roko init knows ({}), so it used the profile \
                     the project's files show, {}",
                    InitProfile::NAMES,
                    profile.name()
                ),
            );
        }
        gates
    }

    /// The rungs of `profile` for the project `scan` read, with `tools`.
    fn for_profile(profile: InitProfile, scan: &ProjectScan, tools: &Toolbox) -> Self {
        let mut gates = Self {
            profile,
            ..Self::default()
        };
        match profile {
            InitProfile::Rust => gates.toolchain(
                tools,
                "cargo",
                &[
                    ("compile", "cargo check --workspace"),
                    ("test", "cargo test --workspace"),
                ],
                "cargo clippy --workspace --no-deps -- -D warnings",
            ),
            InitProfile::Go => gates.toolchain(
                tools,
                "go",
                &[("compile", "go build ./..."), ("test", "go test ./...")],
                "go vet ./...",
            ),
            InitProfile::Solidity => gates.toolchain(
                tools,
                "forge",
                &[("compile", "forge build"), ("test", "forge test")],
                "forge fmt --check",
            ),
            InitProfile::TypeScript => gates.node(scan, tools),
            InitProfile::Python => gates.python(scan, tools),
            InitProfile::General => gates.notes.push(
                "no project roko init knows here (Cargo.toml, package.json, go.mod, pyproject.toml \
                 or Python files, foundry.toml): no gate rungs, so runs end unverified until you add \
                 some"
                    .to_string(),
            ),
        }
        gates
    }

    /// Rungs that run `program`'s `steps`, and `lint` as an optional rung,
    /// when `program` is on `PATH`.
    fn toolchain(&mut self, tools: &Toolbox, program: &str, steps: &[(&str, &str)], lint: &str) {
        if !tools.has(program) {
            self.notes.push(format!(
                "`{program}` is not on PATH: no gate rungs; install it, then add them"
            ));
            return;
        }
        for (name, command) in steps {
            self.push(name, command);
        }
        self.push_optional("lint", lint);
    }

    /// Node rungs: the package's `typecheck` or `tsc`, its `test` script, and
    /// its `lint` script as an optional rung, run with the package manager
    /// its lockfile names. A package with dependencies installs them first
    /// when `node_modules` is missing, as it is in the git worktree roko runs
    /// each task in.
    fn node(&mut self, scan: &ProjectScan, tools: &Toolbox) {
        let manager = [
            ("pnpm-lock.yaml", "pnpm"),
            ("yarn.lock", "yarn"),
            ("bun.lock", "bun"),
            ("bun.lockb", "bun"),
        ]
        .iter()
        .find(|(lockfile, manager)| scan.names.contains(*lockfile) && tools.has(manager))
        .map_or("npm", |(_, manager)| *manager);
        if !tools.has(manager) {
            self.notes.push(
                "`npm` is not on PATH: no gate rungs; install Node.js, then add them".to_string(),
            );
            return;
        }
        let install = match manager {
            "npm" if scan.names.contains("package-lock.json") => "npm ci --no-audit --no-fund",
            "npm" => "npm install --no-audit --no-fund",
            "pnpm" => "pnpm install --frozen-lockfile",
            "yarn" => "yarn install --frozen-lockfile",
            _ => "bun install --frozen-lockfile",
        };
        let installed = |command: String| {
            if scan.npm_dependencies {
                format!("(test -d node_modules || {install}) && {command}")
            } else {
                command
            }
        };
        let run = |script: &str| installed(format!("{manager} run {script}"));
        if let Some(script) = ["typecheck", "type-check"]
            .into_iter()
            .find(|script| scan.npm_scripts.contains_key(*script))
        {
            self.push("compile", &run(script));
        } else if scan.names.contains("tsconfig.json") {
            if scan.typescript_installed && tools.has("npx") {
                self.push(
                    "compile",
                    &installed("npx --no-install tsc --noEmit".to_string()),
                );
            } else {
                self.notes.push(
                    "tsconfig.json found, but node_modules holds no TypeScript compiler: no \
                     compile rung"
                        .to_string(),
                );
            }
        }
        match scan.npm_scripts.get("test") {
            // npm's placeholder test script always fails.
            Some(script) if !script.contains("no test specified") => {
                // `bun test` runs bun's own runner, not the script.
                let test = if manager == "bun" {
                    run("test")
                } else {
                    installed(format!("{manager} test"))
                };
                self.push("test", &test);
            }
            _ => self.notes.push(
                "package.json has no test script: no test rung; add one, then a rung that runs it"
                    .to_string(),
            ),
        }
        if scan.npm_scripts.contains_key("lint") {
            self.push_optional("lint", &run("lint"));
        }
        if scan.npm_dependencies && !self.rungs.is_empty() {
            self.notes.push(format!(
                "the rungs run `{install}` first when node_modules is missing, as it is in the git \
                 worktree roko runs each task in"
            ));
        }
    }

    /// Python rungs: the tests, through pytest when `python3` imports it,
    /// else through unittest when the tests use only unittest; a syntax check
    /// of every file when there are no tests to run; ruff and, when the
    /// project configures it, mypy as optional rungs.
    fn python(&mut self, scan: &ProjectScan, tools: &Toolbox) {
        if !tools.has("python3") {
            self.notes.push(
                "`python3` is not on PATH: no gate rungs; install Python, then add them"
                    .to_string(),
            );
            return;
        }
        let pytest = tools.python_modules.contains("pytest");
        match &scan.python_test_dir {
            Some(_) if pytest => self.push(
                "test",
                "PYTHONDONTWRITEBYTECODE=1 python3 -m pytest -q -p no:cacheprovider",
            ),
            Some(dir) if scan.pytest_tests == 0 => self.push(
                "test",
                &format!("PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s {dir} -q"),
            ),
            Some(_) => self.notes.push(format!(
                "{} test file{} need{} pytest, which python3 cannot import: install it \
                 (python3 -m pip install pytest), then add a test rung",
                scan.pytest_tests,
                if scan.pytest_tests == 1 { "" } else { "s" },
                if scan.pytest_tests == 1 { "s" } else { "" },
            )),
            None => self.notes.push(
                "no Python tests found (test_*.py or *_test.py): the compile rung only checks \
                 that every file parses"
                    .to_string(),
            ),
        }
        if self.rungs.is_empty() {
            self.push(
                "compile",
                "python3 -c \"import ast, pathlib; [ast.parse(p.read_bytes(), str(p)) for p in \
                 pathlib.Path('.').rglob('*.py') if not any(part.startswith('.') or part in \
                 ('venv', 'node_modules', 'build', 'dist') for part in p.parts)]\"",
            );
        }
        if tools.has("ruff") {
            self.push_optional("lint", "ruff check .");
        } else if tools.python_modules.contains("ruff") {
            self.push_optional("lint", "python3 -m ruff check .");
        }
        if scan.mypy_configured && tools.python_modules.contains("mypy") {
            self.push_optional("typecheck", "python3 -m mypy .");
        }
    }

    fn push(&mut self, name: &str, command: &str) {
        let timeouts = TimeoutConfig::default();
        let timeout_secs = match name {
            "test" => timeouts.gate_test_secs,
            _ => timeouts.gate_compile_secs,
        };
        self.rungs.push(GateRungConfig {
            name: name.to_string(),
            command: command.to_string(),
            timeout_secs,
            required: true,
            ..GateRungConfig::default()
        });
    }

    /// A rung that runs only when a task asks for it, as a lint whose
    /// result on the unchanged project `roko init` cannot know.
    fn push_optional(&mut self, name: &str, command: &str) {
        self.push(name, command);
        if let Some(rung) = self.rungs.last_mut() {
            rung.timeout_secs = TimeoutConfig::default().gate_clippy_secs;
            rung.required = false;
        }
    }

    /// One line per rung, `test: cargo test --workspace`.
    #[must_use]
    pub fn describe_rungs(&self) -> Vec<String> {
        self.rungs
            .iter()
            .map(|rung| {
                let optional = if rung.required { "" } else { " (optional)" };
                format!("{}: {}{optional}", rung.name, rung.command)
            })
            .collect()
    }
}

// ─── The template ─────────────────────────────────────────────────────────

/// What `roko init` puts in a new `roko.toml`.
#[derive(Clone, Debug, Default)]
pub struct InitOptions {
    /// `--cloud`: serve on every interface.
    pub cloud: bool,
    /// The providers this machine reaches.
    pub machine: InitMachine,
    /// `--model`: the one model every task runs on.
    pub model: Option<String>,
    /// The project's gate rungs.
    pub gates: ProjectGates,
}

impl InitOptions {
    /// Everything `roko init` finds for the directory `target`, with the
    /// `--profile` and `--model` given. Fails on a model it does not know,
    /// before anything is written.
    pub fn detect(
        target: &Path,
        cloud: bool,
        profile: Option<&str>,
        model: Option<&str>,
    ) -> Result<Self> {
        let options = Self {
            cloud,
            machine: InitMachine::detect(),
            model: model.map(str::to_string),
            gates: ProjectGates::detect(target, profile),
        };
        options.models()?;
        Ok(options)
    }

    /// The models and ladder these options write.
    pub fn models(&self) -> Result<InitModels> {
        let global = self.model.as_ref().map(|_| {
            let mut global = RokoConfig::default();
            // A global config that does not load names no model.
            let _ = roko_core::config::loader::merge_global_into(&mut global);
            global
        });
        InitModels::plan(&self.machine, self.model.as_deref(), global.as_ref())
    }
}

/// Whether `workdir` still needs `roko init`, that is, has no `roko.toml`.
///
/// `.roko/` cannot decide this: roko's own log creates it before any
/// command runs, so even an empty directory has one.
#[must_use]
pub fn needs_init(workdir: &Path) -> bool {
    !workdir.join("roko.toml").is_file()
}

/// Write the default `roko.toml` into `target` for a machine with only
/// `provider`, and no gate rungs, and return the text written.
pub fn write_init_config(target: &Path, cloud: bool, provider: InitProvider) -> Result<String> {
    let options = InitOptions {
        cloud,
        machine: provider.into(),
        ..InitOptions::default()
    };
    write_init(target, &options)
}

/// Write the `roko.toml` `options` describe into `target`, and return the
/// text written.
///
/// The text must pass the offline checks of `roko config validate`
/// (`roko_cli::config_cmd::write_checked_config`) before it is written, so
/// `roko init` never leaves a config that commands cannot load.
pub fn write_init(target: &Path, options: &InitOptions) -> Result<String> {
    let text = render_init(options)?;
    roko_cli::config_cmd::write_checked_config(&target.join("roko.toml"), &text)?;
    Ok(text)
}

/// Render the default `roko.toml` template used by `roko init`, for this
/// machine, with no gate rungs.
pub(crate) fn render_init_template(cloud: bool) -> Result<String> {
    render_init(&InitOptions {
        cloud,
        machine: InitMachine::detect(),
        ..InitOptions::default()
    })
}

/// Render the default `roko.toml` template for a machine with only
/// `provider`, with no gate rungs.
pub fn render_init_template_for(cloud: bool, provider: InitProvider) -> Result<String> {
    render_init(&InitOptions {
        cloud,
        machine: provider.into(),
        ..InitOptions::default()
    })
}

/// Render the `roko.toml` `options` describe.
///
/// The base document comes from the v2 schema serializer so the generated
/// workspace starts in the provider/model world rather than the legacy
/// v1 `[agent]` command world.
pub fn render_init(options: &InitOptions) -> Result<String> {
    let models = options.models()?;

    let mut config = RokoConfig::default();
    config.agent.default_backend = "claude".to_string();
    config.agent.default_model = models.default_model.clone();
    if options.cloud {
        config.server.bind = "0.0.0.0".to_string();
    }
    config.gates.custom_rungs = options.gates.rungs.clone();
    config.routing.ladder.rungs = models.rungs.clone();
    config.routing.ladder.start = models.start();

    let mut rendered = config
        .to_toml_pretty()
        .context("serialize default v2 roko.toml")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    let mut out = String::with_capacity(rendered.len() + 2048);
    out.push_str("# REQUIRED_ENV\n");
    out.push_str("# Required environment variables (set in .env or shell):\n");
    out.push_str("# GITHUB_TOKEN       - GitHub personal access token (for MCP GitHub server)\n");
    out.push_str("# GITHUB_WEBHOOK_SECRET - GitHub webhook secret for deploy registration\n");
    out.push_str("# SLACK_BOT_TOKEN    - Slack bot token (for MCP Slack server)\n");
    out.push_str("# SLACK_SIGNING_SECRET - Slack webhook signing secret\n");
    out.push_str("# ANTHROPIC_API_KEY  - Claude API key (for direct API agents, not needed for CLI agents)\n\n");
    out.push_str(&rendered);

    // Every `[models.*]` entry must name a configured provider (config
    // invariant 3), so a model block is only written with its provider.
    if !models.providers.is_empty() {
        out.push_str(
            "\n# The providers and models of the model ladder ([routing.ladder]), those\n",
        );
        out.push_str(
            "# `roko init` found this machine runs: an API key set, or an agent CLI on PATH.\n",
        );
        for provider in &models.providers {
            push_provider(&mut out, provider, "");
        }
        for model in &models.models {
            push_model(&mut out, model, "");
        }
    } else if let Some(rung) = models.rungs.first() {
        out.push_str(&format!(
            "\n# The model ladder's one model, {}, is defined in ~/.roko/config.toml.\n",
            rung.model
        ));
    } else {
        out.push_str(
            "\n# No model provider was found when this workspace was initialized: no API\n",
        );
        out.push_str(
            "# key (ANTHROPIC_API_KEY, OPENAI_API_KEY, OPENROUTER_API_KEY, CEREBRAS_API_KEY,\n",
        );
        out.push_str(
            "# ZAI_API_KEY or MOONSHOT_API_KEY, in the shell or ~/.roko/.env) and no `claude`\n",
        );
        out.push_str(
            "# on PATH, so the model ladder has no rung. Either export ANTHROPIC_API_KEY (the\n",
        );
        out.push_str(
            "# built-in claude-sonnet-4-6 model then uses the Anthropic API), or install\n",
        );
        out.push_str(
            "# Claude CLI and uncomment both blocks below, or set a key, move this file\n",
        );
        out.push_str("# aside and run `roko init` again.\n");
        let claude = MODELS
            .iter()
            .find(|model| model.provider.id == CLAUDE_CLI.id)
            .context("roko init knows a Claude CLI model")?;
        push_provider(&mut out, &CLAUDE_CLI, "# ");
        push_model(&mut out, claude, "# ");
    }

    if options.gates.rungs.is_empty() {
        append_no_rungs_hint(&mut out);
    } else if options.gates.rungs.iter().any(|rung| !rung.required) {
        out.push_str(
            "\n# Gate rungs with `required = false` run only for a task that asks for them:\n",
        );
        out.push_str("# set `required = true` once the unchanged project passes them.\n");
    }

    if options.cloud {
        // A commented example: `roko deploy` registers a webhook on every
        // configured repository, so the template must not name any.
        out.push_str("\n# Auto-register webhooks after deploy: uncomment and name your own\n");
        out.push_str("# repository (one table per repository).\n");
        out.push_str("# [[serve.deploy.webhooks]]\n");
        out.push_str("# provider = \"github\"\n");
        out.push_str("# owner = \"<your-github-owner>\"\n");
        out.push_str("# repo = \"<your-repo>\"\n");
    }

    Ok(out)
}

/// Append `provider`'s table, each line prefixed with `prefix` (`"# "`
/// writes it commented out).
fn push_provider(out: &mut String, provider: &ProviderSpec, prefix: &str) {
    let mut lines = vec![
        format!("[providers.{}]", provider.id),
        format!("kind = \"{}\"", provider.kind),
    ];
    if let Some(base_url) = provider.base_url {
        lines.push(format!("base_url = \"{base_url}\""));
    }
    match provider.access {
        Access::Key(name) => lines.push(format!("api_key_env = \"{name}\"")),
        Access::Command(command) => lines.push(format!("command = \"{command}\"")),
    }
    push_lines(out, &lines, prefix);
}

/// Append `model`'s table, each line prefixed with `prefix`.
fn push_model(out: &mut String, model: &ModelSpec, prefix: &str) {
    let mut lines = vec![
        format!("[models.{}]", model.key),
        format!("provider = \"{}\"", model.provider.id),
        format!("slug = \"{}\"", model.slug),
    ];
    lines.extend(model.fields.iter().map(|line| (*line).to_string()));
    push_lines(out, &lines, prefix);
}

fn push_lines(out: &mut String, lines: &[String], prefix: &str) {
    out.push('\n');
    for line in lines {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
}

fn append_no_rungs_hint(out: &mut String) {
    out.push_str("\n# No gate rungs were written: roko init found no project it checks here\n");
    out.push_str(&format!(
        "# (profiles: {}). Until there is one, a run ends\n",
        InitProfile::NAMES
    ));
    out.push_str("# `unverified`. Replace `rungs = []` under [gates] with rungs such as:\n");
    out.push_str("#   [[gates.rungs]]\n");
    out.push_str("#   name = \"test\"\n");
    out.push_str("#   command = \"make test\"\n");
    out.push_str("# or move this file aside and run `roko init --profile <profile>`.\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9306: `roko deploy` registers a webhook on every configured
    /// repository, so neither `roko init --cloud` nor the example config may
    /// name one. Both keep the webhook table as a commented example.
    #[test]
    fn cloud_init_template_names_no_third_party_repos() {
        let cloud = render_init_template_for(true, InitProvider::ClaudeCli).expect("render");
        let example = RokoConfig::example_toml();
        for (name, text) in [("roko init --cloud", &cloud), ("example config", &example)] {
            let config = RokoConfig::from_toml(text)
                .unwrap_or_else(|err| panic!("{name}: does not parse: {err}"));
            assert!(
                config.serve.deploy.webhooks.is_empty(),
                "{name}: registers deploy webhooks: {:?}",
                config.serve.deploy.webhooks
            );
            assert!(
                text.contains("# [[serve.deploy.webhooks]]"),
                "{name}: lost the commented webhook example"
            );
            // The default worker image moves to the owner's registry in its
            // own change (9341); no other line may name that account.
            let third_party = text
                .lines()
                .filter(|line| line.to_ascii_lowercase().contains("nunchi"))
                .filter(|line| !line.trim_start().starts_with("worker_image"))
                .collect::<Vec<_>>();
            assert!(
                third_party.is_empty(),
                "{name}: names a third-party account: {third_party:?}"
            );
        }
    }

    /// Every rung of `config`'s ladder names one of its `[models.*]` keys,
    /// whose provider it configures, and every tier starts on a rung.
    fn assert_ladder_runs(config: &RokoConfig, label: &str) {
        let ladder = &config.routing.ladder;
        for rung in &ladder.rungs {
            let model = config
                .models
                .get(&rung.model)
                .unwrap_or_else(|| panic!("{label}: rung {} names no model", rung.name));
            assert!(
                config.providers.contains_key(&model.provider),
                "{label}: {} runs on unconfigured provider {}",
                rung.model,
                model.provider
            );
        }
        assert!(ladder.issues().is_empty(), "{label}: {:?}", ladder.issues());
        for tier in roko_core::task::TaskTier::ALL {
            let resolved = ladder.resolve("implementer", tier, None, |model| {
                config.models.contains_key(model)
            });
            assert_eq!(
                resolved.is_some(),
                !ladder.rungs.is_empty(),
                "{label}: {tier} has no start rung"
            );
        }
    }

    /// bug-4e336b: `roko init` wrote D11's four rungs whatever the machine
    /// had, though it defined only the top rung's model, so every task
    /// started on the top rung, and it ignored `--model`. Now each rung
    /// names a model it writes, on a provider this machine reaches, an API
    /// model has a rung whenever a key is set, and `--model` pins the
    /// workspace to its model.
    #[test]
    fn init_writes_a_runnable_ladder_and_honours_model() {
        let render = |machine: InitMachine, model: Option<&str>| {
            let options = InitOptions {
                machine,
                model: model.map(str::to_string),
                ..InitOptions::default()
            };
            let text = render_init(&options).expect("render");
            let dir = tempfile::tempdir().expect("tempdir");
            write_init(dir.path(), &options).expect("roko config validate accepts it");
            RokoConfig::from_toml(&text).expect("parse")
        };
        let rungs = |config: &RokoConfig| {
            config
                .routing
                .ladder
                .rungs
                .iter()
                .map(|rung| format!("{}={}", rung.name, rung.model))
                .collect::<Vec<_>>()
        };
        let provider_of = |config: &RokoConfig, rung: &str| {
            let model = &config
                .routing
                .ladder
                .rungs
                .iter()
                .find(|r| r.name == rung)
                .unwrap()
                .model;
            config.models[model].provider.clone()
        };

        // Will's machine: Cerebras, Z.ai, OpenAI and Moonshot keys, and the
        // Claude CLI.
        let will = InitMachine::default()
            .with_key("CEREBRAS_API_KEY")
            .with_key("ZAI_API_KEY")
            .with_key("OPENAI_API_KEY")
            .with_key("MOONSHOT_API_KEY")
            .with_command("claude");
        let config = render(will.clone(), None);
        assert_ladder_runs(&config, "keys and claude");
        assert_eq!(
            rungs(&config),
            [
                "cheap=gpt-oss-120b",
                "mid=glm-4-7",
                "strong=gpt-5-4-mini",
                "top=claude-sonnet-4-6"
            ]
        );
        assert_eq!(provider_of(&config, "top"), "claude_cli");
        assert_eq!(config.agent.default_model, "claude-sonnet-4-6");
        assert_eq!(config.routing.ladder.start, LadderStart::d11());

        // The Claude CLI alone: one rung, as before, and no phantom ones.
        let config = render(InitMachine::default().with_command("claude"), None);
        assert_ladder_runs(&config, "claude only");
        assert_eq!(rungs(&config), ["top=claude-sonnet-4-6"]);

        // A key gives an API model a rung, even beside the Claude CLI.
        let config = render(
            InitMachine::default()
                .with_command("claude")
                .with_key("ANTHROPIC_API_KEY"),
            None,
        );
        assert_ladder_runs(&config, "claude and anthropic");
        assert_eq!(
            rungs(&config),
            ["strong=claude-haiku-4-5", "top=claude-sonnet-4-6"]
        );
        assert_eq!(provider_of(&config, "strong"), "anthropic");
        assert_eq!(provider_of(&config, "top"), "claude_cli");
        let config = render(InitMachine::default().with_key("OPENROUTER_API_KEY"), None);
        assert_ladder_runs(&config, "openrouter");
        assert_eq!(config.routing.ladder.rungs.len(), 4);
        assert!(
            config
                .models
                .values()
                .all(|model| model.provider == "openrouter")
        );
        assert_eq!(config.agent.default_model, "claude-sonnet-4-6-openrouter");
        let config = render(InitMachine::default().with_key("OPENAI_API_KEY"), None);
        assert_ladder_runs(&config, "openai");
        assert_eq!(rungs(&config), ["strong=gpt-5-4-mini"]);
        assert_eq!(config.agent.default_model, "gpt-5-4-mini");
        let start = &config.routing.ladder.start;
        assert_eq!(start.mechanical.as_deref(), Some("strong"));
        assert_eq!(start.architectural.as_deref(), Some("strong"));

        // Nothing reachable: no rung, rather than rungs that cannot run.
        let config = render(InitMachine::default(), None);
        assert!(config.routing.ladder.rungs.is_empty());
        assert!(config.models.is_empty());

        // --model pins every task to its model, by key or slug.
        for pin in ["glm-4-7", "glm-4.7"] {
            let config = render(will.clone(), Some(pin));
            assert_ladder_runs(&config, pin);
            assert_eq!(config.agent.default_model, "glm-4-7", "{pin}");
            assert_eq!(rungs(&config), ["top=glm-4-7"], "{pin}");
            assert_eq!(config.models["glm-4-7"].provider, "zai");
            let start = &config.routing.ladder.start;
            assert_eq!(start.mechanical.as_deref(), Some("top"));
        }
        // A model whose provider this machine lacks is still written, with
        // a note, so the workspace runs once the key is set.
        let plan = InitModels::plan(&InitMachine::default(), Some("gpt-5.4-mini"), None).unwrap();
        assert_eq!(plan.default_model, "gpt-5-4-mini");
        assert!(
            plan.notes
                .iter()
                .any(|note| note.contains("OPENAI_API_KEY"))
        );
        // A model defined only in ~/.roko/config.toml is named as is.
        let global = RokoConfig::from_toml(
            "[providers.zai]\nkind = \"openai_compat\"\napi_key_env = \"ZAI_API_KEY\"\n\n\
             [models.glm-5-1]\nprovider = \"zai\"\nslug = \"glm-5.1\"\n",
        )
        .unwrap();
        let plan = InitModels::plan(&will, Some("glm-5.1"), Some(&global)).unwrap();
        assert_eq!(plan.default_model, "glm-5-1");
        assert!(plan.models.is_empty());
        // An unknown model is refused before anything is written.
        let err = InitModels::plan(&will, Some("no-such-model"), Some(&global)).unwrap_err();
        assert!(format!("{err:#}").contains("no-such-model"), "{err:#}");
    }

    /// A project with `files` (path, text) in a new directory.
    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (path, text) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    const UNITTEST_PROJECT: &[(&str, &str)] = &[
        ("calc/__init__.py", "def add(a, b):\n    return a + b\n"),
        ("tests/__init__.py", ""),
        (
            "tests/test_calc.py",
            "import unittest\n\nfrom calc import add\n\n\nclass AddTest(unittest.TestCase):\n    \
             def test_add(self):\n        self.assertEqual(add(2, 3), 5)\n",
        ),
    ];

    /// gap-1625c0: `roko init` wrote no gate rung for a Python project, so
    /// its runs ended unverified. A stdlib project (a package and unittest
    /// tests, no packaging file) is detected as Python and gets a test rung
    /// that passes on the unchanged project, through pytest when python3
    /// imports it and unittest otherwise.
    #[test]
    fn python_profile_writes_a_test_rung() {
        let dir = project(UNITTEST_PROJECT);
        let gates = ProjectGates::detect(dir.path(), None);
        assert_eq!(gates.profile, InitProfile::Python);
        assert!(!gates.chosen);

        let scan = ProjectScan::read(dir.path());
        let python = Toolbox::default().with_command("python3");
        let unittest = ProjectGates::for_profile(InitProfile::Python, &scan, &python);
        assert_eq!(
            unittest.describe_rungs(),
            ["test: PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -q"]
        );
        let pytest = python.clone().with_python_module("pytest");
        let with_pytest = ProjectGates::for_profile(InitProfile::Python, &scan, &pytest);
        assert_eq!(
            with_pytest.describe_rungs(),
            ["test: PYTHONDONTWRITEBYTECODE=1 python3 -m pytest -q -p no:cacheprovider"]
        );

        // The rungs this machine gets pass on the unchanged project, and
        // leave nothing behind for the run's commit.
        if !command_on_path("python3") {
            eprintln!("python3 is not on PATH: the rung is not run");
            return;
        }
        assert!(!gates.rungs.is_empty(), "{gates:?}");
        for rung in &gates.rungs {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&rung.command)
                .current_dir(dir.path())
                .output()
                .expect("run the rung");
            assert!(
                output.status.success(),
                "{}: {}\n{}",
                rung.command,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert!(!dir.path().join("tests/__pycache__").exists());
        assert!(!dir.path().join("calc/__pycache__").exists());
    }

    /// Tests that need pytest get no unittest rung, which would fail the
    /// unchanged project, and a project with no tests gets a syntax check.
    #[test]
    fn python_rungs_fall_back_honestly() {
        let python = Toolbox::default().with_command("python3");
        let pytest_style = project(&[
            ("app.py", "def double(x):\n    return 2 * x\n"),
            (
                "tests/test_app.py",
                "from app import double\n\ndef test_double():\n    assert double(2) == 4\n",
            ),
        ]);
        let scan = ProjectScan::read(pytest_style.path());
        assert_eq!(InitProfile::detect(&scan), InitProfile::Python);
        let gates = ProjectGates::for_profile(InitProfile::Python, &scan, &python);
        assert_eq!(gates.rungs.len(), 1, "{gates:?}");
        assert_eq!(gates.rungs[0].name, "compile");
        assert!(
            gates.notes.iter().any(|note| note.contains("pytest")),
            "{gates:?}"
        );

        let no_tests = project(&[
            ("requirements.txt", "requests\n"),
            ("main.py", "print(1)\n"),
        ]);
        let scan = ProjectScan::read(no_tests.path());
        assert_eq!(InitProfile::detect(&scan), InitProfile::Python);
        let gates = ProjectGates::for_profile(InitProfile::Python, &scan, &python);
        assert_eq!(gates.rungs.len(), 1);
        assert_eq!(gates.rungs[0].name, "compile");
        if command_on_path("python3") {
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&gates.rungs[0].command)
                .current_dir(no_tests.path())
                .status()
                .expect("run the rung");
            assert!(status.success());
            std::fs::write(no_tests.path().join("broken.py"), "def (:\n").unwrap();
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&gates.rungs[0].command)
                .current_dir(no_tests.path())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("run the rung");
            assert!(!status.success(), "a syntax error fails the compile rung");
        }

        // Without python3 nothing is written that cannot run.
        let gates = ProjectGates::for_profile(InitProfile::Python, &scan, &Toolbox::default());
        assert!(gates.rungs.is_empty());
        assert!(!gates.notes.is_empty());
    }

    /// Plain `roko init` writes the detected project's rungs, `--profile`
    /// overrides the detection, and each language's rungs run tools the
    /// machine has.
    #[test]
    fn init_detects_the_project_and_writes_its_rungs() {
        let detect =
            |files: &[(&str, &str)]| InitProfile::detect(&ProjectScan::read(project(files).path()));
        assert_eq!(
            detect(&[("Cargo.toml", "[package]\nname = \"x\"\n")]),
            InitProfile::Rust
        );
        assert_eq!(detect(&[("go.mod", "module x\n")]), InitProfile::Go);
        assert_eq!(detect(&[("package.json", "{}")]), InitProfile::TypeScript);
        assert_eq!(detect(&[("pyproject.toml", "")]), InitProfile::Python);
        assert_eq!(detect(&[("foundry.toml", "")]), InitProfile::Solidity);
        assert_eq!(detect(&[("README.md", "# x\n")]), InitProfile::General);
        assert_eq!(detect(&[]), InitProfile::General);

        let cargo = Toolbox::default().with_command("cargo");
        let rust = ProjectGates::for_profile(InitProfile::Rust, &ProjectScan::default(), &cargo);
        assert_eq!(
            rust.describe_rungs(),
            [
                "compile: cargo check --workspace",
                "test: cargo test --workspace",
                "lint: cargo clippy --workspace --no-deps -- -D warnings (optional)",
            ]
        );
        let without = ProjectGates::for_profile(
            InitProfile::Rust,
            &ProjectScan::default(),
            &Toolbox::default(),
        );
        assert!(without.rungs.is_empty());

        let node = project(&[(
            "package.json",
            r#"{"scripts": {"test": "node --test", "lint": "eslint ."}}"#,
        )]);
        let npm = Toolbox::default().with_command("npm");
        let gates = ProjectGates::for_profile(
            InitProfile::TypeScript,
            &ProjectScan::read(node.path()),
            &npm,
        );
        assert_eq!(
            gates.describe_rungs(),
            ["test: npm test", "lint: npm run lint (optional)"]
        );
        // A package with dependencies installs them where node_modules is
        // missing, as in a task's git worktree.
        let with_deps = project(&[
            (
                "package.json",
                r#"{"scripts": {"test": "jest"}, "devDependencies": {"jest": "^29"}}"#,
            ),
            ("package-lock.json", "{}"),
        ]);
        let gates = ProjectGates::for_profile(
            InitProfile::TypeScript,
            &ProjectScan::read(with_deps.path()),
            &npm,
        );
        assert_eq!(
            gates.describe_rungs(),
            ["test: (test -d node_modules || npm ci --no-audit --no-fund) && npm test"]
        );
        let placeholder = project(&[(
            "package.json",
            r#"{"scripts": {"test": "echo \"Error: no test specified\" && exit 1"}}"#,
        )]);
        let gates = ProjectGates::for_profile(
            InitProfile::TypeScript,
            &ProjectScan::read(placeholder.path()),
            &npm,
        );
        assert!(
            gates.rungs.is_empty(),
            "npm's placeholder test always fails: {gates:?}"
        );

        let general = ProjectGates::detect(project(&[]).path(), None);
        assert_eq!(general.profile, InitProfile::General);
        assert!(general.rungs.is_empty());
        let chosen = ProjectGates::detect(project(&[]).path(), Some("python"));
        assert_eq!(chosen.profile, InitProfile::Python);
        assert!(chosen.chosen);
        // A profile init does not know falls back to the detected one, and
        // says so.
        let cargo_project = project(&[("Cargo.toml", "[package]\nname = \"x\"\n")]);
        let unknown = ProjectGates::detect(cargo_project.path(), Some("coding"));
        assert_eq!(unknown.profile, InitProfile::Rust);
        assert!(!unknown.chosen);
        assert!(unknown.notes[0].contains("--profile coding"), "{unknown:?}");

        // The rungs land in [[gates.rungs]], and roko run verifies with them.
        let options = InitOptions {
            machine: InitMachine::default().with_command("claude"),
            gates: rust,
            ..InitOptions::default()
        };
        let config = RokoConfig::from_toml(&render_init(&options).unwrap()).unwrap();
        assert_eq!(config.gates.custom_rungs.len(), 3);
        assert_eq!(config.gates.required_rungs().count(), 2);
    }
}
