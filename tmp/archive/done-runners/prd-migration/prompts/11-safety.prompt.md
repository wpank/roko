# Prompt: 11-safety

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/11-safety/`. Covers defense-in-depth, capability tokens, audit chain, taint-aware ingestion, permits and allowlists, loop detection, sandboxing, prompt security (ventriloquist defense), threat model (21 failure catalog), adaptive risk, MEV protection, temporal logic verification, witness DAG, formal verification, Cognitive Kernel Primitives (Namespaces + Signals + Scheduling + Syscalls), Forensic AI regulatory compliance. **CRITICAL**: flag the #1 integration gap — SafetyLayer wired to ToolDispatcher but dispatcher never invoked from CLI pipeline.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §Provenance & Attestation, §Decay (memory management not mortality)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §IX Forensic AI / Causal Replay (regulatory compliance table), §XII Cognitive Kernel Primitives (Namespaces with ACL, Signals, Scheduling, Engram Syscalls)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 1G (production hardening)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`

## Step 3 — SOURCE-INDEX entry `## 11-safety.md`

Read every file. Key legacy:
- All of `bardo-backup/prd/10-safety/` (00-defense through 10-mev-protection)
- `bardo-backup/prd/04-memory/09-safety.md`, `bardo-backup/prd/12-inference/07-safety.md`
- `bardo-backup/tmp/mori-refactor/11-safety-observability-learning.md`
- `bardo-backup/tmp/roko-progress/09-refactor-gaps.md` — safety gaps
- `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md`

## Step 4 — implementation-plans

- `03-safety-hooks.md`
- `11-inconsistencies.md` — **read carefully, contains the #1 integration gap**
- `12b-chain-layer.md` §P Privacy (Valhalla TEE, PSI, ZK range proofs)

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/**/*.rs`
- Read: `mod.rs` (256 lines — SafetyLayer). Also read the individual guards: `bash.rs`, `git.rs`, `network.rs`, `path.rs`, `scrub.rs`, `rate_limit.rs`.
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` — see `.with_safety(layer)` wiring.
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/safety/` (if exists)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` — observe that it uses ExecAgent directly, never creating a ToolDispatcher.

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/11-safety
```

Write **17 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-defense-in-depth.md` | Defense-in-depth overview. Layers of safety: capabilities, audit, taint, sandbox, prompt security, threat model. Why each layer matters. |
| 01 | `01-capability-tokens.md` | Typed, unforgeable authorization tokens. Least-privilege per role. Capability declarations at Framework layer. |
| 02 | `02-content-addressed-audit-chain.md` | BLAKE3 lineage DAG. Tamper-evident. Every Engram traces to its inputs. Replay any decision chain. |
| 03 | `03-taint-aware-ingestion.md` | TaintLevel: Trusted / Unverified / Suspicious. Taint propagates through lineage DAG. Reviewers see what was touched by unverified sources. |
| 04 | `04-permits-and-allowlists.md` | Permit-based authorization. Allowlists per tool category. Configuration in roko.toml. |
| 05 | `05-loop-detection-and-guard.md` | Detecting agents stuck in loops. Guard against infinite recursion, runaway token burn, repeated same action. |
| 06 | `06-sandboxing-validation-only.md` | Validation-only sandbox approach. No hardware isolation for local execution (not needed). Safety via capability and policy, not process isolation. |
| 07 | `07-prompt-security-and-ventriloquist.md` | Prompt injection prevention. **Ventriloquist defense** (cross-reference 08-chain.md §05): SHA-256 system prompt hash committed on-chain, TEE verifies before each job, updates require 24h timelock, >3 changes in 30 days → reputation penalty. |
| 08 | `08-threat-model-21-failures.md` | The 21 production failure catalog from `bardo-backup/tmp/mori-refactor-plan/00-issues-catalog.md`. Each failure mode enumerated with reframed framing. Cross-reference 07-conductor.md. |
| 09 | `09-adaptive-risk-management.md` | Risk scoring. Adaptive thresholds. Integration with Daimon PAD (high arousal → more conservative). |
| 10 | `10-mev-protection.md` | MEV (Maximal Extractable Value) protection for chain agents. Transaction ordering defenses. Cross-reference 08-chain.md. |
| 11 | `11-temporal-logic-verification.md` | Temporal logic constraints. Safety invariants over sequences. |
| 12 | `12-witness-dag.md` | Witness DAG for cryptographic verification of agent actions. Ed25519 signatures per Engram. Chain of custody from prompt to verdict. |
| 13 | `13-formal-verification-pipeline.md` | Formal verification integration. Property-based tests, model checking, theorem proving. Where and when each applies. |
| 14 | `14-cognitive-kernel-primitives.md` | OS-level primitives for agents. (1) **Cognitive Namespaces**: isolated knowledge spaces with ACL, explicit cross-namespace channels. Permissioned subnets use namespaces. Full Rust struct. (2) **Cognitive Signals**: typed interrupts (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown) — behavior modification, not process killing (cross-reference 07-conductor.md). (3) **Cognitive Scheduling**: `cognitive_priority = task_urgency × expected_value × (1/cognitive_cost)`. (4) **Engram Syscalls**: every meaningful agent action passes through Policy.decide() → permit/deny/modify/log. Single enforcement point for security, auditing, rate limiting, cost tracking. |
| 15 | `15-forensic-ai-regulatory-compliance.md` | Full regulatory compliance mapping table (cross-reference 04-verification.md §12 for causal replay): EU AI Act Art. 14 (human oversight mechanisms) + FRIA (fundamental rights impact assessment), SEC/CFTC (trading decision reconstruction, MiFID II), HIPAA (clinical decision audit trail, PHI-aware Gate), SOX (financial control documentation), GDPR (purpose-limitation Policy). Pre-certified agent templates (SEC-Compliant Trading Agent, HIPAA-Compliant Clinical Agent, GDPR-Compliant Data Agent). Enterprise value $100-500K/month. Certification moat — multi-year, multi-million-dollar process to get regulator blessing. |
| 16 | `16-critical-integration-gap.md` | **THE #1 KNOWN INTEGRATION GAP**. Safety policies ARE implemented: `roko-agent/src/safety/mod.rs` has `SafetyLayer` (256 lines) composing bash/git/network/path/scrub/rate_limit guards. `roko-agent/src/dispatcher/mod.rs` has `ToolDispatcher.with_safety(layer)` integration. **But** `roko-cli/src/orchestrate.rs` never creates a `ToolDispatcher` — it calls `ExecAgent::run()` directly. The SafetyLayer is wired but never invoked from the CLI pipeline. This is the #1 known gap per `11-inconsistencies.md`. Explain clearly. Explain what fixing it requires: refactor orchestrate.rs to create a ToolDispatcher, plug in SafetyLayer, route agent tool calls through it. This is part of Tier 1 hardening. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥4000 total. Citations: CaMeL (Debenedetti et al.), OWASP Top 10, Constitutional AI (Anthropic), Cohen undecidability theorems, C2PA content credentials, DIDs (W3C), EU AI Act, HIPAA, SOX, GDPR, MiFID II.

Cross-reference topics 00-architecture (Provenance, Attestation), 02-agents (ExecAgent vs ToolDispatcher), 04-verification (gates + causal replay), 07-conductor (cognitive signals), 08-chain (Valhalla privacy, ventriloquist defense).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- **The #1 integration gap MUST be prominently flagged.** SafetyLayer is built and wired to ToolDispatcher, but ToolDispatcher is never invoked from the CLI pipeline. Make this a separate sub-doc (16) and also mention it in INDEX.md.
- Apply naming map: golem → agent; mori → Roko Orchestrator.
- No death framing.
- Use Write tool. Don't ask questions.
