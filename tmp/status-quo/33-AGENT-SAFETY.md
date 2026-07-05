# Safety Layer — Contracts, Capabilities, Taint, Audit

> Status-quo audit · verified 2026-07-07, re-verified 2026-07-08 (all file:line claims below spot-checked against HEAD `5852c93c05`; every claim held) · sources: 20 files / 10,519 LOC in `crates/roko-agent/src/safety/`, 7 files / ~3,387 LOC in `crates/roko-orchestrator/src/safety/`, dispatcher + provider + orchestrate wiring (~10 files), 8 embedded contract YAMLs, 17 v1 docs (`docs/v1/11-safety/`), 8 v2 docs (`docs/v2/16-SECURITY.md`, `docs/v2-depth/17-security/`), 8 git commits, 284 inline + ~23 integration tests. `.roko/GAPS.md` contains **zero** safety entries (still true 2026-07-08 — a documentation gap in its own right; see checklist P3).

> ⚠️ **CLAUDE.md is STALE on this subsystem.** The root `CLAUDE.md` row still reads *"Partial — AgentContract wired but falls back to permissive default when YAML missing."* That is **wrong as of HEAD**: missing/malformed YAML now yields a fail-closed `AgentContract::restricted` (deny-all) via `ContractLoadMode::RestrictedFallback` (`contract.rs:155-173`, `mod.rs:929-941`). The word "permissive" survives only in `#[cfg(test)]` helpers and one deliberate operator-TOML carve-out (§Enforcement reality). Fix the CLAUDE.md row when touching this area.

## Summary

The safety layer is **real, fail-closed, and enforced on every roko-managed tool call** — a genuine defense-in-depth pipeline, not vaporware. `ToolDispatcher::new` hard-defaults to `SafetyLayer::with_defaults()` (`crates/roko-agent/src/dispatcher/mod.rs:113`), and CLAUDE.md's "falls back to permissive default when YAML missing" is **stale**: commits `29ed84580` (T1-15), `78c4e3d00`, and `8f3497063` replaced it. Missing contract YAML now yields `AgentContract::restricted` (deny-all tools, 4K tokens, $0.50/turn, 3-failure cap) via `ContractLoadMode::RestrictedFallback` (`safety/mod.rs:929-941`, `safety/contract.rs:247-266`); `SafetyLayer::permissive()` and `ToolDispatcher::new_unguarded()` are `#[cfg(test)]`-only (`safety/mod.rs:273-275`, `dispatcher/mod.rs:126-141`). One deliberate permissive carve-out survives (see Enforcement reality §4).

The catch: what's enforced is the **wave-1 subset** (path/bash/git/network/scrub/rate-limit/contracts/role-whitelists + custody JSONL). The v1/v2 flagship concepts — capability warrants, taint-triggered confirmation, LTL temporal monitor, adaptive-risk budget, witness DAG, hash-chained audit, hook chain, tool selector, CaMeL dual-LLM — are all **built with tests but never constructed in production** (`with_warrant`/`with_safety_budget`/`with_temporal_monitor`/`with_hook_chain`/`with_tool_selector` have zero non-test callers). And a **second, parallel safety implementation** sits unwired in `roko-orchestrator/src/safety/` (capability_tokens, permit, loop_guard, sandboxing, taint_propagation, audit_chain — the only *actually* hash-chained audit log in the repo). Shape verdict: the enforced core is v2-pipeline-ish (§36.e "policies as pure validators, chained, first-failure short-circuits"), but it lives bolted inside roko-agent per 🕰️ v1 topology, not as the v2 graph/cell capability-stack.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| SafetyLayer pipeline (pre-exec chain) | v1 00-defense-in-depth; v2-depth 02-pipeline | `crates/roko-agent/src/safety/mod.rs:377-465` | ✅ wired | dispatcher calls at `dispatcher/mod.rs:395`; default at `:113` |
| Path escape prevention | v1 04, 06 | `safety/path.rs` (487 LOC) | ✅ wired | `mod.rs:426-438`; post-check `mod.rs:772-784` |
| Bash allow/deny policy | v1 04-permits-allowlists | `safety/bash.rs` (486 LOC) | ✅ wired | `mod.rs:411-416`; exec path `mod.rs:592` |
| Git branch protection | v1 04 | `safety/git.rs` (732 LOC) | ✅ wired | `mod.rs:414`, `mod.rs:593`; orchestrate git ops `orchestrate.rs:19771,19807,19838` |
| Network egress allowlist (+private-IP block) | v1 04, 08 | `safety/network.rs` (464 LOC) | ✅ wired | `mod.rs:419-423`; test `mod.rs:1214-1223` |
| Secret scrubbing (output + prompt + result) | v1 07-prompt-security (partial) | `safety/scrub.rs` (472 LOC) | ✅ wired | `dispatcher/mod.rs:502`; `orchestrate.rs:16427,17062` |
| Per-role/tool rate limiting | v1 09 (partial) | `safety/rate_limit.rs` (508 LOC) | ✅ wired | on by default `mod.rs:258`; checked `mod.rs:390-396,574-580` |
| AgentContract (invariants/governance/recovery) | v1 concept; hardened 2026-05 | `safety/contract.rs` (1,343 LOC) + 8 embedded YAMLs `safety/contracts/*.yaml` | ✅ wired | embedded `contract.rs:36-71`; enforced `mod.rs:459-462`; stateful via `ctx.external_actions` (`contract.rs:524,556,566`) |
| Restricted/hardened contract fallback | commits 29ed84580, 8f3497063 | `contract.rs:155-193`, `mod.rs:244-267,917-974` | ✅ wired | `RestrictedFallback` deny-all; per-role cache + `invalidate_contract_cache` (`contract.rs:275`) |
| Role tool whitelists (TOML `[agent.roles]`) | roko.toml design | `mod.rs:207,219-234,380-387,977-1010` | ✅ wired | `SafetyLayer::from_config` at `orchestrate.rs:4598,4833,5061` |
| Per-role layer at spawn / ACP | mega-parity R3_F03/G02 | `crates/roko-cli/src/agent_spawn.rs:77-78`; `crates/roko-acp/src/runner.rs:935-946`; `bridge_events.rs:1281-1282,1545` | ✅ wired | `.with_role(role)` per agent; ACP pre/post dispatch checks |
| Orchestrator pre/post dispatch checks (AGT-01) | v1 16-integration-gap | `mod.rs:680-821` | ✅ wired | `orchestrate.rs:16443` (pre), `:17093` (post) |
| Safety→prompt anti-patterns (INT-14) | v2 compose integration | `mod.rs:830-915` | ✅ wired | `orchestrate.rs:16146` |
| Subprocess/exec safety | task 076 redesign | `mod.rs:569-609`; `exec.rs:142` | ✅ wired | pipeline steps `orchestrate.rs:17562`; backends via task-local `provider/mod.rs:191,309-333` |
| Custody log (audit trail) | v1 02-audit-chain (weak form) | `safety/provenance.rs` (327 LOC); `crates/roko-cli/src/custody.rs` | ✅ wired | dispatch+gate records `orchestrate.rs:10841-10868`; CLI `roko knowledge custody list/show/verify` |
| Signal audit events per dispatch phase | v1 02 | `dispatcher/mod.rs:546-575` | ✅ wired | `Kind::ToolInvocation` → `ctx.audit_sink` per phase |
| Forensic replay (SAFE-12) | v1 15-forensic-ai; v2-depth 04 | `crates/roko-core/src/forensic.rs`; `roko-gate/src/forensic.rs` | ✅ wired | used by `roko-cli/src/main.rs`, `commands/util.rs`, `runner/persist.rs` (`roko replay`) |
| Loop detection | v1 05-loop-detection | `roko-conductor/src/diagnosis.rs:66,563`; `orchestrate.rs:10927-10949`; `roko-orchestrator/src/safety/loop_guard.rs` | 🟡 partial | diagnosis category wired; dedicated `LoopGuard` unwired |
| Capability tokens / OCaps warrants | v1 01-capability-tokens; v2 §2-3 | `safety/capabilities.rs` (405 LOC: `AgentWarrant`, `delegate`, `PluginTier`) | 🔌 built-not-wired | checked only "if present" `mod.rs:399-408`; `with_warrant` has zero non-test callers |
| Taint tracking / IFC | v1 03; v2 §4 taint lattice | `provenance.rs` (Taint), `hooks.rs` (TaintedString), `mod.rs:483-553` (AllowWithConfirm escalation); `roko-core` `Taint`/`TaintInfo` (lib.rs:265) | 🔌 built-not-wired | `authorize_call_with_taint` zero external callers; `Custody.with_taint`/`to_signal_taint` unused; only `roko-cli/src/custody.rs` displays taint |
| Authz decisions + confirmation channels | v2-depth 02 (HITL) | `safety/authz.rs` (394 LOC: `AuthzDecision`, `ConfirmationChannel`, `ApproveAll/DenyAll/LogAndDeny`) | 🔌 built-not-wired | zero callers outside safety module |
| Safety hook chain (TOOL-02) + hooks | v2-depth 02-pipeline | `dispatcher/hook_chain.rs`; hooks: `allowlist.rs`, `spending.rs`, `hallucination.rs`, `result_filter.rs` | 🔌 built-not-wired | dispatch honors it (`dispatcher/mod.rs:413-460`) but `with_hook_chain` never called in prod |
| Tool selector (TOOL-03) | task 076; fix 4cbca27fc | `dispatcher/tool_selector.rs` | 🔌 built-not-wired | CRITICAL allow-all wildcard fixed to deny-by-default read-only, but `with_tool_selector` has zero prod callers |
| Temporal logic monitor (LTL) | v1 11-temporal-logic | `safety/temporal.rs` (485 LOC) | 🔌 built-not-wired | hook exists `mod.rs:453-457`; `with_temporal_monitor` tests-only |
| Adaptive-risk safety budget | v1 09-adaptive-risk; v2-depth 05 | `safety/risk.rs` (667 LOC: Beta, Kelly, irreversibility) | 🔌 built-not-wired | `safety_budget: None` in defaults (`mod.rs:259`); `with_safety_budget` tests-only |
| Witness DAG (BLAKE3, signatures) | v1 12-witness-dag; v2-depth 04 | `safety/witness.rs` (587 LOC) | 🔌 built-not-wired | no runtime instantiation anywhere |
| Hash-chained audit log | v1 02-audit-chain | `roko-orchestrator/src/safety/audit_chain.rs` (prev_hash + `verify()`) | 🔌 built-not-wired | executor accepts `with_audit_chain` (`executor/mod.rs:427`) — never constructed in prod |
| Sandboxing | v1 06-sandboxing | `roko-orchestrator/src/safety/sandboxing.rs` | 🔌 built-not-wired 🕰️ | zero non-test users; real isolation = worktrees + PathPolicy only |
| Permits / capability tokens / taint (orchestrator copies) | v1 01/03/04 | `roko-orchestrator/src/safety/{permit,capability_tokens,taint_propagation}.rs` | 🔌 built-not-wired 🕰️ | duplicate parallel implementation; zero non-test users |
| CaMeL dual-LLM prompt security | v1 07; v2-depth 06 | `safety/data_llm.rs` (439 LOC, `sanitize_input`) | 🔌 built-not-wired | config admits it: "Reserved for future CaMeL dual-LLM isolation" (`roko-core/src/config/agent.rs:74-79`); `data_llm: None` default |
| Immune system (quarantine vault) | v2-depth immune-system-as-graph | `roko-core/src/immune.rs` | 🔌 built-not-wired | zero users outside roko-core |
| MEV protection | v1 10-mev-protection | — | ❌ missing | zero code hits (`MevProtection`, commit-reveal) |
| Formal verification | v1 13; v2-depth 07 | — | ❌ missing | no kani/prusti/creusot/proof code |
| Cognitive kernel safety | v1 14; v2-depth 07 | — | ❌ missing | zero code hits |
| Threat model | v1 08-threat-model | doc-only by nature | ❌ (doc) | partially reified as network private-IP blocks, bash denylist |

## Enforcement reality (what actually blocks/permits at runtime)

**Trace — every roko-managed tool call** (`ToolDispatcher::dispatch`, `dispatcher/mod.rs:230-509`):
1. tool filter (`ctx.allowed_tools`/`denied_tools`) → deny + audit signal (`:328-345`)
2. role permission flags (`ToolPermissions.satisfied_by`, `:350-376`)
3. `safety.check_pre_execution` (`:395`) which chains, first-failure-wins (`safety/mod.rs:377-465`): **role tool whitelist → rate limit → [warrant]* → bash+git policy → network policy → path canonicalization → [budget]* → [temporal]* → contract (allowed_tools ∩ invariants ∩ governance)**. *Bracketed stages are skipped in production because nothing attaches them.*
4. [hook chain]* (`:413-460`) → 5. handler with timeout+cancel (`:490-498`) → 6. truncate → 7. `scrub_output` (`:502`) → 8. `check_recovery` contract recovery rules (`:503`) → 9. terminal audit Signal.

**Trace — plan execution** (`orchestrate.rs`): `SafetyLayer::from_config` (`:4598/:4833/:5061`) → anti-patterns into prompt layer 7 (`:16146`) → prompt scrubbed (`:16427`) → `pre_dispatch_check` blocks task before spawn (`:16443`) → agent spawned with layer + role (`agent_spawn.rs:77-78`, task-local propagation `provider/mod.rs:309-333`) → dispatcher built `with_safety` (`:16754`) → result scrubbed (`:17062`) → `post_dispatch_check` (secret leak, path escape, forbidden-write → **Warn-severity violations, logged not blocked**, `safety/mod.rs:749-821`, `:17093`) → custody records for dispatch + gate (`:10841-10868`). Raw subprocess/git paths go through `check_exec_command` (`:17562`, `:19771-19838`; `exec.rs:142`).

**Contract hardening is real**: gate-approval and token counts can no longer be forged via LLM-supplied args — `has_gate_approval`/`estimated_tokens` read only orchestrator-recorded `ctx.external_actions` (commit `600c06cd0`; `contract.rs:717,762`); stateful governance (MaxToolCallsPerTurn, MaxConsecutiveFailures, RequireToolBeforeEdit) reads the same trusted history (`contract.rs:524,556,566`).

**Surviving permissive paths** (exact code):
- `safety/mod.rs:949-956` — if the restricted fallback produced deny-all `allowed_tools` **but the role is operator-defined in TOML** (any `[agent.roles.X]` entry, even without a `tools` list), `contract.allowed_tools = None`. Comment: *"An operator-defined role without a tools list is intentionally permissive."* Bash/network/path policies still apply, but tool-level allowlisting is gone for such roles.
- `mod.rs:244-267` — `hardened_default` for the `default` role intentionally leaves `allowed_tools = None` (invariants/governance still apply).
- Warn-vs-Block: all `post_dispatch_check` findings are `ViolationSeverity::Warn` (`mod.rs:767,781,803`) — nothing post-hoc ever blocks/rolls back.
- **Claude-CLI subprocess dispatch**: tools execute inside the external `claude` binary, so per-tool-call interception doesn't apply there — only prompt scrub, pre/post dispatch checks, `--allowedTools` CSV (`orchestrate.rs:16578`), and exec-command checks. This is exactly the residual gap v1 doc 16 describes (`docs/v1/11-safety/16-critical-integration-gap.md:18-20`).

## V2-aligned

- Policy-pipeline shape (pure validators, chained, short-circuit) matches v2-depth `02-defense-in-depth-as-pipeline` — `safety/mod.rs:1-25` even documents it that way.
- Fail-closed contract loading + embedded assets + per-role cache (`contract.rs`, commits `78c4e3d00`, `b9235b566`, `6ff90572e`).
- Trust-boundary discipline (ctx-recorded actions as the only evidence source) matches v2 "no-laundering" spirit.
- Custody + forensic replay (`roko-core/src/forensic.rs` 7-step reconstruction) is a solid seed for v2-depth `04-audit-witness-and-forensics`.
- Taint escalation to `AllowWithConfirm` (`mod.rs:514-550`) implements v2 §4.6 "declassification requires human approval" — it just needs a caller and a confirmation channel.

## Old paradigm & tech debt

- 🕰️ **Topology**: safety is a bolt-on inside roko-agent + a clone inside roko-orchestrator, not the v2 three-layer capability stack (cell declaration → graph allow-list → space grant, `docs/v2/16-SECURITY.md` §3). No capability types on cells/graphs exist.
- 🕰️ **Duplicate implementation**: `roko-orchestrator/src/safety/` (~3.4K LOC, 6 modules) re-implements capability tokens, permits, taint, sandboxing, loop guard, audit chain — all unwired. The *better* audit log (hash-chained `AuditChain`) is in the dead copy; the *wired* custody log verifies only JSON-parse + monotonic timestamps + non-empty fields (`custody.rs:201-235`) — **not tamper-evident** despite printing "Chain integrity: OK" (`custody.rs:245`).
- 🕰️ Two taint vocabularies (action-centric `safety::provenance::Taint` vs signal-level `roko_core::Taint`) bridged by an unused `to_signal_taint` (`provenance.rs:60`).
- Dead-but-honored config: `hook_chain`/`tool_selector` slots on the dispatcher, `warrant`/`budget`/`temporal_monitor` slots on the layer — five extension points that production never populates; they rot silently.
- `data_llm` config knob exists (`[agent.data_llm]`) but routing is never constructed — config that does nothing.

## Not implemented (incl. doc-only safety concepts)

- **Zero-code v1 docs (3/17)**: `10-mev-protection.md`, `13-formal-verification.md`, `14-cognitive-kernel-safety.md`.
- **Doc-only in effect (built, zero runtime effect)**: 01-capability-tokens, 03-taint-tracking (runtime propagation), 06-sandboxing (as a mechanism beyond worktree+path), 09-adaptive-risk (budget never attached), 11-temporal-logic, 12-witness-dag; v2-depth 06-prompt-security-and-camel (dual-LLM), immune-system-as-graph.
- No human-in-the-loop confirmation transport (channels exist, nothing routes `AllowWithConfirm` to a human).
- No v2 taint lattice / monotonic join / tag propagation on signals; no capability narrowing across delegation at runtime.
- No safety observability surface: roko-serve has no safety/custody/witness routes (`crates/roko-serve/src/routes/` — auth.rs is HTTP auth only).

## Migration checklist

- [ ] **[P0]** Tamper-evident audit — **`custody verify` currently prints "Chain integrity: OK" while checking only JSON-parse + monotonic timestamps + non-empty action/principal (`custody.rs:206-235,244-245`); a hand-edited record with a valid timestamp passes.** This is a false assurance, not just a missing feature. Fix: either instantiate the already-hash-chained orchestrator `AuditChain` (executor `with_audit_chain`, `executor/mod.rs:427`, never constructed in prod) or add prev-hash linking to `CustodyLogger` records; make `custody verify` recompute+compare hashes — verify: `cargo run -p roko-cli -- knowledge custody verify` should FAIL on a hand-edited line (today it passes).
- [ ] **[P1]** Decide whether `post_dispatch_check` findings should ever block. Today secret-leak, path-escape, and forbidden-write post-checks are all `ViolationSeverity::Warn` (`mod.rs:767,780,803`) — logged, never blocked/rolled back — while `pre_dispatch_check` correctly `Block`s (`mod.rs:702,716,732`). A leaked secret in agent output is detected post-hoc but the task is not quarantined (see Open Q1 + `roko-core/src/immune.rs`) — verify: `roko run` with a task that emits a secret; branch should quarantine, not just warn.
- [ ] **[P1]** Attach `SafetyBudgetTracker` from config in `SafetyLayer::from_config` (adaptive risk becomes live) — verify: `grep -n 'with_safety_budget' crates/roko-cli/src/orchestrate.rs` + a plan run that exhausts `footprint_limit`
- [ ] **[P1]** Route taint: record `Custody::with_taint` on web_fetch/MCP results in orchestrate, call `authorize_call_with_taint` in dispatch, and wire a `ConfirmationChannel` (TUI/serve) for `AllowWithConfirm` — verify: `cargo test -p roko-agent taint` + manual `roko run` with a web_fetch task
- [ ] **[P1]** Decide the duplicate: delete or merge `roko-orchestrator/src/safety/{permit,capability_tokens,taint_propagation,sandboxing,loop_guard}.rs` into roko-agent — verify: `grep -rn 'orchestrator::safety' crates --include='*.rs' | grep -v tests`
- [ ] **[P2]** Attach `ToolSelector`/`SafetyHookChain` (spending, hallucination, allowlist, result-filter hooks) in `build_dispatcher` (`provider/mod.rs:333`) behind config — verify: `roko run` with a hook-rejecting profile; audit signals show `hook_chain` phase
- [ ] **[P2]** Close the TOML carve-out: warn loudly (or require `tools=[...]`) when an operator role clears deny-all `allowed_tools` (`safety/mod.rs:949-956`) — verify: `cargo test -p roko-agent contract` + `roko doctor` warning
- [ ] **[P2]** Issue `AgentWarrant`s per task from plan metadata (capability tokens live) — verify: `cargo test -p roko-agent -- exec_command_requires_warrant`
- [ ] **[P2]** Wire `TemporalMonitor` with per-role LTL properties from contract YAML — verify: force-push property test via `roko run`
- [ ] **[P3]** Persist `WitnessDag` vertices for dispatch/gate/commit events; link from episodes — verify: `.roko/` witness file grows during `plan run`
- [ ] **[P3]** Update CLAUDE.md safety row ("Partial — permissive fallback" → fail-closed w/ TOML carve-out) and log the 🔌 items in `.roko/GAPS.md` — verify: `grep -i safety .roko/GAPS.md`
- [ ] **[P3]** v2 shape: introduce capability declarations on cells/graph allow-lists when roko-graph cells go live (GAPS.md Task 103) — verify: `docs/v2/16-SECURITY.md` §3 mapping doc

## Open questions

1. Post-dispatch violations are Warn-only — should secret-leak or path-escape post-checks quarantine the task branch (tie into `roko-core/src/immune.rs`) instead of just logging?
2. For Claude-CLI subprocess dispatch, is `--allowedTools` + prompt anti-patterns considered sufficient, or should roko proxy tools via MCP to regain per-call enforcement (v1 doc 16's endgame)?
3. Which audit store is canonical long-term: Signal audit events, custody JSONL, orchestrator AuditChain, or WitnessDag? Four candidates exist; only the two weakest are wired.
4. Should the operator-TOML permissive carve-out (`mod.rs:949-956`) require an explicit `unrestricted = true` flag rather than being implied by a role section's existence?
5. `hardened_default` caps (4K tokens, $0.50, 3 failures) — are these tuned for real implementer workloads, or will they silently throttle the `default` role once budgets get attached?
