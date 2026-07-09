# Domain-Specific Agents

> **TL;DR**: Roko is a general-purpose agent toolkit, but most
> deployments will be domain-specific: coding agents, research
> agents, blockchain agents, data-engineering agents, ops agents.
> Each domain reuses ~80% of the kernel and customizes the
> remaining ~20% via roles, tools, gates, heuristics, and
> composer templates. This doc proposes six canonical domain
> profiles, shows what each needs from the kernel, and identifies
> two new subsystems (typed-context and chain-of-custody) that
> would unlock domains that are currently awkward.

> **For first-time readers**: A "profile" is a plugin bundle that wraps
> tier-1/2/3/4 extensions (see 17) for a specific domain. Users install
> a profile — coding, research, blockchain, data, ops, writing — and
> get a coherent starting point: tools, roles, gates, starter
> heuristics, composer templates. Two primitives this doc requires from
> the kernel are **TypedContext** (structured situation data) and
> **Custody** (chain-of-custody records for auditable actions). Read 17
> first for the plugin story; 14 for the heuristic story; 11 for HDC.

## 1. The domain matrix

| Domain | Core tools | Key gates | Heuristic sources | Memory shape |
|---|---|---|---|---|
| **Coding** | fs, cargo/npm/etc, git, mcp-code | unit, compile, clippy, diff | test outcomes, PR reviews | episodes + playbooks |
| **Research** | web, arxiv, pdf, note-taking | citation-check, factuality | paper claims, prior searches | dense heuristic library |
| **Blockchain** | rpc, signer, explorer, compiler | simulation, gas, invariant | historical exploits, docs | immutable audit trail |
| **Data/ML** | sql, pandas, jupyter, notebooks | schema, sample-check, metric | metric regressions | dataset fingerprints |
| **Ops/SRE** | kubectl, logs, metrics, runbook | dry-run, blast-radius | incident postmortems | runbook library |
| **Writing** | corpus, dictionary, style, citation | style, fact, tone | editorial feedback | voice fingerprint |

The kernel is the same. What differs:

- Which tools are present.
- Which gates are wired into the pipeline.
- Which heuristics are seeded.
- Which roles and templates are default.
- Which substrate features (e.g., chain-of-custody for blockchain)
  are enabled.

Plugin tiers from `17` already support this. Domain profiles are
plugin bundles — a curated set of Tier-1 (prompts), Tier-2
(profile), Tier-3 (tools), and Tier-4 (native) extensions that
ship together.

## 2. Coding agent (the default)

What Roko is best at today. Domain-specific reinforcements:

- **Tools**: file-system, version control, language-specific build
  systems (cargo/npm/pip/go). All exist.
- **Gates**: compile, unit, integration, clippy/lint, diff. All
  wired.
- **Heuristics**: from `14` §4 starter kit — flaky-test-logging,
  lockfile-on-merge-failure, etc. Starter library of ~30.
- **Roles**: researcher, planner, implementer, reviewer. Exist.

Gaps:

- **Code-graph awareness**: `roko-mcp-code` provides some; deeper
  integration with language servers would let the agent navigate
  semantically (rename-refactor safely, etc.).
- **Dependency-aware suggestions**: when the agent touches code,
  suggest updating callers; today it relies on the LLM noticing.

## 3. Research agent

Has partial support (`roko research *` subcommands). Expansions:

- **Tools**: arxiv API, Semantic Scholar, Papers With Code,
  Google Scholar, PDF extraction (use the pdf skill), citation
  manager integration (Zotero/BibTeX).
- **Gates**: citation-check (every claim cites a source),
  factuality (cross-check against fresh retrievals), novelty
  (not a duplicate of existing lit).
- **Heuristics**: seeded from `16-research-to-runtime.md` starter
  kit.
- **Memory**: Paper + Claim Engrams with Replication Ledger. Search
  is HDC-similarity rather than keyword.
- **Output modes**: literature review, annotated bibliography,
  research plan, replication report.

New subsystem need: **verified citations**. A citation is valid if
it points to a resolvable source and the quoted text actually
appears. A `CitationGate` that checks this should ship as part of
the research profile.

## 4. Blockchain agent

Least-supported today but high-leverage because mistakes are
catastrophic and audit trail is legally useful.

- **Tools**: Ethereum/Solana RPC, contract compiler (solc,
  anchor), block explorer, signer (with hardware-key support),
  simulator (tenderly, anvil), static analysis (slither, mythril).
- **Gates**:
  - **Simulation gate**: every proposed on-chain action is
    dry-run first; proceed only if the simulation succeeds *and*
    matches an explicit user-approved intent fingerprint.
  - **Gas gate**: proposed gas cost under a budget.
  - **Invariant gate**: contract invariants (maintained by a
    pluggable checker) still hold after the action.
  - **Blast-radius gate**: if the action touches funds above a
    threshold, require human approval.
- **Heuristics**: seeded from historical exploits (reentrancy
  patterns, integer overflows, missing access controls).
  Replication ledger against published audits.
- **Memory**: chain-of-custody — every transaction is an Engram
  with a witness (the actual on-chain receipt). Phase 2+
  `roko-chain` is purpose-built for this.

New subsystem need: **typed intents**. The user expresses intent
in typed form ("send N tokens from A to B with max gas G"); the
agent produces a transaction; the simulation verifies the
transaction matches the intent; then it's signed. Typed-intent
verification is a gate that blockchain domain fundamentally
requires.

## 5. Data / ML agent

- **Tools**: SQL (typed via sqlx-like introspection), pandas/polars,
  Jupyter kernel, notebook renderer, plotting, data
  profiling (great_expectations-style).
- **Gates**:
  - **Schema gate**: query doesn't violate the known schema.
  - **Sample-check gate**: materialize a small sample; look at
    distribution; reject if it's out of expected bounds.
  - **Metric-regression gate**: proposed change to a training
    pipeline must not regress key metrics beyond a threshold.
- **Heuristics**: from data-engineering best practices (nullable
  columns catch you, timezone drift is real, CSV encoding
  matters).
- **Memory**: dataset fingerprints (HDC encoding of schema +
  distribution summary), lineage between derived tables.

New need: **notebook-first workflow**. Roko should be able to
author, execute, and inspect Jupyter notebooks as first-class
artifacts. Notebooks can be Engrams; cells can be Pulses.

## 6. Ops / SRE agent

High-risk because mistakes affect running systems.

- **Tools**: kubectl (namespaced, dry-run default), logs (Loki,
  Elastic, CloudWatch), metrics (Prometheus), runbook retrieval,
  pager (PagerDuty, Opsgenie).
- **Gates**:
  - **Dry-run gate**: every action is dry-run; proceed only if
    diff is within expected scope.
  - **Blast-radius gate**: number of nodes/pods affected is under
    threshold without human approval.
  - **Change-window gate**: actions outside approved windows
    require override.
- **Heuristics**: from postmortems — the "we've been here before"
  database. Postmortems as Paper Engrams.
- **Memory**: incident archives, runbook executions, pattern
  library.
- **Modes**:
  - **Observer** (read-only, proposes fixes).
  - **Advisor** (proposes steps, waits for human).
  - **Executor** (acts, with guardrails).

New need: **explainable actions**. Every ops action should carry
a human-readable justification trace that's auditable after the
fact. Tie to `14-worldview-validation.md` (heuristic provenance)
and `16-research-to-runtime.md` (claim provenance).

## 7. Writing / content agent

- **Tools**: corpus search, style guide lookup, fact-check,
  citation manager, grammar/style (write-good, vale).
- **Gates**: style, fact, tone, plagiarism, length.
- **Heuristics**: from editorial feedback — "passive voice here,
  action here", "this paragraph introduces a new concept without
  defining it".
- **Memory**: *voice fingerprint* — an HDC encoding of the
  author's style, learned from their prior writing. Used to gate
  whether a generated draft "sounds like them."

New need: **stylistic fingerprinting**. An HDC encoder that takes
a text corpus and produces a fingerprint characterizing voice.
Drafts with fingerprint far from the author's get flagged.

## 8. Cross-domain shared subsystems

Looking at the domains side-by-side, two patterns recur:

### 8.1 Typed context

Every domain wants to express "the situation" in a structured way
so gates and heuristics can match on it. Today situations are
mostly free-text episode summaries. A `TypedContext` primitive:

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

Each domain profile declares its keys (e.g., coding declares
`language`, `repo_root`, `file_set`; blockchain declares `chain`,
`wallet`, `intent`). Gates and heuristics match on typed
predicates rather than string parsing.

This is the missing data primitive that holds domains together.

### 8.2 Chain of custody

Every domain has actions with consequences that should be auditable:
blockchain transactions, ops deploys, data pipeline changes,
published writing. A common `Custody` record:

```rust
pub struct Custody {
    pub action: ActionHash,
    pub who: PrincipalId,
    pub when: Timestamp,
    pub why: Vec<HeuristicId>,     // which heuristics influenced
    pub how: Vec<ClaimId>,         // which claims backed them
    pub approved_by: Option<PrincipalId>,
    pub simulation: Option<SimulationHash>,
    pub result: Option<ResultHash>,
    pub witness: Option<ChainWitness>,  // Phase 2+
}
```

Every domain benefits. Ops teams need it for compliance; blockchain
agents need it for dispute resolution; data teams need it for
lineage; writing needs it for editorial review. Shipping this
once, in `roko-core`, pays off in every domain.

## 9. Domain profiles as installable bundles

Following `17`, each domain is a *profile bundle*:

```
roko plugin install @roko/coding-profile
roko plugin install @roko/research-profile
roko plugin install @roko/blockchain-profile
# ...
```

A bundle is a tier-2 profile wrapping tier-1/3/4 extensions:

```toml
# @roko/coding-profile/profile.toml
name = "coding"
description = "Default coding agent profile"

tools = [
  "fs.read", "fs.write",
  "git.status", "git.diff", "git.commit",
  "cargo.build", "cargo.test", "cargo.clippy",
  "mcp-code.*",
]

roles = ["researcher", "planner", "implementer", "reviewer"]

gates = [
  { rung = "unit", id = "cargo.test" },
  { rung = "type", id = "cargo.check" },
  { rung = "style", id = "cargo.clippy" },
  { rung = "diff",  id = "roko.diff_gate" },
]

heuristics = "@roko/coding-heuristics-starter"
templates  = "@roko/coding-templates"
```

Users install a profile and get a coherent experience. Power users
customize by overriding specific tools/gates/heuristics while
keeping the rest of the profile intact. Profiles are themselves
versioned and can depend on minimum core versions.

## 10. Domain composition

The interesting case: **one project uses multiple domains**.
A blockchain startup's Roko instance might need both coding and
blockchain domains. Composition rules:

- **Tools merge**: union of tools from all installed profiles.
- **Roles merge**: union; if two profiles define the same role
  name, a collision warning fires.
- **Gates stack**: all gates from all profiles run on all tasks
  unless scoped. Scoping: `gates only in profile=<name>`.
- **Heuristics coexist**: all heuristics are available; routing
  picks based on HDC similarity of the situation.

This lets a team add a new domain without disrupting existing
workflow — it's additive until you explicitly wire it in.

## 11. The "agent for X" template

Community contribution pattern: someone builds an agent for their
domain (legal, medicine, accounting, infosec), packages it as a
profile bundle, publishes to the registry. Each published domain
benefits from the shared kernel AND from what the commons has
learned in adjacent domains.

This is where the Metcalfe's-law effect from `18` materializes:
every new domain profile expands what Roko can do, and every
domain profile benefits from the shared substrate.

## 12. What to ship first for domains

Priority:

1. **`TypedContext`** primitive in `roko-core`. Unblocks
   everything. One week.
2. **`Custody`** record + simple ops integration. Two weeks.
3. **Coding profile formalization**: most work already exists,
   package as a bundle. Three days.
4. **Research profile**: build on existing `roko research *`.
   Add citation-gate, paper-claim-heuristic integration. One week.
5. **Blockchain profile**: typed intents + simulation gate + chain
   witness scaffolding. Two weeks (partly Phase 2).
6. **Ops profile**: dry-run gate + blast-radius + postmortems as
   memory. Two weeks.
7. **Data / ML profile**: notebook support + schema gate. Two
   weeks.
8. **Writing profile**: voice fingerprint + style gates. One week.

Total about 2-3 months. At the end Roko has credible support for
six domains and a pattern for adding more.

## 13. Domain-specific evaluation suites

Each profile ships with a benchmark suite whose results go into the
replication ledger (16). Examples:

- **Coding**: a set of known bugs in OSS repos with frozen SHAs.
  Agent scores = time-to-green + token count + correctness.
- **Research**: a curated set of arxiv abstracts with known follow-up
  papers. Agent proposes follow-ups; match rate is the score.
- **Blockchain**: a set of known vulnerable contracts. Agent detection
  rate and false-positive rate are the scores.
- **Data**: a set of dirty datasets with known issues (type
  mismatches, duplicates, outliers). Agent's diagnosis report is
  compared to ground truth.
- **Ops**: a set of simulated incidents with known root causes. Time
  to correct diagnosis is the score.
- **Writing**: a set of style-fingerprinted corpora. Draft fidelity
  to target voice is the score (measured by HDC similarity to
  author's fingerprint, per §7).

These suites *also* serve as the load test for superlinear scaling
(15 §12): run them periodically, measure slope of score vs deployment
age. A flat slope signals a broken feedback loop.

## 14. Concrete starter heuristics per domain

Beyond the structural items in §2–§7, each profile ships a small
starter heuristic library. Illustrative examples (format follows 14 §2):

**Coding**:
- `h.code.001` — "When unit tests pass locally but fail in CI, check
  for dependency-version drift first." Precondition: `GateRecentlyFailed(unit, intermittent=true)`.
- `h.code.002` — "When a compile error mentions a trait bound, the
  next action is usually adding `impl` or adjusting generics, not
  modifying the trait itself."
- `h.code.003` — "Before bumping a dep in Cargo.toml, run `cargo tree -d`
  to check what else depends on it."

**Research**:
- `h.research.001` — "Claim whose effect size is reported without CI
  in the abstract is a replication risk; flag for falsifier
  sharpening."
- `h.research.002` — "If a paper cites its own preprint as the
  replication, treat it as untested."

**Blockchain**:
- `h.chain.001` — "A contract that modifies state before external
  call — probably reentrant. Run slither before signing."
- `h.chain.002` — "Gas estimation from mainnet node differs from
  fork; reject proposal if gap > 15%."

**Ops**:
- `h.ops.001` — "When error rate spikes but latency is flat,
  upstream is likely the cause. Check there before touching
  anything."
- `h.ops.002` — "Rolling restart before the change window ends:
  don't."

**Writing**:
- `h.write.001` — "When a draft's HDC fingerprint distance to the
  author's voice exceeds 0.35, the tone probably slipped."

**Data**:
- `h.data.001` — "If a column's null rate doubles overnight,
  upstream schema change is most likely. Check the source system's
  changelog before touching the loader."

These aren't exhaustive — they're seeds. Each profile ships with
~20-30 heuristics. Users' own deployments calibrate them and grow
new ones organically via `/learn` from episodes (28 §4).

## 15. Profile composition at runtime

Installing two profiles simultaneously (coding + blockchain, say)
needs explicit conflict resolution. Proposed rules (formalizing
§10):

```toml
# When merging profiles, conflicts resolve via profile priority.
[profile_resolution]
# Lower priority loses on key conflicts.
order = ["coding", "blockchain", "research"]

# Gates cumulate unless explicitly scoped.
gate_mode = "cumulate"    # or "scope_by_task_tag"

# Tools merge; duplicates (same id) use the first declaring profile.
tool_mode = "first_wins"

# Heuristics coexist; routing picks by HDC fit.
heuristic_mode = "coexist"

# Role prompts: collision warning if same name declared twice.
role_mode = "warn_on_collision"
```

A `roko profile check` command validates composition before use,
reports conflicts, and offers resolutions. CI-friendly exit codes
let CD pipelines catch bad profile combinations early.

## 16. Second TypedContext example — blockchain

To show the TypedContext primitive from §8.1 across domains:

```rust
// Coding TypedContext
TypedContext {
    domain: Domain::Coding,
    fields: [
        (key!("language"),   ContextValue::String("Rust".into())),
        (key!("repo_root"),  ContextValue::String("/workspace".into())),
        (key!("file_set"),   ContextValue::List(vec![
            ContextValue::String("src/lib.rs".into()),
            ContextValue::String("src/core.rs".into()),
        ])),
        (key!("last_gate"),  ContextValue::String("compile:fail".into())),
    ].into_iter().collect(),
}

// Blockchain TypedContext
TypedContext {
    domain: Domain::Blockchain,
    fields: [
        (key!("chain"),      ContextValue::String("ethereum-mainnet".into())),
        (key!("wallet"),     ContextValue::String("0xABC...".into())),
        (key!("intent"),     ContextValue::Nested([
            (key!("action"), ContextValue::String("transfer".into())),
            (key!("to"),     ContextValue::String("0xDEF...".into())),
            (key!("amount"), ContextValue::String("1.5 ETH".into())),
            (key!("max_gas"), ContextValue::Int(200_000)),
        ].into_iter().collect())),
        (key!("simulation"), ContextValue::Hash(sim_hash)),
    ].into_iter().collect(),
}
```

The same data shape serves both domains. Gates and heuristics match
against typed keys; no domain has to parse free-text situation
descriptions. Third-party domains register their own key schemas
and share the same match infrastructure.

## 17. Cross-references

- Plugin SPI that domains ride on: `17-plugin-extension-architecture.md`.
- Starter heuristic templates: `14-worldview-validation.md` §4.
- Replication of domain claims: `16-research-to-runtime.md` §8.
- Custody / safety spine: `32-safety-sandbox-provenance.md` §5.
- Deployment considerations for domain-specific setups:
  `24-deployment-ux.md` §2.
- Observability per domain:
  `33-observability-telemetry.md` §5.
