# Part I: Roko architecture

This document gives you everything you need to implement a new gate inside Roko. It is self-contained. Every type, trait, and struct is defined inline with exact signatures taken from the codebase. You do not need access to the source to understand the architecture.

---

## 1. What Roko is

Roko is a Rust toolkit for building agents that build themselves. 18 crates, ~177K lines of code. The system reads PRDs (product requirement documents), generates implementation plans, executes tasks via Claude-powered agents, validates results through gates, and persists everything as immutable signals.

Roko develops itself. The plan-execute-gate-persist loop that runs user tasks also runs Roko's own development.

The crates relevant to gate work:

| Crate | Path | Purpose |
|---|---|---|
| `roko-core` | `crates/roko-core/` | The kernel: `Engram` type, 6 verb traits, `Verdict`, `Score`, `Decay`, config, errors |
| `roko-gate` | `crates/roko-gate/` | 11+ concrete gate implementations, 7-rung pipeline, adaptive thresholds, gate composition |
| `roko-cli` | `crates/roko-cli/` | CLI binary, all subcommands, and the orchestration loop that wires gates into execution |
| `roko-serve` | `crates/roko-serve/` | HTTP control plane (~85 REST routes on port 6677) |
| `roko-compose` | `crates/roko-compose/` | Prompt assembly, 9-layer system prompt builder |
| `roko-learn` | `crates/roko-learn/` | Episodes, playbooks, model routing, experiments, efficiency tracking |
| `roko-agent` | `crates/roko-agent/` | LLM backends (Claude CLI, Claude API, Codex, OpenAI-compat, Ollama, Gemini, Perplexity), tool loop, safety |

---

## 2. The signal: Engram

The `Engram` is the single universal noun. Every event, every piece of data, every agent output, every gate verdict is an Engram. They are:

- **Content-addressed** -- BLAKE3 hash is the identity
- **Decaying** -- weight fades over time via a configurable decay function
- **Scored** -- multi-dimensional quality rating (7 axes)
- **Traced** -- lineage forms a DAG for auditing

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Engram {
    /// Content-addressed identity (computed from kind + body + author + tags).
    pub id: ContentHash,

    /// HDC fingerprint plus encoder metadata for similarity search and clustering.
    /// Optional because callers can construct engrams before fingerprinting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<HdcFingerprint>,

    /// What kind of engram this is.
    pub kind: Kind,

    /// The engram's payload.
    pub body: Body,

    /// Unix milliseconds when this engram was first emitted.
    pub created_at_ms: i64,

    /// How this engram's weight decays over time.
    pub decay: Decay,

    /// Producer attribution and trust.
    pub provenance: Provenance,

    /// Quality score at emission time (may be recomputed by scorers).
    pub score: Score,

    /// ContentHashes of engrams this derived from (forms a DAG for auditing).
    pub lineage: Vec<ContentHash>,

    /// Arbitrary string metadata (ordered for stable hashing).
    pub tags: BTreeMap<String, String>,

    /// Optional cryptographic proof of origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<Attestation>,

    /// Optional emotional metadata from the daimon affect engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotional_tag: Option<EmotionalTag>,
}
```

`HdcFingerprint` wraps a hyperdimensional computing vector with versioning:

```rust
pub struct HdcFingerprint {
    /// The semantic fingerprint vector.
    pub vector: HdcVector,
    /// Monotonic version of the encoder that derived the vector.
    pub encoder_version: u32,
}
```

### Body

The typed payload carried by an engram:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", content = "data")]
pub enum Body {
    /// Empty -- the signal is purely a marker (kind and tags carry meaning).
    Empty,
    /// UTF-8 text (logs, prompts, messages).
    Text(String),
    /// Structured JSON value.
    Json(serde_json::Value),
    /// Raw bytes (binary artifacts, compressed data).
    Bytes(Vec<u8>),
}
```

Gates typically read the body as JSON. The `GatePayload` type (defined below) is the standard JSON body that gates expect.

### Kind

What an engram represents. The enum is `#[non_exhaustive]` with a `Custom(String)` escape hatch:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Kind {
    // Agent runtime
    ProcessSpawn,
    ProcessExit,
    AgentMessage,
    AgentOutput,
    TokenUsage,
    ApprovalRequested,

    // Verification
    GateVerdict,
    TestResult,
    CompileDiagnostic,

    // Tasks and plans
    Task,
    Prompt,
    Completion,
    PlanRevision,

    // Context and memory
    Pheromone,
    Episode,
    PlaybookRule,
    Skill,
    Compound(Vec<Kind>),

    // Observability
    Metric,
    ExperimentResult,
    ToolInvocation,
    ToolHealthDegraded,

    // Chain participation (Phase 8+)
    ChainInsight,

    // Escape hatch for extensions
    Custom(String),
}
```

### Score

Seven axes combined into a single scalar for ranking:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// [0..1] how confident are we this signal is correct/valid?
    pub confidence: f32,
    /// [0..1] how novel is this signal compared to prior signals?
    pub novelty: f32,
    /// [0..inf) how useful has this signal proven historically?
    pub utility: f32,
    /// [0..inf) reputation of the signal's author at emission time.
    pub reputation: f32,
    /// [0..1] how exact or narrowly applicable is this signal?
    #[serde(default)]
    pub precision: f32,
    /// [0..1] how much extra ranking weight should this signal receive?
    #[serde(default)]
    pub salience: f32,
    /// [0..1] how internally consistent is the evidence?
    #[serde(default)]
    pub coherence: f32,
}
```

The combination formula:

```rust
impl Score {
    pub fn effective(&self) -> f32 {
        let salience_factor = if self.salience == 0.0 { 1.0 } else { 0.5 + 0.5 * self.salience };
        let coherence_factor = if self.coherence == 0.0 { 1.0 } else { 0.5 + 0.5 * self.coherence };

        self.confidence
            * (1.0 + self.novelty)
            * (1.0 + self.utility)
            * self.reputation
            * salience_factor
            * coherence_factor
    }
}
```

Non-finite values (NaN, infinity) in any axis force `effective()` to return 0.0.

Constructor helpers:
- `Score::new(confidence, novelty, utility, reputation)` -- sets precision, salience, coherence to 0.0
- `Score::new_extended(confidence, novelty, utility, reputation, precision, salience, coherence)`
- `Score::ZERO` -- all axes zeroed

### Decay

How a signal's weight diminishes over time. `Decay::apply(age_ms)` returns a multiplier in [0.0, 1.0]:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Decay {
    /// No decay -- permanent weight (config, schemas, identity).
    None,

    /// Exponential half-life: weight = 0.5 ^ (age / half_life_ms).
    HalfLife { half_life_ms: u64 },

    /// Hard cutoff: full weight until ttl_ms, then zero.
    Ttl { ttl_ms: u64 },

    /// Ebbinghaus forgetting curve: weight = exp(-age / (strength * scale_ms)).
    Ebbinghaus { strength: f32, scale_ms: u64 },
}
```

### weight_at

The engram's effective weight at a point in time combines score and decay:

```rust
impl Engram {
    pub fn weight_at(&self, now_ms: i64) -> f32 {
        let age = now_ms - self.created_at_ms;
        self.score.effective() * self.decay.apply(age)
    }
}
```

### Provenance

Who produced a signal and how trusted they are:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    /// Identifier of the producer (agent role, user email, chain address, etc.).
    pub author: String,
    /// Trust score [0..1] at time of emission.
    /// 1.0 = fully trusted; 0.5 = unverified but internal; 0.0 = untrusted.
    pub trust: f32,
    /// Typed taint classification. Taint::Clean means untainted.
    #[serde(default)]
    pub taint: Taint,
}
```

Taint variants: `Clean`, `External { source }`, `Propagated { detail, inherited_from }`, `UserInput`.

### Engram builder

Engrams are constructed via a builder pattern:

```rust
let engram = Engram::builder(Kind::GateVerdict)
    .body(Body::Json(serde_json::to_value(&verdict)?))
    .score(Score::new(1.0, 0.0, 0.0, 1.0))
    .decay(Decay::None)
    .provenance(Provenance { author: "ui_gate".into(), trust: 1.0, taint: Taint::Clean })
    .build();
```

The builder computes the `ContentHash` from kind + body + author + tags at `.build()` time.

---

## 3. The six verb traits

Everything Roko does is an implementation of one of six traits. They define the entire operational surface.

### Substrate -- storage

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    /// Persist an engram and return its content hash.
    async fn write(&self, engram: Engram) -> Result<ContentHash>;

    /// Retrieve an engram by its content hash.
    async fn read(&self, hash: &ContentHash) -> Result<Option<Engram>>;

    /// Query engrams matching a topic filter.
    async fn query(&self, filter: &TopicFilter) -> Result<Vec<Engram>>;

    /// Human-readable name for this substrate.
    fn name(&self) -> &str;
}
```

Primary implementation: `FileSubstrate` -- writes JSONL files to `.roko/signals.jsonl`.

### Scorer -- rating

```rust
pub trait Scorer: Send + Sync {
    /// Rate an engram along the 7 score axes given a runtime context.
    fn score(&self, engram: &Engram, ctx: &Context) -> Score;

    /// Human-readable name.
    fn name(&self) -> &'static str;
}
```

Pure function of (engram, context). Implementations: `RelevanceScorer`, `RecencyScorer`, `ReputationScorer`, `CatalyticScorer`.

### Gate -- verification

```rust
#[async_trait]
pub trait Gate: Send + Sync {
    /// Verify a signal against ground truth.
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;

    /// Human-readable name.
    fn name(&self) -> &str;
}
```

**This is the trait you will implement.** Gates bridge the system to external reality: compile the code, run tests, check the UI, validate schemas. A gate returning `passed: true` claims the engram is correct in some domain.

The `verify` method receives:
- `engram` -- the signal to verify (its body typically contains a `GatePayload` with the working directory)
- `ctx` -- the runtime context (current time, goal, budget, arbitrary attributes)

It returns a `Verdict` (defined below).

### Router -- selection

```rust
pub trait Router: Send + Sync {
    /// Select one engram from candidates given context.
    fn select(&self, candidates: &[Engram], ctx: &Context) -> Option<Selection>;

    /// Learn from an outcome after acting on a selection.
    fn feedback(&self, outcome: &Outcome);

    /// Human-readable name.
    fn name(&self) -> &str;
}
```

Implementations: `StaticRouter`, `LinUCBRouter`, `CascadeRouter` (learns which model to use per task complexity), `WeightedRouter`.

### Composer -- assembly

```rust
pub trait Composer: Send + Sync {
    /// Combine multiple engrams into one under budget constraints.
    fn compose(
        &self,
        engrams: &[Engram],
        budget: &Budget,
        scorer: &dyn Scorer,
        ctx: &Context,
    ) -> Result<Engram>;

    /// Human-readable name.
    fn name(&self) -> &str;
}
```

Used for prompt assembly. The 9-layer `SystemPromptBuilder` is a Composer. Budget constraints include token limits, byte limits, and wall-time limits.

### Policy -- reaction

```rust
pub trait Policy: Send + Sync {
    /// Watch the engram stream and emit new engrams in response.
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
}
```

Policies fire after persistence. They implement circuit breakers, episode logging, pheromone reactions, gate failure replan triggers, and adaptive threshold updates.

---

## 4. The Context type

Every trait method receives a `Context`:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Context {
    /// Current time in Unix milliseconds. Used for decay calculations and timeouts.
    pub now_ms: i64,
    /// High-level goal description (what the agent/gate/router is trying to achieve).
    pub goal: String,
    /// Arbitrary key-value attributes. Extensible without changing the struct.
    pub attrs: BTreeMap<String, String>,
}
```

Constructor helpers:
- `Context::now()` -- sets `now_ms` to the current wall clock
- `Context::now().with_goal("verify UI renders correctly")`
- `Context::now().with_attr("plan_id", "P1")`

---

## 5. The universal loop

Every operation in Roko follows this 8-step loop:

```
query -> score -> route -> compose -> dispatch -> gate -> persist -> react
```

For a task execution:

1. **Query**: Pull relevant engrams from the Substrate -- prior episodes, knowledge entries, task context.
2. **Score**: Rate each engram along 7 axes using configured Scorers.
3. **Route**: CascadeRouter selects which LLM model/backend to use based on task complexity and history.
4. **Compose**: SystemPromptBuilder assembles a 9-layer prompt from role template, task description, retrieved context, tool definitions, safety constraints, etc.
5. **Dispatch**: Send the composed prompt to the selected LLM agent (Claude CLI, Claude API, etc.). The agent executes and produces output.
6. **Gate**: Run the gate pipeline on the agent's output. This is where verification happens -- compile, test, lint, and (with the visual gate feature) visual verification.
7. **Persist**: Write the agent output, gate verdicts, and episode metadata as Engrams to the Substrate (FileSubstrate writes JSONL files to disk).
8. **React**: Policies fire -- episode logging, efficiency tracking, adaptive threshold updates, gate failure replan triggers, pheromone deposits.

---

## 6. The gate system

### Verdict

Every gate produces a `Verdict`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verdict {
    /// Did the signal pass the gate?
    pub passed: bool,
    /// Human-readable reason (used for logs, error messages).
    pub reason: String,
    /// Identifier of the gate that rendered this verdict.
    pub gate: String,
    /// Numeric score in [0..1] -- useful for thresholding (e.g. judge gates).
    pub score: f32,
    /// Optional detail string (stdout, error output, diagnostic).
    pub detail: Option<String>,
    /// Structured test counts (populated by test gates).
    pub test_count: Option<TestCount>,
    /// Structured error digest for feeding back to agents.
    pub error_digest: Option<String>,
    /// Wall-clock duration the gate took, in milliseconds.
    pub duration_ms: u64,
}
```

`TestCount`:

```rust
pub struct TestCount {
    pub passed: u32,
    pub failed: u32,
    pub ignored: u32,
}
```

Constructor helpers:

```rust
// Passing verdict: passed=true, score=1.0, empty reason.
Verdict::pass("ui_gate")

// Failing verdict: passed=false, score=0.0, with reason.
Verdict::fail("ui_gate", "button overlaps sidebar")

// Builder methods (chainable):
.with_score(0.7)          // override the numeric score (clamped to [0..1])
.with_detail("stdout...")  // attach diagnostic output
.with_duration(elapsed_ms) // set wall-clock duration
.with_error_digest("...")  // structured error summary for agent feedback
```

### The 7-rung pipeline

Gates are organized into 7 rungs, executed in order. The pipeline short-circuits on the first failure by default.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rung {
    Compile       = 0,  // CompileGate -- does the code compile?
    Lint          = 1,  // ClippyGate -- lint warnings/errors?
    Test          = 2,  // TestGate -- do tests pass?
    Symbol        = 3,  // SymbolGate -- do expected symbols exist?
    GeneratedTest = 4,  // GeneratedTestGate -- do AI-generated tests pass?
    PropertyTest  = 5,  // PropertyTestGate + FactCheckGate
    Integration   = 6,  // LlmJudgeGate + IntegrationGate
}

pub const CANONICAL_ORDER: [Rung; 7] = [
    Rung::Compile,
    Rung::Lint,
    Rung::Test,
    Rung::Symbol,
    Rung::GeneratedTest,
    Rung::PropertyTest,
    Rung::Integration,
];
```

### Plan complexity and rung selection

Which rungs execute depends on plan complexity:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanComplexity {
    Trivial,   // Compile only
    Simple,    // Compile + Lint
    Standard,  // Compile + Lint + Test + Symbol
    Complex,   // All 7 rungs
}
```

The `select_rungs` function determines which rungs to run:

```rust
pub fn select_rungs(
    complexity: PlanComplexity,
    caps: &RungCaps,
    prior_failures: u32,
) -> Vec<Rung>
```

On repeated failure, complexity escalates. `PlanComplexity::escalate()` moves one step toward `Complex` (saturates there). Two prior failures promote Trivial to Simple; four promote Simple to Standard.

Rung availability is filtered by capabilities:

```rust
pub struct RungCaps {
    pub has_lint_tool: bool,
    pub has_symbol_manifest: bool,
    pub has_generated_tests: bool,
    pub has_property_tests: bool,
    pub has_integration_scenario: bool,
}
```

A rung only runs if the corresponding capability is present. Compile and Test always run (no capability check). `RungCaps::all()` enables every rung.

### GatePayload

The standard input body that gates expect to find in an engram:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatePayload {
    /// Directory to run the gate in (the repo root or a worktree).
    pub working_dir: PathBuf,
    /// Optional CARGO_TARGET_DIR override.
    pub target_dir: Option<PathBuf>,
    /// Additional environment variables for the gate's subprocess.
    pub extra_env: Vec<(String, String)>,
    /// Optional identifying label for logging.
    pub label: Option<String>,
}
```

Constructor: `GatePayload::in_dir("/path/to/project")` with chainable `.with_env("key", "val")` and `.with_label("T1")`.

Gates read this from the engram body:

```rust
let payload: GatePayload = serde_json::from_value(
    engram.body.as_json().cloned().unwrap_or_default()
)?;
```

### GatePipeline

The pipeline composes inner gates behind a single `Gate` impl:

```rust
pub struct GatePipeline {
    gates: Vec<Box<dyn Gate>>,
    short_circuit: bool,
    name: String,
}
```

The pipeline is itself a `Gate`. It runs inner gates in push order.

```rust
impl GatePipeline {
    /// Start with an empty pipeline. Short-circuit is on by default.
    pub fn new(name: impl Into<String>) -> Self;

    /// Append a gate.
    pub fn push(&mut self, gate: Box<dyn Gate>);

    /// Disable short-circuit: run all gates even if one fails.
    pub fn without_short_circuit(mut self) -> Self;
}

#[async_trait]
impl Gate for GatePipeline {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

The aggregate Verdict:
- `passed` is true only if ALL inner gates pass
- `score` is the minimum of all inner scores
- `detail` concatenates all inner details
- `duration_ms` is the sum of all inner durations

### Gate composition combinators

Beyond the sequential pipeline, three standalone combinators exist:

```rust
/// Run all gates concurrently. Fail if any fails. Score = min of inner scores.
pub struct ParallelGate { name: String, gates: Vec<Box<dyn Gate>> }

/// Run all gates. Require N-of-M to pass. Score = mean of passing scores.
pub struct VotingGate { name: String, gates: Vec<Box<dyn Gate>>, required: usize }

/// Try primary gate first. If it fails, try fallback. Score = first passing verdict.
pub struct FallbackGate { name: String, primary: Box<dyn Gate>, fallback: Box<dyn Gate> }
```

All three implement `Gate`, so they compose with the pipeline and with each other.

### Runtime rung dispatch

The `run_rung` function maps rung index to concrete gate and runs it:

```rust
pub async fn run_rung(
    base_signal: &Engram,
    ctx: &Context,
    rung: u32,
    inputs: &RungExecutionInputs,
    config: &RungExecutionConfig,
) -> Vec<Verdict>
```

For rung index > 6, it runs all canonical rungs in order. The orchestrator calls this per-task after agent dispatch.

### How the orchestrator invokes gates

In `crates/roko-cli/src/orchestrate.rs`, the gate invocation looks like this:

1. The orchestrator builds a `GatePayload` engram with the task's working directory.
2. It calls `selected_gate_steps(plan_id, exec_dir)` to determine which gates to run (based on complexity, caps, prior failures).
3. It constructs a `GatePipeline`, pushes each selected gate wrapped in a `RecordingGate` (which captures verdicts for persistence).
4. It calls `pipeline.verify(&payload_signal, &ctx).await` to get the aggregate verdict.
5. The recorded verdicts are stored as engrams and used for adaptive threshold updates, episode logging, and replan-on-failure triggers.

The key call site:

```rust
let mut pipeline = GatePipeline::new(format!("gate-pipeline:{plan_id}"));
for (rung, gate) in self.selected_gate_steps(plan_id, exec_dir) {
    pipeline.push(Box::new(RecordingGate::new(rung, gate, Arc::clone(&sink))));
}
let aggregate = pipeline.verify(payload_sig, &ctx).await;
```

### Adaptive thresholds

Per-rung pass rates are tracked via exponential moving averages (EMA). The `AdaptiveThresholds` struct:

- Tracks `ema_pass_rate`, `total_observations`, `consecutive_passes` per rung
- Uses CUSUM change-point detection to spot shifts in pass rates
- Suggests retry budgets based on history
- Can skip rungs after 20+ consecutive passes
- Persists to `.roko/learn/gate-thresholds.json`

```rust
pub struct AdaptiveThresholds {
    entries: HashMap<u32, RungHistory>,
}
```

Key methods:
- `observe(rung_index, passed)` -- record an observation
- `threshold_for(rung_index) -> f64` -- suggested pass threshold
- `suggest_retries(rung_index) -> u32` -- retry budget based on history
- `should_skip(rung_index) -> bool` -- skip if 20+ consecutive passes

### Gate ratchet

Once a plan has passed rung N, the ratchet prevents regression to rung N-1:

```rust
pub struct GateRatchet {
    /// Maps plan_id -> highest rung index that plan has passed.
    entries: HashMap<String, u32>,
}
```

The ratchet is checked before accepting a verdict. If an agent fixes a compile error but breaks lint, then fixes lint but breaks compile again, the ratchet makes the regression visible and blockable.

Persists to `.roko/state/ratchet.json`.

### Replan on failure

When `learning_config.replan_on_gate_failure` is enabled in the config, a gate failure triggers `build_gate_failure_plan_revision`. The orchestrator generates a revised plan incorporating the failure feedback (the error digest from the verdict), and the agent retries with that context.

This means gate verdicts with good `error_digest` and `detail` fields directly improve retry quality.

---

## 7. The orchestration loop

This is how plans execute. Understanding this is critical because a new gate plugs in here.

### Plans and tasks

A plan is a directory containing a `tasks.toml` file:

```toml
[[task]]
id = "T1"
title = "Implement login form"
description = "Create a React login form with email/password fields"
tier = "standard"
deps = []
files = ["src/Login.tsx"]
verify = ["cargo test"]
acceptance = [
    "Login form renders with email and password fields",
    "Form validates required fields",
]
```

Tasks execute in dependency order. The executor resolves the DAG and runs independent tasks in parallel.

### Agent dispatch per task

For each task:

1. The 9-layer `SystemPromptBuilder` assembles a prompt containing: role identity, task description, file context, tool definitions, safety constraints, prior episode context, playbook tips, efficiency guidance, and output format requirements.
2. The `CascadeRouter` selects a model (e.g., Claude Opus for complex tasks, Sonnet for simpler ones). The router learns from feedback -- models that produce passing gate results for certain task types are preferred.
3. The agent runs (typically via Claude CLI subprocess) and produces output.

### Gate verification per task

After the agent completes:

1. The rung selector determines which rungs to run based on plan complexity and prior failure count.
2. `GatePipeline` executes selected rungs in order (Compile -> Lint -> Test -> ...).
3. Each rung produces a `Verdict`.
4. If any rung fails and short-circuit is on, remaining rungs are skipped.
5. The aggregate verdict is returned.

**A visual gate fits here** -- as a new rung (index 7 or higher) that runs after the existing 7 rungs. It verifies that the UI actually works and looks right.

### Learning and persistence

After gate verification:

- The episode (agent turns + verdicts) is written to `.roko/episodes.jsonl`
- Efficiency events are logged to `.roko/learn/efficiency.jsonl`
- The CascadeRouter receives feedback (which model was used, did it pass)
- Adaptive thresholds are updated per rung
- If the gate failed and replan-on-failure is enabled, a revised plan is generated
- Artifacts are stored under `.roko/state/`

---

## 8. Writing a new gate

Here is the pattern. Every existing gate follows it.

### Step 1: implement the Gate trait

```rust
use async_trait::async_trait;
use roko_core::{Context, Engram, Gate, Verdict};

pub struct MyGate {
    // configuration fields
}

#[async_trait]
impl Gate for MyGate {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict {
        let started = std::time::Instant::now();

        // 1. Extract the GatePayload from the engram body
        let payload: GatePayload = match engram.body.as_json() {
            Some(json) => match serde_json::from_value(json.clone()) {
                Ok(p) => p,
                Err(e) => return Verdict::fail(self.name(), format!("bad payload: {e}"))
                    .with_duration(elapsed_ms(started)),
            },
            None => return Verdict::fail(self.name(), "missing gate payload")
                .with_duration(elapsed_ms(started)),
        };

        // 2. Do verification work (shell commands, HTTP calls, file checks, etc.)

        // 3. Return a verdict
        Verdict::pass(self.name())
            .with_score(0.95)
            .with_detail("verification output here")
            .with_duration(elapsed_ms(started))
    }

    fn name(&self) -> &str {
        "my_gate"
    }
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    started.elapsed().as_millis() as u64
}
```

### Step 2: register in the pipeline

The gate gets pushed into the `GatePipeline` in the orchestrator. For a new rung, this means adding it to `selected_gate_steps` in `orchestrate.rs` or extending `rung_dispatch.rs`.

### Step 3: re-export from roko-gate

Add the module and re-export from `crates/roko-gate/src/lib.rs`.

---

## 9. Existing code shape

The crate directory structure:

```
crates/
  roko-core/src/
    engram.rs             # Engram struct, builder, HdcFingerprint
    traits.rs             # 6 verb traits (Substrate, Scorer, Gate, Router, Composer, Policy)
    verdict.rs            # Verdict, Selection, Outcome, TestCount
    score.rs              # Score (7 axes), effective()
    decay.rs              # Decay enum (None, HalfLife, Ttl, Ebbinghaus)
    body.rs               # Body enum (Empty, Text, Json, Bytes)
    kind.rs               # Kind enum (non-exhaustive, 20+ variants)
    context.rs            # Context (now_ms, goal, attrs)
    provenance.rs         # Provenance (author, trust, taint)
    config/               # Config types, schema, compat
    lib.rs                # Re-exports

  roko-gate/src/
    compile.rs            # CompileGate (Rung 0)
    clippy_gate.rs        # ClippyGate (Rung 1 -- lint)
    test_gate.rs          # TestGate (Rung 2)
    symbol_gate.rs        # SymbolGate (Rung 3)
    generated_test_gate.rs # GeneratedTestGate (Rung 4)
    property_test_gate.rs # PropertyTestGate (Rung 5)
    integration_gate.rs   # IntegrationGate (Rung 6)
    llm_judge_gate.rs     # LlmJudgeGate (Rung 6, paired)
    fact_check.rs         # FactCheckGate (Rung 5, paired)
    gate_pipeline.rs      # GatePipeline (sequential composition of gates)
    composition.rs        # ParallelGate, VotingGate, FallbackGate
    adaptive_threshold.rs # EMA tracking, CUSUM, per-rung history
    rung_selector.rs      # Rung enum, PlanComplexity, RungCaps, select_rungs()
    rung_dispatch.rs      # run_rung() -- maps rung index to concrete gate
    ratchet.rs            # GateRatchet -- prevents rung regression
    payload.rs            # GatePayload, BuildSystem
    shell.rs              # ShellGate (generic shell command gate)
    diff_gate.rs          # DiffGate
    benchmark_gate.rs     # BenchmarkGate
    review_verdict.rs     # ReviewVerdict
    compile_errors.rs     # Compile error parsing
    verdict_publisher.rs  # VerdictPublisher
    spc.rs                # SPC (statistical process control) detectors
    lib.rs                # Re-exports

  roko-cli/src/
    orchestrate.rs        # The main orchestration loop (8000+ lines)
    vision_loop/          # Earlier prototype (screenshot capture, prompt construction, evaluation)
      mod.rs              # VisionLoopConfig, VisionIteration types
      screenshot.rs       # Screenshot capture
      prompt.rs           # Prompt construction for vision models
      checkpoint.rs       # Iteration checkpointing
      orchestrator.rs     # Vision loop orchestrator
      evaluator.rs        # Score evaluation

  roko-serve/src/
    routes/               # HTTP route handlers (~85 routes)

  roko-compose/src/
    system_prompt_builder.rs  # 9-layer prompt assembly
    templates/                # Role templates

  roko-learn/src/            # Episode, playbook, routing, experiments

  roko-agent/src/
    dispatcher/mod.rs        # Agent dispatch (Claude CLI, API, etc.)
```

The `vision_loop/` directory contains an earlier prototype with screenshot capture, prompt construction, and evaluation patterns. Some patterns may be reusable, but the visual gate design supersedes it.

---

## 10. Key file paths

All paths are absolute from the workspace root `/Users/will/dev/nunchi/roko/roko/`.

| What | Path |
|---|---|
| Workspace root | `/Users/will/dev/nunchi/roko/roko/` |
| All crates | `/Users/will/dev/nunchi/roko/roko/crates/` |
| Core types | `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/` |
| Gate implementations | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/` |
| Gate pipeline | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` |
| Rung selector | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/rung_selector.rs` |
| Rung dispatch | `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/rung_dispatch.rs` |
| Orchestration loop | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` |
| HTTP routes | `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/` |
| Vision loop prototype | `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/vision_loop/` |
| Executor snapshots | `/Users/will/dev/nunchi/roko/roko/.roko/state/` |
| Episode log | `/Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl` |
| Gate thresholds | `/Users/will/dev/nunchi/roko/roko/.roko/learn/gate-thresholds.json` |
| Ratchet state | `/Users/will/dev/nunchi/roko/roko/.roko/state/ratchet.json` |
