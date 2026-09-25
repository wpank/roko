# 05 -- Domain Profiles

> **Implementation status (2026-09):** Proposed. The profile concept is
> specified and the underlying extension points (roles, tools, gates,
> templates) are all live. The domain plugin enum (`DomainPlugin`) and
> lifecycle manifests exist in `crates/roko-agent/src/lifecycle.rs`.
> Full profile installation as a single TOML bundle is not yet wired.

---

## Profile Framing

Roko is domain-agnostic at the kernel level, but deployments are not.
Most real uses want a domain-shaped bundle that ships with the right
default roles, tools, gates, heuristics, and prompt templates from
day one.

A **domain profile** is that bundle. It is the installable unit that
wraps the lower-level extension points described in
`depth/05-agent/extensibility.md`:

- Tier 1: role prompts and task templates.
- Tier 2: profile metadata and defaults.
- Tier 3: tool registrations and handlers.
- Tier 4: native integrations or specialized execution paths.

The profile is the thing a team installs. The roles are the things the
profile uses.

---

## Shared Composition Rules

All profiles follow the same composition model:

1. **Roles are selected first**, then specialized by the profile's
   prompts and tool allowlists.
2. **Tools merge by union** when multiple profiles are installed.
3. **Gates stack** unless a gate is explicitly scoped to one profile.
4. **Heuristics coexist**; routing chooses the best fit for the
   current context.
5. **Profile collisions should be explicit.** If two profiles claim the
   same role name or tool id, the operator needs a visible resolution
   policy.
6. **Context should be structured**, not free-form. That is the job of
   `TypedContext`.

This makes profile composition additive instead of exclusive. A
deployment can start with a single profile and grow into multi-profile
operation without changing the kernel.

---

## Canonical Profiles

| Profile | Default roles | Core tools | Core gates | Memory shape |
|---|---|---|---|---|
| Coding | Researcher, Planner, Implementer, Reviewer, Tester | fs, git, language toolchains, code MCP | compile, unit, clippy, diff | episodes, playbooks, build history |
| Research | Researcher, Analyst, Explorer, Reviewer | web, PDF, citation manager, note tools | citation, factuality, novelty | paper claims, replication ledger |
| Blockchain | Architect, Implementer, Reviewer, Operator | RPC, signer, explorer, compiler, simulator | simulation, gas, invariant, approval | chain-of-custody, audit trail |
| Data/ML | Analyst, Implementer, Tester, Reviewer | SQL, notebooks, pandas/polars, profiling | schema, sample-check, metric regression | dataset fingerprints, lineage |
| Ops/SRE | Operator, Deployer, Monitor, Reviewer | kubectl, logs, metrics, runbooks, pager | dry-run, blast-radius, change-window | incident archive, runbook library |
| Writing | DocWriter, Researcher, Reviewer | corpus search, style guide, fact-check, citation | style, fact, tone, plagiarism | voice fingerprint, editorial archive |

### Coding

The coding profile is the default Roko shape today. It combines
implementation and review roles with build tooling and diff-oriented
gates. It should feel boringly reliable: fast iteration, strong test
feedback, and small-gate pressure on every turn.

`TypedContext` keys: `language`, `repo_root`, `file_set`, `last_gate`.

### Research

Tuned for evidence collection, citation quality, and claim tracking.
It prefers retrieval, note synthesis, and claim verification over
speculative writing.

`TypedContext` keys: `question`, `corpus`, `source_ids`, `claim_set`.
A claim that cannot be tied back to a source remains provisional until
the profile's citation gate resolves it.

### Blockchain

The highest-risk profile. It needs typed intent, a simulator, and
custody records for every action that can touch funds or consensus
state.

This is the clearest case for both shared primitives:
- `TypedContext` carries structured intent: chain, wallet, target,
  amount, gas ceiling, approval state.
- `Custody` records who authorized the action, what simulation was run,
  and what on-chain witness or receipt proved the outcome.

### Data / ML

Treats datasets and notebooks as first-class artifacts. The profile
knows how to inspect schema drift, sample slices, and metric deltas
before recommending a change.

`TypedContext` keys: `dataset`, `notebook`, `pipeline_stage`,
`target_metric`, `schema_version`.

### Ops / SRE

Prioritizes low-blast-radius action, dry-run discipline, and
explainable decision traces. Defaults to observation and advisory modes
unless the operator explicitly allows execution.

`Custody` is useful here even when the action is not
blockchain-related: incident commands, deploys, and remediation steps
should be traceable after the fact.

### Writing

Focuses on style, factuality, and editorial voice. Less about tool
breadth and more about consistent output quality.

`TypedContext` keys: `audience`, `publication_type`, `tone`,
`source_set`, `voice_target`.

---

## DomainPlugin Enum

The lifecycle module defines domain plugins activated during agent
creation:

```rust
// crates/roko-agent/src/lifecycle.rs

pub enum DomainPlugin {
    Chain(ChainConfig),
    Coding(CodingConfig),
    Research(ResearchConfig),
    Custom(CustomPluginConfig),
}

pub struct ChainConfig {
    pub network: String,       // e.g., "base", "base-sepolia"
    pub custody_mode: String,  // e.g., "delegation"
}

pub struct CodingConfig {
    pub workspace_path: String,
    pub language: Option<String>,
}

pub struct ResearchConfig {
    pub topic: Option<String>,
    pub citations_enabled: bool,
}

pub struct CustomPluginConfig {
    pub id: String,
    pub params: HashMap<String, String>,
}
```

---

## TypedContext

`TypedContext` is the structured situation record that domain profiles
share. It replaces ad hoc free-text task summaries whenever a domain
needs reliable matching on situation shape.

```rust
pub struct TypedContext {
    pub domain: Domain,
    pub fields: BTreeMap<ContextKey, ContextValue>,
}

pub enum ContextValue {
    String(String),
    Int(i64),
    Float(f64),
    Hash(EngramHash),
    Fingerprint(HdcVector),
    List(Vec<ContextValue>),
    Nested(BTreeMap<ContextKey, ContextValue>),
}
```

The important property is not the exact shape but that the profile can
declare and validate keys instead of inferring everything from prose.
This gives gates and heuristics a stable contract across domains.

---

## Custody

`Custody` is the chain-of-custody record that attaches accountability
to profile actions:

```rust
pub struct Custody {
    pub action: ActionHash,
    pub who: PrincipalId,
    pub when: Timestamp,
    pub why: Vec<HeuristicId>,
    pub how: Vec<ClaimId>,
    pub approved_by: Option<PrincipalId>,
    pub simulation: Option<SimulationHash>,
    pub result: Option<ResultHash>,
    pub witness: Option<ChainWitness>,
}
```

Every profile can use custody records, but the need is strongest where
actions have external consequences:

- Blockchain: transaction approval and witness receipts.
- Ops/SRE: deploys, rollbacks, incident remediation.
- Data/ML: lineage and reproducibility.
- Writing: editorial review and source provenance.

---

## Profile Installation

Profiles are installable bundles:

```toml
[profile.coding]
roles = ["researcher", "planner", "implementer", "reviewer"]
tools = ["fs.read", "fs.write", "git.status", "cargo.build"]
gates = ["unit", "type", "style", "diff"]
heuristics = "@roko/coding-heuristics-starter"
templates = "@roko/coding-templates"

[profile.research]
roles = ["researcher", "analyst", "explorer", "reviewer"]
tools = ["web.search", "pdf.extract", "citation.lookup"]
gates = ["citation", "factuality", "novelty"]
heuristics = "@roko/research-heuristics-starter"
templates = "@roko/research-templates"
```

The exact package format can evolve, but the contract should remain
stable: install a profile, get a domain-shaped agent stack.

---

## Evaluation Suites

Each profile should ship with a benchmark suite:

- **Coding:** bug-fix tasks with frozen SHAs and test outcomes.
- **Research:** claim-to-source matching and follow-up paper detection.
- **Blockchain:** vulnerable-contract detection and false-positive
  tracking.
- **Data/ML:** dirty-dataset diagnosis and metric-regression handling.
- **Ops/SRE:** simulated incidents and time-to-correct-diagnosis.
- **Writing:** style-fidelity checks against a known author corpus.

Results feed into the replication and learning layers so profile
quality improves over time.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-agent/src/lifecycle.rs` | DomainPlugin enum, ChainConfig, CodingConfig, ResearchConfig |
| `crates/roko-core/src/agent.rs` | AgentRole (28 variants) |
| `crates/roko-compose/src/templates/` | Role templates (11 templates) |
| `crates/roko-gate/` | Gate pipeline (19 gates) |

---

## Citations

1. `crates/roko-agent/src/lifecycle.rs` -- DomainPlugin enum, lifecycle
   manifests.
2. `crates/roko-core/src/agent.rs` -- AgentRole enum.
3. `crates/roko-compose/src/templates/` -- Role prompt templates.
