# Generalized Cybernetic Agent Architecture Gaps

**Purpose:** Identify where the current Roko/Nunchi architecture is too rigid,
too product-specific, too prompt-template-driven, or not yet generalized enough,
especially around roles, prompt systems, context assembly, context injection,
learning feedback, and self-improving cybernetic control.

**Created:** 2026-04-25

---

## Audit Coverage

This audit used a full-file ingestion pass across every file in the requested
documentation roots, then compared the documented target architecture against
the current Rust implementation paths for roles, prompt composition, context
assembly, config, and serve templates.

| Root | Files | Lines |
|------|------:|------:|
| `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/PRDs` | 78 | 54,732 |
| `/Users/will/dev/nunchi/roko/roko/tmp/architecture` | 22 | 12,173 |
| `/Users/will/dev/nunchi/roko/roko/docs` | 422 | 190,416 |
| `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd` | 25 | 13,720 |
| **Total** | **547** | **271,041** |

High-signal corpus clusters:

- Agents, roles, prompts, context, gates, HDC, dreams, conductor, auctions,
  active inference, foraging, stigmergy, dashboard projections, and feedback
  loops dominate the source docs.
- The docs repeatedly describe a compositional, domain-agnostic, self-improving
  scaffold: Engram/Bus/Substrate/Scorer/Gate/Router/Composer/Policy, active
  inference, VCG attention, HDC/VSA knowledge, stigmergic coordination, dreams,
  conductor loops, and dashboard lenses.
- The live code has many useful pieces, but a lot of decision-making is still
  encoded as Rust enum matches, static strings, fixed prompt sections, and
  orchestration-layer glue.

Primary code anchors inspected:

- `crates/roko-core/src/agent.rs`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/context_provider.rs`
- `crates/roko-compose/src/templates/common.rs`
- `crates/roko-compose/src/templates/implementer.rs`
- `crates/roko-compose/src/templates/reviewer.rs`
- `crates/roko-compose/src/templates/scribe.rs`
- `crates/roko-cli/src/prompting.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-serve/src/templates.rs`

---

## Executive Diagnosis

The current implementation is not "bad"; it already contains several strong
foundations: `PromptComposer`, `SystemPromptBuilder`, `ContextProvider`,
`ContextAssembler`, section-effectiveness tracking, skill/playbook injection,
PAD affect guidance, context tiers, and learning feedback hooks.

The architectural problem is that these pieces are still organized around a
compiled-in role/template model. The target docs describe adaptive cognitive
scaffolding, but the runtime often behaves like:

1. Choose an `AgentRole` enum variant.
2. Match that enum to a static identity string.
3. Match that enum to fixed backend/model/budget/tool defaults.
4. Assemble fixed prompt layers with optional learned nudges.
5. Inject context from an orchestrator-owned list of sources.
6. Send the resulting string to an agent backend.

That is much less general than the docs' own architecture. The system should
instead treat roles, prompts, context, tools, gates, and learning policies as
data-driven, observable, evolvable control surfaces.

The deepest mismatch:

- The docs say Roko should be a cybernetic scaffold whose policies evolve from
  evidence.
- The code still uses static personas and per-role constants as the organizing
  primitive.

---

## Gap Map

### G1. Role is a compiled enum, not a dynamic policy object

Current state:

- `AgentRole` in `roko-core/src/agent.rs` is a 28-variant enum.
- Backend, model tier, turn budget, tool permissions, labels, and indexes are
  all hardcoded via `match` arms.
- `RoleOverride` can override model/backend/effort/tools/budget/temperament, but
  it cannot override the role's prompt, context policy, learning policy,
  section policy, safety posture, or morphogenetic behavior.

Why this is too rigid:

- Adding a new role requires Rust changes.
- Roles are personas instead of evolvable control policies.
- The role taxonomy bakes in old Mori/Bardo/Golem assumptions.
- Domain profiles and archetypes in the docs want "role as composition", not
  "role as enum variant".

Target:

- Keep `AgentRole` only as a compatibility key for built-in roles.
- Introduce `RoleProfile` as the runtime primitive:

```rust
pub struct RoleProfile {
    pub id: String,
    pub display_name: String,
    pub purpose: String,
    pub capabilities: Vec<CapabilityRef>,
    pub prompt_policy: PromptPolicyRef,
    pub context_policy: ContextPolicyRef,
    pub tool_policy: ToolPolicyRef,
    pub gate_policy: GatePolicyRef,
    pub learning_policy: LearningPolicyRef,
    pub temperament: Temperament,
    pub morphogenesis: Option<MorphogenesisPolicy>,
}
```

Built-in roles become bundled manifests. User and network roles become loaded
profiles.

### G2. Role prompts are still hardcoded Rust strings

Current state:

- `role_identity_for()` in `roko-compose/src/role_prompts.rs` maps `AgentRole`
  to static template modules.
- `ImplementerTemplate`, `ReviewerTemplate`, `ScribeTemplate`, and others
  contain large static identity strings.
- `CONTEXT_LAYOUT_STANZA` is a static Rust string.
- Role prompt behavior requires recompilation unless it is injected as an extra
  convention or anti-pattern from the orchestration layer.

Why this is too rigid:

- Prompt text is not versioned as a policy artifact.
- Experiments can influence sections, but cannot naturally graduate into the
  source prompt profile.
- Role identity is not an Engram with provenance, lineage, outcomes, and
  measured lift.
- The system cannot publish, compare, import, or roll back prompt policies as
  first-class objects.

Target:

- Move built-in role identities into `RoleProfile` manifests.
- Move prompt layers into `PromptPolicy` manifests.
- Render prompts from a declarative prompt graph, not string constants.

```toml
[role]
id = "implementer"
purpose = "Produce production code and verification evidence"

[prompt_policy]
id = "builtin.implementer.v1"
layers = [
  "role_identity",
  "project_conventions",
  "capability_contract",
  "context_workspace",
  "task_contract",
  "tool_contract",
  "learned_techniques",
  "safety_constraints",
  "affect_guidance",
]

[[prompt_policy.layers]]
id = "learned_techniques"
source = "skill_library"
bidder = "playbook_rules"
priority = "normal"
max_tokens = 500
placement = "middle"
enabled_when = "task.has_skill_matches"
```

### G3. Roko has two prompt systems that do not share one abstraction

Current state:

- `roko-compose` has role prompt templates used by CLI/orchestration.
- `roko-serve` has `AgentTemplate` blueprints used for deployed/cloud workers.
- `roko-serve` built-ins contain independent system prompt strings.
- The serve templates explicitly warn they are "not to be confused with" the
  role prompt templates.

Why this is too rigid:

- Same conceptual primitive, two incompatible implementations.
- Prompt experiments and learning signals are not uniformly available across
  CLI plan execution and deployed agents.
- Dashboard agent creation is likely to create more template surface area
  instead of reusing the cognitive composition layer.

Target:

- Unify both under `AgentBlueprint`.

```rust
pub struct AgentBlueprint {
    pub id: String,
    pub archetype: ArchetypeManifest,
    pub role_profile: RoleProfileRef,
    pub domain_profile: DomainProfileRef,
    pub prompt_policy: PromptPolicyRef,
    pub context_policy: ContextPolicyRef,
    pub model_policy: ModelPolicyRef,
    pub tool_policy: ToolPolicyRef,
    pub lifecycle: AgentMode,
    pub triggers: Vec<TriggerSpec>,
}
```

`roko-serve::AgentTemplate` should become a thin persistence/API view over
`AgentBlueprint`, not a separate prompt engine.

### G4. Mori/Bardo/Golem residues still leak into live prompt semantics

Current state:

- `CONTEXT_LAYOUT_STANZA` tells agents `.mori/plans/` is the canonical plan
  artifact root.
- Integration templates reference `bardo-test-harness` and write reports to
  `.mori/plans/reviews`.
- Quick-fix templates write to `.mori/plans/completion`.
- Role docs and comments still cite Mori source files as if they are canonical.
- Test fixture names like `golem-mortality` appear throughout prompt template
  tests.

Why this matters:

- This is not just cosmetic. These strings can become live instructions in
  agent prompts.
- It creates path confusion against Roko's newer `plans/` and `.roko/` layout.
- It encourages more compatibility patches instead of defining a generic layout
  contract.

Target:

- Introduce `ArtifactLayoutProfile` loaded from `RokoLayout`.
- Prompt layers should render paths from the layout object.
- Legacy path aliases can exist only as migration aliases, never as canonical
  prompt instructions.

### G5. Context categories are not first-class enough

Current state:

- Docs define a `CognitiveWorkspace` and rich `ContextCategory` set.
- Live code mostly uses `PromptSection { name: String, content, priority,
  cache_layer, placement, bidder }`.
- `ContextProvider` uses `ContextSource`, but category, bidder, provenance,
  policy, diagnostics, and learning are not all unified in one workspace object.
- Some context is assembled in `ContextProvider`, some in `ContextAssembler`,
  some in `orchestrate.rs`, some in `SystemPromptBuilder`, and some in serve
  template rendering.

Why this is too rigid:

- Section names are stringly typed.
- Learning tracks section effectiveness, but it is not the canonical policy
  currency for all context sources.
- The prompt output is less auditable than the docs require: the "why included",
  "who bid", "who lost", "what displaced what", "what feedback updated what",
  and "what policy changed" trail is fragmented.

Target:

- Make `CognitiveWorkspace` real.
- Every LLM call receives a workspace object first, rendered to strings only at
  the backend boundary.

```rust
pub struct CognitiveWorkspace {
    pub invocation_id: InvocationId,
    pub role_profile: RoleProfileRef,
    pub domain_profile: DomainProfileRef,
    pub task: TaskRef,
    pub policy: ContextPolicyRef,
    pub sections: Vec<WorkspaceSection>,
    pub allocation: AllocationDiagnostics,
    pub assembly_log: Vec<AssemblyDecision>,
    pub feedback_hooks: Vec<FeedbackHookRef>,
}
```

### G6. ContextProvider hardcodes a tiered source sequence

Current state:

- `ContextTier` is `Surgical`, `Focused`, `Full`.
- `ContextProvider::resolve()` adds surgical, focused, then full sources in a
  fixed order.
- Budgets are static defaults: 4K, 12K, 24K.
- Full-tier context sources are hardcoded: plan brief, research, invariants,
  cross-plan context, decomposition.

Why this is too rigid:

- It improves over dumping everything into every prompt, but it is still a
  hand-authored pipeline.
- The docs describe bidders, active inference, foraging, and network context
  policies. The provider should be a market/controller, not a fixed pipeline.
- Extension hooks and domain profiles should be able to register sources and
  bidders without editing `ContextProvider`.

Target:

- Replace fixed source methods with a registry of `ContextBidder`s.

```rust
pub trait ContextBidder {
    fn id(&self) -> &BidderId;
    fn supports(&self, request: &ContextRequest) -> bool;
    async fn candidates(&self, request: &ContextRequest) -> Result<Vec<ContextCandidate>>;
    fn update(&mut self, feedback: &ContextFeedback);
}
```

The built-in tier policy becomes only a cold-start fallback.

### G7. VCG, active inference, and foraging are present but not the organizing path

Current state:

- `PromptComposer` has bidder-aware selection, PAD modulation, VCG-style
  payment diagnostics, optional foraging, and optional HDC dedup.
- `LearningBidder` exists, but docs note paths where learned bidders are not
  registered into production composition.
- `attention.auction_enabled` defaults false and is not the primary control
  plane for composition.
- VCG documentation is internally inconsistent: some docs say design/not yet,
  some status text says shipping, and code implements a partial live mechanism
  rather than the full PRD's bidder system.

Why this is too rigid:

- The algorithms are features inside a static scaffold rather than the scaffold
  itself.
- Current policy cannot cleanly choose between priority fallback, active
  inference, VCG, alpha-fairness, safety-floor hybrid, and foraging strategies
  from config or learning.
- Auction diagnostics do not yet become first-class dashboard/control-plane
  signals.

Target:

- Add `AttentionPolicy`:

```toml
[attention]
mode = "hybrid" # priority | active_inference | vcg | hybrid
fairness_alpha = 0.0
safety_floor_tokens = 256
foraging_enabled = true
hdc_dedup_threshold = 0.85
cold_start_min_trials = 20
publish_diagnostics = true
```

- Use active inference for scoring, VCG/fairness for allocation, MVT for source
  search stopping, and HDC for dedup/similarity.
- Persist all allocation decisions as Engrams.

### G8. Context injection is orchestrator-centric

Current state:

- `orchestrate.rs` gathers context from many systems and pushes prompt sections
  into the composer.
- This includes skills, playbooks, learned context, neuro chunks, external
  search, daimon state, enrichment artifacts, tool manifests, predictive
  calibration, C-factor, and more.
- This is powerful, but it makes the CLI orchestrator the coupling point for the
  whole cognitive architecture.

Why this is too rigid:

- The docs call for Bus/StateHub/projections and composable traits.
- Orchestration-layer injection makes it harder for persistent agents,
  dashboard copilots, WebMCP sessions, reactive agents, and remote agents to
  share one context assembly path.
- Adding a new source requires finding the correct place in a very large
  orchestrator file.

Target:

- Move context assembly into a `ContextEngine`.
- Orchestrator submits a `ContextRequest`; the engine queries registered
  bidders, policy, memory, tools, extensions, Bus topics, and dashboard state.
- CLI, serve, sidecar, copilot, persistent runtime, and scheduled agents all use
  the same engine.

### G9. Feedback loops exist, but policy evolution is not generalized enough

Current state:

- The learning docs describe eight loops: health to routing, conductor to
  routing, section to scaffold, failure to replanning, skills to prompts, cost
  to routing, latency to reward, experiments to static.
- Some loops are wired in the current code.
- However, loop outputs are not uniformly represented as policy updates against
  `RoleProfile`, `PromptPolicy`, `ContextPolicy`, `ToolPolicy`, and
  `GatePolicy`.

Why this is too rigid:

- Learning can nudge a section priority or model route, but the architecture
  lacks one place where "this feedback updated this policy" is recorded.
- Prompt experiments do not naturally graduate into versioned prompt manifests.
- Gate verdicts, reflections, dreams, and dashboard human actions do not all
  update the same policy substrate.

Target:

- Introduce a policy update ledger:

```rust
pub struct PolicyUpdate {
    pub policy_id: PolicyId,
    pub source_signal: EngramId,
    pub update_kind: PolicyUpdateKind,
    pub before_hash: ContentHash,
    pub after_hash: ContentHash,
    pub confidence: f64,
    pub rollback: Option<RollbackSpec>,
}
```

Every self-improvement loop should emit a policy update or a deliberate
no-op with reasons.

### G10. Knowledge/HDC/stigmergy are not canonical enough for roles and prompts

Current state:

- HDC, Neuro, dreams, stigmergy, pheromones/signals, and InsightStore concepts
  are richly documented.
- Context assembly can include neuro knowledge and pheromone chunks.
- But role selection, prompt policy, and context policy are still largely
  controlled by static role/task tier labels.

Why this is too rigid:

- The docs describe morphogenesis: specialization should emerge from knowledge
  gradients, pheromone fields, performance history, and demand.
- Current roles do not morph. They are selected, not evolved.
- HDC is a retrieval/support mechanism, not yet the common representational
  substrate for role fit, task fit, context value, and cross-domain transfer.

Target:

- Encode role profiles, tasks, policies, skills, gates, and outcomes into HDC
  fingerprints.
- Select and mutate roles by similarity, outcome, novelty, and demand.
- Allow a persistent agent to develop a specialization vector:

```rust
pub struct AgentMorphology {
    pub specialization: HdcVector,
    pub capability_distribution: Vec<(CapabilityRef, f64)>,
    pub domain_resonance: Vec<(DomainProfileRef, f64)>,
    pub recent_gradient: MorphologyGradient,
}
```

### G11. Tool permissions are static, not capability leases

Current state:

- `AgentRole::tool_permissions()` returns read/write/exec/git/network booleans.
- `RoleOverride.tools` can add a whitelist.
- Safety layer concerns exist, including docs noting Claude CLI bypass risks.

Why this is too rigid:

- Tools should be granted by task need, risk, provenance, current state, and
  gate posture.
- A role should not own permanent access just because of its name.
- The dashboard and chain docs need pre-action gates, custody, audit, scoped
  API keys, and visible capability state.

Target:

- Replace per-role permissions as the primary mechanism with capability leases:

```rust
pub struct CapabilityLease {
    pub capability: Capability,
    pub granted_to: AgentId,
    pub reason: EngramId,
    pub scope: CapabilityScope,
    pub expires_at: Option<Timestamp>,
    pub pre_action_gates: Vec<GateRef>,
    pub audit_policy: AuditPolicyRef,
}
```

Roles request leases; policy grants or denies them.

### G12. Config does not expose the right policy surfaces

Current state:

- `RokoConfig` has many useful sections: agent, routing, learning, attention,
  gates, tools, conductor, chain, relay, agents, etc.
- `[agent.roles.<name>]` supports overrides for model/backend/effort/temperament
  context window/tools/budget/thresholds/routing/turn budget.
- It does not expose declarative prompt policies, context policies, bidder
  registries, role profile manifests, artifact layouts, or policy evolution
  settings.

Why this is too rigid:

- The user can tune models and tools but cannot fully redefine what "architect",
  "implementer", or "scribe" means without code.
- The docs' extension/domain/archetype/recipe primitive vocabulary has nowhere
  clean to land for prompt/context behavior.

Target:

```toml
[profiles.role.implementer]
source = "builtin:implementer@1"
prompt_policy = "builtin:prompt.implementer@1"
context_policy = "builtin:context.code-focused@1"
tool_policy = "builtin:tools.code-writer@1"
learning_policy = "builtin:learning.self-hosting@1"

[prompt_policies.builtin.prompt.implementer.v1]
manifest = ".roko/policies/prompts/implementer.toml"

[context_policies.builtin.context.code-focused.v1]
mode = "hybrid"
sources = ["task", "code", "neuro", "skills", "gates", "signals"]
```

### G13. Dashboard primitives are richer than backend policy projections

Current state:

- Dashboard PRDs describe universal primitives, composition patterns, lenses,
  agent copilot context, WebMCP tools, and authoring surfaces.
- Backend code and plans expose many routes and projections, but prompt/context
  assembly decisions are not yet a first-class dashboard object.

Why this is too rigid:

- Operators cannot inspect why a prompt included a section, which bidder won,
  what was displaced, what feedback changed the policy, or how an agent's role
  morphology is evolving.
- Agent authoring risks becoming UI over static templates instead of UI over
  live cognitive policies.

Target dashboard objects:

- Role profile inspector.
- Prompt policy editor and experiment history.
- Context auction trace.
- Context source health and marginal utility.
- Gate verdict to policy update trace.
- Agent morphology and specialization vector.
- Capability lease/audit timeline.

### G14. Documentation statuses conflict with implementation reality

Current state:

- Several docs mark features as "Shipping" while their body says scaffold,
  design-only, or partially implemented.
- VCG is the clearest example: design docs say not implemented, current status
  docs say partially implemented, code contains bidder-aware greedy allocation
  and payment diagnostics but not the full bidder/policy architecture.
- Role docs refer to a 6-layer prompt model while current code and newer docs
  refer to 9 layers.

Why this matters:

- Agents reading these docs can make wrong implementation choices.
- Future plans may duplicate existing partial implementations or assume missing
  pieces are done.

Target:

- Add a docs parity rule: every "Shipping" claim must cite a code path and an
  acceptance gate.
- Split statuses into:
  - `specified`
  - `scaffolded`
  - `wired-partial`
  - `wired-production`
  - `verified`

---

## Target Architecture

### 1. RoleProfile instead of role persona

Roles should be runtime data objects with policies, capabilities, constraints,
history, and morphogenetic state.

`AgentRole` can remain as a built-in alias layer:

```rust
AgentRole::Implementer -> RoleProfileRef("builtin.implementer@1")
```

But no new domain role should require adding an enum variant.

### 2. AgentBlueprint as the common deployment object

CLI orchestration, cloud workers, dashboard-created agents, reactive agents,
and persistent agents should all instantiate the same blueprint type.

Blueprints compose:

- RoleProfile
- DomainProfile
- PromptPolicy
- ContextPolicy
- ModelPolicy
- ToolPolicy
- GatePolicy
- Lifecycle triggers
- Extension chain

### 3. PromptPolicy as a declarative graph

Prompt assembly should be a policy graph:

- layers
- conditions
- source refs
- transformations
- bidders
- budgets
- placements
- cache tiers
- experiments
- safety floors
- feedback hooks

This allows prompts to be tested, versioned, rolled back, published, and learned.

### 4. CognitiveWorkspace as the unit of context

The workspace is the object that gets assembled, audited, rendered, and learned
from. Strings are backend-specific renderings of the workspace.

Every workspace should preserve:

- sections included
- sections excluded
- source provenance
- bidder values
- allocation diagnostics
- token/cost impact
- gate outcomes
- policy updates

### 5. ContextEngine instead of orchestrator glue

The context engine owns:

- bidder discovery
- context source execution
- active inference scoring
- VCG/fairness allocation
- MVT foraging stopping
- HDC dedup
- cache alignment
- policy feedback
- diagnostics

The orchestrator should submit requests, not own the assembly algorithm.

### 6. Cybernetic Controller as the operating loop

The general loop should be:

```text
Observe -> Predict -> Allocate -> Act -> Verify -> Update -> Consolidate
```

Mapped to Roko primitives:

- Observe: Bus events, feeds, connectors, dashboard state, repo state
- Predict: model routing, gate risk, outcome forecasts, affect appraisal
- Allocate: context auction, tool leases, model choice, budget/latency tradeoff
- Act: agent execution, tool calls, chain calls, file edits
- Verify: gates, evals, witness DAG, policy checks
- Update: section effectiveness, bidder posteriors, role morphology, playbooks
- Consolidate: dreams, HDC knowledge, InsightStore publication

### 7. Dynamic role morphogenesis

Roles should emerge and specialize from:

- task distribution
- success/failure gradients
- HDC similarity to prior work
- pheromone/signal fields
- capability lease history
- domain resonance
- gate risk
- human dashboard feedback

Examples:

- An `implementer` that repeatedly succeeds on config/schema work develops a
  config-specialist specialization vector.
- A `scribe` whose docs repeatedly fail citation gates receives more PRD/source
  context and stricter citation gates.
- A `researcher` whose findings are repeatedly validated by gates gains higher
  bid priors in research context allocation.

---

## Migration Plan

### P0. Remove live Mori path leakage

Scope:

- Replace `.mori/plans` prompt instructions with layout-rendered `.roko`/`plans`
  paths.
- Replace `bardo-test-harness` live instructions with Roko-owned test harness
  language.
- Move old Mori/Bardo/Golem references in comments to migration notes where they
  are not runtime instructions.

Acceptance:

- `rg -n "\\.mori|bardo-test-harness|mori↔bardo|mori-local-gateway" crates`
  has no live prompt/runtime instruction hits.
- Existing tests updated to assert Roko layout, not Mori layout.

### P1. Add RoleProfile and PromptPolicy manifests

Scope:

- Add `RoleProfile`, `PromptPolicy`, and manifest loaders.
- Export current built-in roles into bundled manifests.
- Keep `AgentRole` mapping to built-ins for compatibility.
- Add config keys for profile/policy selection.

Acceptance:

- Existing role prompts render byte-equivalent output through manifests for the
  default profiles.
- A workspace can override `implementer` role identity without recompiling.

### P2. Unify `roko-compose` role prompts and `roko-serve` templates

Scope:

- Introduce `AgentBlueprint`.
- Make `roko-serve::AgentTemplate` load/save blueprint-compatible manifests.
- Replace serve built-in prompt strings with references to prompt policies.

Acceptance:

- A dashboard-created agent and a CLI-dispatched plan agent can use the same
  role profile and prompt policy.
- Prompt experiments and section-effectiveness signals apply to both paths.

### P3. Implement `CognitiveWorkspace`

Scope:

- Add `WorkspaceSection`, `ContextCategory`, `AssemblyDecision`,
  `AllocationDiagnostics`, and `ContextFeedback`.
- Convert current `PromptSection` into a render-layer view or embedded field.
- Emit one workspace artifact per agent invocation.

Acceptance:

- Every agent dispatch records included and dropped sections with provenance.
- Gate outcome feedback references the workspace id.

### P4. Turn context assembly into a bidder registry

Scope:

- Add `ContextBidder` trait.
- Convert current built-in sources to bidders:
  - task
  - code intelligence
  - neuro
  - skills/playbooks
  - research
  - daimon
  - gates/oracles
  - signals/pheromones
  - dashboard/WebMCP
  - extensions
- Move fixed tier logic into cold-start `ContextPolicy`.

Acceptance:

- Adding a new context source does not require editing `ContextProvider::resolve`.
- Bidders can be enabled/disabled by domain, role profile, or runtime policy.

### P5. Make attention policy real

Scope:

- Wire config to select priority, active inference, VCG, or hybrid allocation.
- Register persisted `LearningBidder` posteriors in production composers.
- Persist auction diagnostics and section payments.
- Add safety floor / alpha-fairness controls.

Acceptance:

- `attention.auction_enabled = true` materially changes allocation.
- Dashboard/API can show bidder winners, payments, displaced sections, and
  policy updates.
- Section-effectiveness and bidder updates feed the next dispatch.

### P6. Move context injection out of `orchestrate.rs`

Scope:

- Add `ContextEngine`.
- Let `orchestrate.rs`, serve workers, sidecars, reactive agents, and copilot
  sessions call the same API.
- Move source-specific assembly out of orchestration glue.

Acceptance:

- Plan run, serve template execution, and copilot chat all produce
  `CognitiveWorkspace` records through the same engine.

### P7. Replace static tool permissions with capability leases

Scope:

- Add `CapabilityLease`.
- Make role permissions defaults only.
- Gate high-risk leases with pre-action gates.
- Ensure Claude CLI path has equivalent safety enforcement or explicit bounded
  tool allowlist enforcement.

Acceptance:

- Tool access is explainable by lease, scope, reason, and gate status.
- A role can receive temporary elevated access for one task without changing its
  profile.

### P8. Add role morphology and HDC policy representations

Scope:

- Encode tasks, roles, prompt policies, context policies, skills, and outcomes
  into HDC fingerprints.
- Add agent specialization vectors.
- Update vectors from success/failure and knowledge gradients.
- Use vectors to select, mutate, or synthesize role profiles.

Acceptance:

- Role selection can rank dynamic profiles by task similarity and outcome
  history.
- A persistent agent's specialization changes over time and is visible in state.

### P9. Dashboard and WebMCP policy surfaces

Scope:

- Add projections for role profiles, prompt policies, context workspaces,
  auction traces, tool leases, policy updates, and morphology.
- Let WebMCP expose page context as a context bidder rather than a bespoke chat
  injection.

Acceptance:

- The dashboard can answer "why did this agent see this context?"
- The dashboard can edit a prompt/context policy and run an experiment against
  it.

### P10. Docs parity and acceptance gates

Scope:

- Create strict status gates for prompt/context claims.
- Add parity rows for role/prompt/context architecture.
- Resolve conflicting "Shipping" vs "Designed" statuses.

Acceptance:

- No prompt/context doc can say `Shipping` without a source file, runtime path,
  and acceptance command.

---

## Anti-Patterns To Avoid

- Do not add more enum variants for every new domain role.
- Do not add a third prompt/template system.
- Do not solve dynamic context by adding more fixed sections to
  `orchestrate.rs`.
- Do not make dashboard agent creation save raw prompt strings as the long-term
  primitive.
- Do not treat VCG, active inference, HDC, dreams, or stigmergy as isolated
  feature flags. They should become policy mechanisms inside one control loop.
- Do not let legacy Mori/Bardo/Golem paths remain in live prompt instructions.
- Do not mark documentation "Shipping" unless the production path and gate are
  named.

---

## Concrete First Implementation Packet

If this plan is turned into code work, start with the smallest slice that
changes architecture direction without destabilizing the orchestrator:

1. Add `RoleProfile` and `PromptPolicy` structs plus TOML manifest loading.
2. Export built-in `implementer`, `architect`, `scribe`, `critic`, `researcher`,
   and `conductor` profiles from the current hardcoded strings.
3. Render the current default prompts from those manifests.
4. Replace live Mori layout text with a `RokoLayout`-rendered artifact layout
   stanza.
5. Add one workspace-level override:

```toml
[agent.roles.implementer]
profile = ".roko/roles/implementer.toml"
prompt_policy = ".roko/policies/prompts/implementer.toml"
```

6. Add tests proving:
   - defaults still render;
   - workspace manifest override changes role identity;
   - invalid prompt policy fails validation;
   - no `.mori/plans` text appears in generated prompts.

That first packet creates the escape hatch from hardcoded role prompts. The
larger cognitive loops can then target policy manifests instead of Rust string
constants.

---

## Decision Summary

The architecture should move from:

```text
role enum -> static prompt string -> fixed context sources -> final prompt
```

to:

```text
agent blueprint
  -> role/domain/prompt/context/tool/gate policies
  -> context bidder registry
  -> active-inference/VCG allocation
  -> CognitiveWorkspace
  -> agent execution
  -> gates/outcomes
  -> policy updates
  -> dreams/knowledge/morphogenesis
```

This is the difference between a hardcoded multi-agent harness and the
cybernetic, self-improving scaffold described across the Roko and Nunchi docs.
