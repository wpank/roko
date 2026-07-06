# GAP-REPORT-V3 — Coverage gaps in the status-quo pack

Scope: cross-checked docs/v1 sections 08/13/14/15/16/17 against the 107-doc status-quo pack
(tmp/status-quo/00–106) and current code (HEAD 5852c93c05), plus a workspace-wide scan for
sizable modules (>500 LOC) whose module name gets **zero hits** anywhere in the pack.

Legend: **RCU** = real code, undocumented (belongs in a status-quo doc). **SO** = spec-only, no
code (belongs to spec-debt ledger 102). Severity P1 (wired + material) … P3 (dead/minor).

---

## A. Genuine gaps — real code, no dedicated pack coverage (RCU)

| # | Concept | Code (file · LOC) | Wired? | Should live in | Sev | Fix |
|---|---|---|---|---|---|---|
| 1 | **Role/Prompt policy manifest contracts** — `RolePolicyManifest`, `RoleProfile`, `PromptPolicy`, `ToolCapabilityPolicy`, `RoleSafetyPolicy`, `GateExpectation`, `PromptBudgetPolicy`. Runtime-agnostic TOML contract layer that feeds the prompt builder + role tools. | `roko-core/src/policy_manifest.rs` (1,364) | **YES** — consumed by roko-compose (`role_prompts.rs`, `context_provider.rs`, `cognitive_workspace.rs`) and roko-agent (`metamorphosis.rs`, `tests/role_tools.rs`) | 34-COMPOSE-PROMPTS (or 30-CORE-SIGNAL) | **P1** | New section/2 paragraphs. Biggest single blind spot: 1.4K wired LOC, 0 hits. |
| 2 | **Perplexity deep-research agent** — multi-step research/citation pipeline (`DeepResearch*`). | `roko-agent/src/perplexity/deep_research.rs` (638) | YES — `roko research`, perplexity adapter/provider | 91-PRD-RESEARCH (or 38) | P2 | Doc paragraph; research is covered generically but this module is unnamed. |
| 3 | **Context-pack cache** — caches assembled context packs for the learning/runtime-feedback loop. | `roko-learn/src/context_pack_cache.rs` (613) | YES — `roko-learn/runtime_feedback.rs` | 40-LEARNING-TELEMETRY | P2 | Doc paragraph. |
| 4 | **Task metric / benchmark model** (`TaskMetric`) — per-task metric records used by baseline/regression/bench. | `roko-learn/src/task_metric.rs` (698) | YES — baseline, regression, runtime_feedback, `roko bench` | 40-LEARNING or 74-TEST-AND-PROOF | P2 | Doc paragraph. |
| 5 | **Native plan-generation path** — `plan_generate.rs` (distinct from orchestrate/prd-plan). | `roko-cli/src/plan_generate.rs` (655) | YES — CLI | 36-ORCHESTRATION or 45-CLI | P3 | One line; concept covered, module unnamed. |

## B. Depth gaps — code covered only as a one-line "NOT WIRED" row (RCU, thin)

| # | Concept | Code (file · LOC) | Current coverage | Should deepen | Sev |
|---|---|---|---|---|---|
| 6 | **16 T0 probes** (v1/16-09) + **attention auction/gating** (v1/16-12) — ~3.7K LOC of the dormant heartbeat cognitive loop. | `roko-runtime/heartbeat_probes.rs` (1,545), `heartbeat_attention.rs` (2,146) | 90-RUNTIME lists them in one `STATUS: NOT WIRED` row; 89-PRIMITIVES covers tier/frequency mapping well | 90-RUNTIME: a paragraph enumerating what the probe/attention modules actually implement vs the doc | P2 |
| 7 | **Coordination sub-mechanisms** — `MorphogeneticState` (v1/13-07), `ResponseThresholds`/habituation (v1/13-04), `CohortMetrics`/`c_factor`, `PromotionGate`. `WisdomGate` gets 2 hits; these get **0**. | `roko-orchestrator/src/coordination.rs` (~1,991) | 36-ORCHESTRATION covers coordination.rs as "built-not-wired dead weight" (whole-file) | 36: name the sub-mechanisms so the v1/13 spec maps to symbols | P3 (dead code) |

## C. Checked and NOT a gap (already covered)

- **v1/16-heartbeat core** (three speeds, Gamma/Theta/Delta↔T0/T1/T2, adaptive clock, `HeartbeatPolicy`/
  `HeartbeatClock` two-clock split): thoroughly covered in **89-PRIMITIVES-HDC** and **90-RUNTIME**.
- **v1/17-lifecycle** (demurrage 29 hits, ebbinghaus 6, genomic 2, knowledge backup/restore): covered
  across 39-NEURO, 41-DREAMS, 55-DATA-DIR, 60-STATE-PERSISTENCE. `demurrage.rs`/`demurrage_consumer.rs`
  are real code and documented.
- **v1/13-coordination** top level (stigmergy/pheromones/`Pheromone`/`MeshRelay`/subnets): covered in
  36-ORCHESTRATION (built-not-wired) — only the sub-mechanisms in (7) are thin.
- **v1/14-identity-economy & v1/08-chain economy** (passport, ERC-8004 registries, reputation, x402,
  ISFR, futures market, korai token): **real code exists** (`roko-chain/{identity_economy_*,reputation_registry,
  validation_registry,agent_registry,x402,isfr*,futures_market,korai_token}.rs`, ~23K LOC) and is
  covered by **42-CHAIN-REGISTRIES-ISFR** (85 hits). Not a gap.
- **v1/15-code-intelligence**: covered by 49-INDEX-LANG.
- **Hermes / OpenClaw harness backends**: covered in 38-AGENT-PROVIDERS-TOOLS (5–7 hits).

## D. Spec-only (SO) — no gap here, belongs to 102-SPEC-DEBT

None newly found. The identity/economy/chain pieces that read spec-heavy in docs/v1/08 & /14
**do** have backing code (section C), so they are RCU-covered, not spec-debt.

---

## Recommendation

No new **epic** is warranted — every gap is a documentation paragraph/section, not missing product
work. Highest-value single fix: **gap #1 (policy_manifest.rs)** — 1,364 wired LOC feeding the prompt
builder with zero pack coverage; fold a section into **34-COMPOSE-PROMPTS**. Gaps #2–#5 are
one-paragraph additions to 40/91. Gaps #6–#7 are depth notes on already-listed dormant/dead code.
