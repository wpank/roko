//! Verify (verification) and pipeline configuration sections.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::agent::default_true;
use crate::task::TaskTier;

// ---- [gates] -------------------------------------------------------------

/// Verification breadth selected for a runner gate.
///
/// `Full` preserves the historical release-safe behavior. `Focused` uses the
/// changed Cargo targets and bounded reverse-dependency analysis. The two
/// lighter modes are explicit operator choices and are never inferred when
/// impact analysis is incomplete.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateMode {
    /// Do not execute canonical or task-authored verification.
    None,
    /// Execute structural task verification only.
    Structural,
    /// Execute impact-selected compile/test verification.
    Focused,
    /// Execute the configured canonical pipeline and authored verification.
    #[default]
    Full,
}

impl std::fmt::Display for GateMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::None => "none",
            Self::Structural => "structural",
            Self::Focused => "focused",
            Self::Full => "full",
        })
    }
}

const fn default_impact_timeout_ms() -> u64 {
    5_000
}

const fn default_impact_max_reverse_dependents() -> usize {
    4
}

const fn default_impact_max_targets() -> usize {
    8
}

const fn default_compile_concurrency() -> usize {
    1
}

const fn default_sibling_settle_secs() -> u64 {
    600
}

/// Output-token cap of a Graph attempt whose role `[gates] max_output_tokens`
/// does not list: about five times the largest attempt recorded so far.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u64 = 200_000;

/// What a Graph attempt's edits to paths outside its task's `files` do
/// (`[gates] diff_scope`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffScope {
    /// Log them; the attempt goes on to its verify steps.
    #[default]
    Record,
    /// Fail the attempt before its verify steps.
    Enforce,
}

// ---- [gates.adaptive] defaults -------------------------------------------

const fn default_ema_alpha() -> f64 {
    0.1
}

/// Matches a task's default `max_retries` (3): since adaptive thresholds bound
/// the retry budget of tasks without an authored `max_retries`, a lower floor
/// would cut retries for gates that usually pass.
const fn default_min_retries() -> u32 {
    3
}

const fn default_max_retries() -> u32 {
    5
}

const fn default_skip_streak_threshold() -> u32 {
    20
}

const fn default_convergence_min_observations() -> u64 {
    50
}

/// A single custom gate rung definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateRungConfig {
    /// Human-readable rung identifier (e.g. `"compile"`, `"lint"`, `"test"`).
    pub name: String,
    /// Shell command executed by the gate runner. Runs under `sh -c`.
    pub command: String,
    /// Maximum seconds the command may run before it is killed. Defaults to 120.
    #[serde(default = "default_gate_rung_timeout")]
    pub timeout_secs: u64,
    /// When `true`, a failure on this rung aborts the entire gate pipeline.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Names of other rungs that may execute concurrently with this one.
    #[serde(default)]
    pub parallel_with: Vec<String>,
}

fn default_gate_rung_timeout() -> u64 {
    120
}

impl GateRungConfig {
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs)
    }
}

/// Verify (verification) settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatesConfig {
    /// Explicit verification breadth. Defaults to the historical full lane.
    #[serde(default)]
    pub mode: GateMode,
    /// Enable clippy / lint gate.
    #[serde(default = "default_true")]
    pub clippy_enabled: bool,
    /// Skip test gate entirely.
    #[serde(default)]
    pub skip_tests: bool,
    /// Max gate retry iterations before giving up.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Whether `cargo fix --allow-dirty` (for compile gates) or
    /// `cargo clippy --fix --allow-dirty` (for clippy gates) is attempted
    /// automatically before falling back to agent retry.
    ///
    /// Defaults to `true`. Set to `false` to disable the auto-fix shortcut
    /// and always hand failures directly to the agent.
    #[serde(default = "default_true")]
    pub cargo_fix_enabled: bool,
    /// Write `EvalGenerator` test artifacts to `.roko/generated-tests/` before
    /// each standard-tier Graph task dispatch.
    ///
    /// Defaults to `false`: `plan run` never executes these files. Only the
    /// legacy Runner-v2 generated-test rung reads them.
    #[serde(default)]
    pub write_eval_artifacts: bool,
    /// Maximum time allowed for changed-target and Cargo metadata analysis.
    #[serde(default = "default_impact_timeout_ms")]
    pub impact_timeout_ms: u64,
    /// Maximum reverse-dependent packages selected by a focused gate.
    #[serde(default = "default_impact_max_reverse_dependents")]
    pub impact_max_reverse_dependents: usize,
    /// Maximum exact Cargo targets selected by a focused gate.
    #[serde(default = "default_impact_max_targets")]
    pub impact_max_targets: usize,
    /// Per-repository Cargo command ownership limit.
    #[serde(default = "default_compile_concurrency")]
    pub compile_concurrency: usize,
    /// Seconds a Graph verify step that failed while sibling tasks were
    /// editing the same working tree waits for them to finish their current
    /// attempt before re-running once; only the re-run counts. `0` disables
    /// the wait, so every failure counts at once. Default: 600.
    ///
    /// It also bounds two waits in a shared working tree. A verify step
    /// waits for siblings mid-edit on what it reads before it runs. An
    /// attempt waits for a sibling's verify step that reads its files before
    /// it starts editing.
    #[serde(default = "default_sibling_settle_secs")]
    pub sibling_settle_secs: u64,
    /// Runaway-output guard for Graph task attempts: the most output tokens
    /// an attempt may report before it fails as a red flag, without running
    /// its verify steps. Keyed by task role, with `default` for roles not
    /// listed, e.g. `{ default = 150000, reviewer = 20000 }`; `0` turns the
    /// cap off. Roles neither lists get [`DEFAULT_MAX_OUTPUT_TOKENS`].
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub max_output_tokens: HashMap<String, u64>,
    /// What a Graph attempt's edits to paths outside its task's `files` do:
    /// `record` (the default) logs them, `enforce` fails the attempt before
    /// its verify steps. Edits that weaken tests or touch verify scripts,
    /// pinned acceptance tests or gate config fail it either way.
    #[serde(default)]
    pub diff_scope: DiffScope,
    /// Extra roko environment variables that gate commands (task `verify`
    /// steps, build and test gates) may inherit: exact names or `PREFIX*`
    /// patterns, e.g. `["DATABASE_URL", "AWS_*"]`.
    ///
    /// Gate commands start from an empty environment and inherit only an
    /// allowlist: system basics (`PATH`, `HOME`, `USER`, `SHELL`, `TERM`,
    /// `TMPDIR`, `TZ`, `CI`), locale (`LANG`, `LC_*`), `XDG_*`, toolchain and
    /// build settings (`CARGO_*`, `RUSTUP_*`, `RUSTC*`, `RUST_*`, `NODE_*`,
    /// `NPM_*`, `GO*`, `PYTHON*`, `CC`, `PKG_CONFIG*`, `OPENSSL_*`, ...),
    /// proxies, and `ROKO_*`. Names that look like credentials (`*_KEY`,
    /// `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, ...) and names roko loaded from
    /// `~/.roko/.env` or `.roko/.env` are dropped even then. A name listed
    /// here is always inherited, secret-looking or not. See
    /// `roko_core::child_env`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_passthrough: Vec<String>,
    /// Per-domain gate overrides. Keys are domain labels (e.g. "research", "docs"),
    /// values are shell commands to run as gates (e.g. `["shell:true"]`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub domain_gates: HashMap<String, Vec<String>>,
    /// Custom gate rungs. When non-empty, these replace the built-in defaults.
    /// `roko run` and every `roko plan run` task run the required ones as
    /// verify steps ([`Self::required_rungs`]).
    #[serde(default, rename = "rungs", alias = "custom_rungs")]
    pub custom_rungs: Vec<GateRungConfig>,
    /// Optional ceiling rung index. Rungs above this index are skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rung: Option<u8>,

    // ── Adaptive threshold tuning ─────────────────────────────────────────
    /// EMA decay factor for pass-rate tracking.
    ///
    /// Controls how quickly the exponential moving average adapts to new
    /// observations. Smaller values weight recent observations more heavily.
    /// Range: (0.0, 1.0). Default: 0.1 (the tuned spec value from
    /// docs/04-verification/06-adaptive-thresholds.md).
    #[serde(default = "default_ema_alpha")]
    pub ema_alpha: f64,

    /// Floor for the adaptive retry suggestion — never suggest fewer than
    /// this many retries for any rung. Default: 3 (a task's default
    /// `max_retries`).
    #[serde(default = "default_min_retries")]
    pub adaptive_min_retries: u32,

    /// Ceiling for the adaptive retry suggestion — never suggest more than
    /// this many retries for any rung. Default: 5.
    #[serde(default = "default_max_retries")]
    pub adaptive_max_retries: u32,

    /// Consecutive passes required before the adaptive system advises
    /// skipping a rung entirely. Default: 20.
    #[serde(default = "default_skip_streak_threshold")]
    pub skip_streak_threshold: u32,

    /// Minimum observations on a rung before its learned EMA is considered
    /// converged and eligible for promotion to `Evolved` config provenance.
    /// Default: 50.
    #[serde(default = "default_convergence_min_observations")]
    pub convergence_min_observations: u64,

    // ── Review cycle cap ──────────────────────────────────────────────
    /// Maximum consecutive REVISE cycles of a non-deterministic review/judge
    /// verdict before it may be force-accepted. Never applies to authored
    /// `[[task.verify]]` steps: those are deterministic, so a failure fails
    /// the task once its retries are exhausted. The Graph plan dispatcher
    /// gates on no review/judge verdicts today, so this has no effect on
    /// `roko plan run`. Default: 3.
    #[serde(default = "default_max_review_cycles")]
    pub max_review_cycles: u32,
}

const fn default_max_iterations() -> u32 {
    3
}

const fn default_max_review_cycles() -> u32 {
    3
}

impl Default for GatesConfig {
    fn default() -> Self {
        Self {
            mode: GateMode::Full,
            clippy_enabled: default_true(),
            skip_tests: false,
            max_iterations: default_max_iterations(),
            cargo_fix_enabled: true,
            write_eval_artifacts: false,
            impact_timeout_ms: default_impact_timeout_ms(),
            impact_max_reverse_dependents: default_impact_max_reverse_dependents(),
            impact_max_targets: default_impact_max_targets(),
            compile_concurrency: default_compile_concurrency(),
            sibling_settle_secs: default_sibling_settle_secs(),
            max_output_tokens: HashMap::new(),
            diff_scope: DiffScope::Record,
            env_passthrough: Vec::new(),
            domain_gates: HashMap::new(),
            custom_rungs: Vec::new(),
            max_rung: None,
            ema_alpha: default_ema_alpha(),
            adaptive_min_retries: default_min_retries(),
            adaptive_max_retries: default_max_retries(),
            skip_streak_threshold: default_skip_streak_threshold(),
            convergence_min_observations: default_convergence_min_observations(),
            max_review_cycles: default_max_review_cycles(),
        }
    }
}

impl GatesConfig {
    /// Output-token cap of an attempt of `role` (`max_output_tokens`), or
    /// `None` when the cap is off.
    #[must_use]
    pub fn max_output_tokens_for(&self, role: &str) -> Option<u64> {
        let cap = self
            .max_output_tokens
            .get(role)
            .or_else(|| self.max_output_tokens.get("default"))
            .copied()
            .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);
        (cap > 0).then_some(cap)
    }

    /// Returns true when `[[gates.rungs]]` custom gate configuration is present.
    #[must_use]
    pub fn has_custom_rungs(&self) -> bool {
        !self.custom_rungs.is_empty()
    }

    /// The declared rungs every change must pass: `[[gates.rungs]]` entries
    /// that are `required` and have a command. Empty when the workspace
    /// declares none; the built-in defaults of [`Self::effective_rungs`] are
    /// not declared rungs.
    pub fn required_rungs(&self) -> impl Iterator<Item = &GateRungConfig> {
        self.custom_rungs
            .iter()
            .filter(|rung| rung.required && !rung.command.trim().is_empty())
    }

    /// Returns custom rungs if configured, otherwise built-in defaults (compile, lint, test).
    #[must_use]
    pub fn effective_rungs(&self) -> Vec<GateRungConfig> {
        if self.has_custom_rungs() {
            return self.custom_rungs.clone();
        }
        vec![
            GateRungConfig {
                name: "compile".to_string(),
                command: "cargo build --workspace".to_string(),
                timeout_secs: 120,
                required: true,
                parallel_with: Vec::new(),
            },
            GateRungConfig {
                name: "lint".to_string(),
                command: "cargo clippy --workspace --no-deps -- -D warnings".to_string(),
                timeout_secs: 120,
                required: true,
                parallel_with: Vec::new(),
            },
            GateRungConfig {
                name: "test".to_string(),
                command: "cargo test --workspace".to_string(),
                timeout_secs: 300,
                required: true,
                parallel_with: Vec::new(),
            },
        ]
    }
}

// ---- [pipeline] ---------------------------------------------------------

/// Reviewer composition for a pipeline band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineReviewerMode {
    /// Single quick-pass reviewer.
    Quick,
    /// Full review suite (architect, auditor, scribe).
    Full,
}

impl PipelineReviewerMode {
    /// Stable config label used in TOML.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
        }
    }
}

impl Default for PipelineReviewerMode {
    fn default() -> Self {
        Self::Quick
    }
}

/// Effective pipeline settings for one complexity band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineBandConfig {
    /// Whether the strategist stage runs before implementation.
    #[serde(default)]
    pub strategist: bool,
    /// Whether reviewer agents run after implementation.
    #[serde(default)]
    pub reviewers: bool,
    /// Which reviewer composition to use when reviewers are enabled.
    #[serde(default)]
    pub reviewer_mode: PipelineReviewerMode,
    /// Maximum implementation-review iterations before stopping.
    #[serde(default = "default_pipeline_band_iterations")]
    pub max_iterations: u32,
    /// Runaway guard: agent turn cap for each Graph task dispatch in this
    /// tier (`plan run`). Not a working budget; ordinary tasks should never
    /// reach it.
    ///
    /// Forwarded as the provider turn limit (`--max-turns` for Claude CLI);
    /// providers without a native turn limit ignore it. A run that hits it
    /// fails as `TurnLimitReached`, and the retry raises the cap by half and
    /// resumes the partial work. Values below 1 are raised to 1. Defaults:
    /// mechanical 40, focused 60, integrative 90, architectural 120.
    #[serde(default = "default_pipeline_band_max_turns")]
    pub max_turns: u32,
}

const fn default_pipeline_band_iterations() -> u32 {
    1
}

const MECHANICAL_MAX_TURNS: u32 = 40;
const FOCUSED_MAX_TURNS: u32 = 60;
const INTEGRATIVE_MAX_TURNS: u32 = 90;
const ARCHITECTURAL_MAX_TURNS: u32 = 120;

const fn default_pipeline_band_max_turns() -> u32 {
    FOCUSED_MAX_TURNS
}

impl PipelineBandConfig {
    /// Defaults for the `mechanical` tier.
    #[must_use]
    pub const fn mechanical() -> Self {
        Self {
            strategist: false,
            reviewers: false,
            reviewer_mode: PipelineReviewerMode::Quick,
            max_iterations: 1,
            max_turns: MECHANICAL_MAX_TURNS,
        }
    }

    /// Defaults for the `focused` tier.
    #[must_use]
    pub const fn focused() -> Self {
        Self {
            strategist: false,
            reviewers: false,
            reviewer_mode: PipelineReviewerMode::Quick,
            max_iterations: 2,
            max_turns: FOCUSED_MAX_TURNS,
        }
    }

    /// Defaults for the `integrative` tier.
    #[must_use]
    pub const fn integrative() -> Self {
        Self {
            strategist: true,
            reviewers: true,
            reviewer_mode: PipelineReviewerMode::Quick,
            max_iterations: 2,
            max_turns: INTEGRATIVE_MAX_TURNS,
        }
    }

    /// Defaults for the `architectural` tier.
    #[must_use]
    pub const fn architectural() -> Self {
        Self {
            strategist: true,
            reviewers: true,
            reviewer_mode: PipelineReviewerMode::Full,
            max_iterations: 3,
            max_turns: ARCHITECTURAL_MAX_TURNS,
        }
    }
}

impl Default for PipelineBandConfig {
    fn default() -> Self {
        Self::mechanical()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
struct PipelineBandConfigOverride {
    #[serde(default)]
    strategist: Option<bool>,
    #[serde(default)]
    reviewers: Option<bool>,
    #[serde(default)]
    reviewer_mode: Option<PipelineReviewerMode>,
    #[serde(default)]
    max_iterations: Option<u32>,
    #[serde(default)]
    max_turns: Option<u32>,
}

impl PipelineBandConfigOverride {
    fn resolve(self, defaults: PipelineBandConfig) -> PipelineBandConfig {
        PipelineBandConfig {
            strategist: self.strategist.unwrap_or(defaults.strategist),
            reviewers: self.reviewers.unwrap_or(defaults.reviewers),
            reviewer_mode: self.reviewer_mode.unwrap_or(defaults.reviewer_mode),
            max_iterations: self.max_iterations.unwrap_or(defaults.max_iterations),
            max_turns: self.max_turns.unwrap_or(defaults.max_turns),
        }
    }
}

fn deserialize_pipeline_band_with_defaults<'de, D>(
    deserializer: D,
    defaults: PipelineBandConfig,
) -> Result<PipelineBandConfig, D::Error>
where
    D: Deserializer<'de>,
{
    let override_cfg = PipelineBandConfigOverride::deserialize(deserializer)?;
    Ok(override_cfg.resolve(defaults))
}

fn default_mechanical_pipeline() -> PipelineBandConfig {
    PipelineBandConfig::mechanical()
}

fn deserialize_mechanical_pipeline<'de, D>(deserializer: D) -> Result<PipelineBandConfig, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_pipeline_band_with_defaults(deserializer, PipelineBandConfig::mechanical())
}

fn default_focused_pipeline() -> PipelineBandConfig {
    PipelineBandConfig::focused()
}

fn deserialize_focused_pipeline<'de, D>(deserializer: D) -> Result<PipelineBandConfig, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_pipeline_band_with_defaults(deserializer, PipelineBandConfig::focused())
}

fn default_integrative_pipeline() -> PipelineBandConfig {
    PipelineBandConfig::integrative()
}

fn deserialize_integrative_pipeline<'de, D>(deserializer: D) -> Result<PipelineBandConfig, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_pipeline_band_with_defaults(deserializer, PipelineBandConfig::integrative())
}

fn default_architectural_pipeline() -> PipelineBandConfig {
    PipelineBandConfig::architectural()
}

fn deserialize_architectural_pipeline<'de, D>(
    deserializer: D,
) -> Result<PipelineBandConfig, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_pipeline_band_with_defaults(deserializer, PipelineBandConfig::architectural())
}

fn default_pipeline_template() -> String {
    "standard".to_string()
}

/// Whether dispatch sets turn caps and attempt timeouts from each tier's
/// history (`[pipeline] learned_limits`, gap-5a6e01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearnedLimitsMode {
    /// Use the configured limits, and read no history.
    Off,
    /// Use the configured limits, and log each tier's learned ones beside
    /// them.
    #[default]
    Shadow,
    /// Use a tier's learned limits once it has enough history.
    On,
}

/// Complexity-to-pipeline mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineConfig {
    /// Workflow template used by `roko run` when no per-task band is resolved.
    /// Valid values: "standard", "express", "focused", "integrative", "full", "architectural".
    /// Defaults to "standard".
    #[serde(default = "default_pipeline_template")]
    pub default_template: String,
    /// Turn caps and attempt timeouts from the p95 of each tier's passed
    /// attempts (`roko_learn::tier_limits`): `off`, `shadow` (log them next
    /// to the configured limits; the default) or `on` (use them). An
    /// authored `timeout_secs` always wins.
    #[serde(default)]
    pub learned_limits: LearnedLimitsMode,
    /// Mechanical tasks: skip strategist and reviewers.
    #[serde(
        default = "default_mechanical_pipeline",
        deserialize_with = "deserialize_mechanical_pipeline"
    )]
    pub mechanical: PipelineBandConfig,
    /// Focused tasks: implement directly, allow one extra loop.
    #[serde(
        default = "default_focused_pipeline",
        deserialize_with = "deserialize_focused_pipeline"
    )]
    pub focused: PipelineBandConfig,
    /// Integrative tasks: strategist plus a quick reviewer.
    #[serde(
        default = "default_integrative_pipeline",
        deserialize_with = "deserialize_integrative_pipeline"
    )]
    pub integrative: PipelineBandConfig,
    /// Architectural tasks: strategist plus the full reviewer suite.
    #[serde(
        default = "default_architectural_pipeline",
        deserialize_with = "deserialize_architectural_pipeline"
    )]
    pub architectural: PipelineBandConfig,
}

impl PipelineConfig {
    /// Resolve the pipeline settings for a task tier.
    #[must_use]
    pub fn for_tier(&self, tier: TaskTier) -> PipelineBandConfig {
        match tier {
            TaskTier::Mechanical => self.mechanical,
            TaskTier::Focused => self.focused,
            TaskTier::Integrative => self.integrative,
            TaskTier::Architectural => self.architectural,
        }
    }

    /// Agent turn cap for a task tier, never below one turn. A task whose
    /// tier is unknown reads as focused (`TaskDef::tier_class`), so the cap
    /// is never unbounded.
    #[must_use]
    pub fn max_turns_for_tier(&self, tier: TaskTier) -> u32 {
        self.for_tier(tier).max_turns.max(1)
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            default_template: default_pipeline_template(),
            learned_limits: LearnedLimitsMode::default(),
            mechanical: PipelineBandConfig::mechanical(),
            focused: PipelineBandConfig::focused(),
            integrative: PipelineBandConfig::integrative(),
            architectural: PipelineBandConfig::architectural(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::schema::RokoConfig;
    use crate::task::TaskTier;

    #[test]
    fn gates_rungs_deserializes_as_custom_rungs() {
        let cfg = RokoConfig::from_toml(
            r#"
[gates]
clippy_enabled = true

[[gates.rungs]]
name = "compile"
command = "cargo check --workspace"
timeout_secs = 120
required = true

[[gates.rungs]]
name = "test"
command = "cargo test --workspace"
timeout_secs = 300
required = true
"#,
        )
        .expect("config parses");

        assert!(cfg.gates.has_custom_rungs());
        assert_eq!(cfg.gates.effective_rungs().len(), 2);
        assert_eq!(cfg.gates.effective_rungs()[0].name, "compile");
        assert_eq!(cfg.gates.effective_rungs()[1].timeout_secs, 300);
    }

    #[test]
    fn required_rungs_are_the_declared_required_rungs_with_a_command() {
        let default = RokoConfig::default();
        assert_eq!(default.gates.effective_rungs().len(), 3);
        assert_eq!(
            default.gates.required_rungs().count(),
            0,
            "built-in defaults are not declared rungs"
        );

        let cfg = RokoConfig::from_toml(
            r#"
[[gates.rungs]]
name = "compile"
command = "cargo check --workspace"

[[gates.rungs]]
name = "bench"
command = "cargo bench"
required = false

[[gates.rungs]]
name = "blank"
command = "  "
"#,
        )
        .expect("config parses");
        let names: Vec<&str> = cfg
            .gates
            .required_rungs()
            .map(|rung| rung.name.as_str())
            .collect();
        assert_eq!(names, ["compile"]);
    }

    #[test]
    fn pipeline_max_turns_defaults_are_bounded_per_tier() {
        let pipeline = super::PipelineConfig::default();
        let turns =
            |tier: &str| pipeline.max_turns_for_tier(TaskTier::parse(tier).unwrap_or_default());
        assert_eq!(turns("mechanical"), 40);
        assert_eq!(turns("trivial"), 40, "an alias of mechanical");
        assert_eq!(turns("focused"), 60);
        assert_eq!(turns("integrative"), 90);
        assert_eq!(turns("Architectural"), 120);
        // Unknown and empty tiers read as focused, never unbounded.
        assert_eq!(turns("mechancial"), 60);
        assert_eq!(turns(""), 60);
    }

    #[test]
    fn pipeline_max_turns_override_keeps_other_band_defaults() {
        let cfg = RokoConfig::from_toml(
            r#"
[pipeline.integrative]
max_turns = 45

[pipeline.focused]
max_turns = 0
"#,
        )
        .expect("config parses");

        assert_eq!(cfg.pipeline.max_turns_for_tier(TaskTier::Integrative), 45);
        assert!(
            cfg.pipeline.integrative.strategist,
            "unset keys keep defaults"
        );
        assert_eq!(cfg.pipeline.integrative.max_iterations, 2);
        assert_eq!(
            cfg.pipeline.max_turns_for_tier(TaskTier::Focused),
            1,
            "zero is raised to one turn"
        );
        assert_eq!(
            cfg.pipeline.max_turns_for_tier(TaskTier::Architectural),
            120
        );
    }

    #[test]
    fn learned_limits_default_to_shadow_and_parse() {
        use super::LearnedLimitsMode;

        let defaults = super::PipelineConfig::default();
        assert_eq!(defaults.learned_limits, LearnedLimitsMode::Shadow);
        let cfg =
            RokoConfig::from_toml("[pipeline]\nlearned_limits = \"on\"\n").expect("config parses");
        assert_eq!(cfg.pipeline.learned_limits, LearnedLimitsMode::On);
        assert_eq!(
            cfg.pipeline.focused, defaults.focused,
            "bands keep defaults"
        );
        assert!(RokoConfig::from_toml("[pipeline]\nlearned_limits = \"sometimes\"\n").is_err());
    }

    #[test]
    fn eval_artifacts_are_opt_in() {
        assert!(!super::GatesConfig::default().write_eval_artifacts);
        let cfg =
            RokoConfig::from_toml("[gates]\nwrite_eval_artifacts = true\n").expect("config parses");
        assert!(cfg.gates.write_eval_artifacts);
    }

    #[test]
    fn output_token_caps_fall_back_from_role_to_default_to_builtin() {
        let builtin = super::GatesConfig::default();
        assert_eq!(
            builtin.max_output_tokens_for("implementer"),
            Some(super::DEFAULT_MAX_OUTPUT_TOKENS)
        );

        let cfg = RokoConfig::from_toml(
            "[gates.max_output_tokens]\ndefault = 5000\nreviewer = 800\nscribe = 0\n",
        )
        .expect("config parses");
        assert_eq!(cfg.gates.max_output_tokens_for("reviewer"), Some(800));
        assert_eq!(cfg.gates.max_output_tokens_for("implementer"), Some(5000));
        assert_eq!(cfg.gates.max_output_tokens_for("scribe"), None, "0 is off");
    }

    #[test]
    fn diff_scope_records_by_default_and_parses_enforce() {
        assert_eq!(
            super::GatesConfig::default().diff_scope,
            super::DiffScope::Record
        );
        let cfg = RokoConfig::from_toml("[gates]\ndiff_scope = \"enforce\"\n").expect("parses");
        assert_eq!(cfg.gates.diff_scope, super::DiffScope::Enforce);
        assert!(RokoConfig::from_toml("[gates]\ndiff_scope = \"strict\"\n").is_err());
    }
}
