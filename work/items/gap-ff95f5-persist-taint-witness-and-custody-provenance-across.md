+++
id = "gap-ff95f5"
kind = "gap"
title = "Persist Taint, Witness, and Custody Provenance Across Restart"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
subsystem = ["roko-agent/safety"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-01
last_verified_rev = "6531d787e"
source = "tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart"
discovered_from = "audit:tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart"
anchors = ["crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker", "crates/roko-agent/src/safety/witness.rs::WitnessLogger", "crates/roko-agent/src/safety/provenance.rs::CustodyLogger", "crates/roko-cli/src/custody.rs::log_chained", "crates/roko-agent/src/dispatcher/mod.rs::ToolDispatcher::dispatch", "crates/roko-agent/src/provider/mod.rs::build_tool_dispatcher_with_audit", "crates/roko-graph/src/snapshot.rs::EXT_SAFETY_PROVENANCE", "crates/roko-cli/src/graph_checkpoint.rs::refresh_gate_verdicts"]
links = { depends_on = [], blocks = [], related = ["gap-1151bf", "gap-bf8d20"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'EXT_SAFETY_PROVENANCE' crates/roko-cli/src && grep -rqw 'fn safety_provenance_intent_recorded_before_handler' crates/roko-agent/ && cargo test -p roko-agent safety_provenance_intent_recorded_before_handler && grep -rqw 'fn safety_provenance_restores_taint_after_restart' crates/roko-cli/ && cargo test -p roko-cli safety_provenance_restores_taint_after_restart"
+++

## Problem

Safety provenance lives only in memory and live dispatch never records it. Three pieces are affected:

- `TaintTracker` in `crates/roko-agent/src/safety/taint_propagation.rs` keeps a monotonic trust lattice (content hash -> taint level, reason, parent hashes) and a propagation audit. It has atomic `save`/`load` (lines 277-338), but no production code creates, saves or restores one.
- `WitnessDag` and `WitnessLogger` (`safety/witness.rs`) are a BLAKE3 content-addressed reasoning DAG (Observation -> Prediction -> Decision -> Resolution -> NeuroEntry) that can append to `.roko/witness.jsonl`. Nothing outside the file uses them. They are only re-exported at `safety/mod.rs:110`.
- `CustodyLogger` (`safety/provenance.rs`) and the hash-chaining helper `log_chained` (`crates/roko-cli/src/custody.rs:52`) are used only by the read-side `roko knowledge custody list/show/verify` commands (`custody.rs:75`, `:150`). No live path writes a custody record, so those commands read an empty or stale log.

Observable result: run `roko plan run <plan>` with an API-backed provider that calls tools. Afterwards `.roko/witness.jsonl` gets no new vertex, `roko knowledge custody list` shows no new record, and the checkpoint `.roko/state/graph/<plan>/checkpoint.json` has no `roko.safety-provenance@1` extension. After a restart or `--resume-plan`, any taint ancestry derived during the run is gone. Nothing can prove afterwards which external or tool inputs led to an output or a denial.

Expected: before each privileged tool effect there is an acknowledged pre-effect record, and after it a terminal result or denial record. The taint index and the witness/custody root hash are checkpointed and restored before new work is scheduled. If required provenance is missing or corrupt, the run fails closed instead of resetting to trusted.

## Why it matters

- Goal `features` (feature ideas). Severity p1: safety provenance is lost on restart, and the forensic components (`roko knowledge custody`, witness DAG) exist but are never populated.
- It turns an unused safety library into evidence that a real run can be audited: which untrusted input reached which privileged tool.
- Risk of leaving it: `roko knowledge custody verify` reports a clean chain only because nothing is written. Taint does not constrain anything across a restart.
- Related: `gap-1151bf` (TaintTracker audit log is ephemeral; closed as a duplicate of this item), parked `gap-bf8d20` (TaintTracker not wired to immune boundary decisions), `gap-22b0a2` (the #282 checkpoint-extension gate; superseded), `spec-5c8b9c` (#208 runtime event schema; done).

## Where

- `crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker`: the lattice. `mark_tainted`, `propagate(parents, child)`, `observe_signal(&Signal)`, `derived_from`, `audit_log`, `save`/`load`.
- `crates/roko-agent/src/safety/witness.rs::WitnessDag`, `::WitnessLogger`: the vertex DAG, a standalone JSONL adapter, and an integrity walk (`IntegrityViolation`).
- `crates/roko-agent/src/safety/provenance.rs::CustodyLogger`, `Custody`, `Taint`, `AttestationLevel`: custody records.
- `crates/roko-cli/src/custody.rs::log_chained`: chains a custody record to the previous hash. The rest of the file is the read-side commands.
- `crates/roko-fs/src/layout.rs`: `custody_log()` (:413) and `witness_log()` (:423, `.roko/witness.jsonl`).
- `crates/roko-agent/src/dispatcher/mod.rs::ToolDispatcher::dispatch` (:531): the one choke point for tool calls from roko's own tool loop. Its order is ingress validation, then `SafetyLayer` stages 1-4, then `production_safety_chain` stages 5-7 (stage 6 is the taint ceiling `TaintLevelHook`, which reads the per-call `ToolContext` taint level, not `TaintTracker`), then `safety_denial_callback` on denial (:781), then `handler.execute` (:857), then the stage 9 result filter (:880). It already has two optional side channels you can copy: `file_audit` (a `ScrubAuditAdapter` JSONL) and `safety_denial_callback`.
- `crates/roko-agent/src/provider/mod.rs::build_tool_dispatcher_with_audit` (:459): the production dispatcher builder. It is used by the Anthropic API (`provider/anthropic_api/tool_loop.rs:50`), OpenAI-compatible (`provider/openai_compat.rs:508`) and Gemini (`gemini/adapter.rs:47`, `:109`) tool loops through `options.tool_audit`.
- `crates/roko-graph/src/snapshot.rs`: `CheckpointExtension` (:192), `ExtensionRegistry::validate_extensions` (:339), and `EXT_SAFETY_PROVENANCE = "roko.safety-provenance@1"` (:550). The namespace is already registered as optional with owner "#351" in `register_known_namespaces` (:572).
- `crates/roko-cli/src/graph_checkpoint.rs`: host-side checkpoint. `register_extension` (:671) is write-once by fingerprint. `refresh_gate_verdicts` (:641) is the precedent for a mutable extension that is rebuilt and re-inserted into `manifest.extensions` on each save (`GATE_VERDICT_EXTENSION`, :62).
- Entry point: `roko plan run <dir>` -> `crates/roko-cli/src/graph_execution/` -> `crates/roko-cli/src/graph_task_dispatch.rs` -> provider tool loop -> `ToolDispatcher::dispatch`.

## Current state

- The dependencies the original packet waited on are gone or done. #208 (runtime event schema) is done (`spec-5c8b9c`, commit `91b4745f8`). The #251 layered checkpoint-extension ledger exists (`CheckpointExtension`, the EXT_* table, round-trip tests in `snapshot.rs`). #282 was superseded (`gap-22b0a2`), and its namespace reservation for this item is already in code. The "[blocked]" status in the original notes is obsolete.
- Nothing encodes, decodes or reconciles `roko.safety-provenance@1`. `grep -rn EXT_SAFETY_PROVENANCE crates/` finds only `snapshot.rs` and the `lib.rs` re-export.
- There is no production `TaintTracker::new`, `WitnessLogger::new` or `log_chained` call.
- Per-call taint does exist: `ToolContext::with_taint_level` is set by `tool_loop/agent_wrapper.rs:204`, `dispatch/plugin_mcp.rs:282`, ACP `bridge_events/dispatch.rs:290`, and others. So the stage 6 ceiling runs per call without any lineage.
- No recent commits touch the three safety files. The last ones are the old batch commits `244f564e1` and `c828afe7f`.
- Scope limit: CLI-backed providers (for example `ClaudeCli`, `CodexCli`) run their own tool loops inside the subprocess. `ToolDispatcher` never sees those calls, so per-tool provenance is only possible for roko's own tool loop (API providers, MCP, plugins). For CLI providers the best available is one record per dispatch turn.

## Plan

1. In `roko-agent`, add a `SafetyProvenanceSink` trait, e.g. `crates/roko-agent/src/safety/provenance_sink.rs`, re-exported from `safety/mod.rs`. Give it `record_intent(&ProvenanceIntent) -> Result<Ack>` and `record_outcome(&ProvenanceOutcome) -> Result<()>`. Records carry only hashes, IDs, taint levels and bounded reason codes: never prompt bodies, tool arguments, tool output or secrets. Use keyed or context-separated BLAKE3 (`blake3::derive_key` or `keyed_hash`) for argument digests, because plain hashes of low-entropy values leak them. Add a deterministic in-memory fake for tests.
2. Add `ToolDispatcher::with_provenance_sink(Arc<dyn SafetyProvenanceSink>)`, next to `with_file_audit`. In `dispatch`:
   - after stages 1-7 pass and before `handler.execute`, call `record_intent`. If it errors, return a `ToolResult::err` without running the handler (fail closed);
   - after the handler (or on any denial branch), call `record_outcome` with success, error or denial and the reason code.
   - Hold a `TaintTracker` in the sink or host and `propagate` parent hashes (the inputs) to the result hash.
3. Host adapter in `roko-cli` (for the Graph run): keep the taint index plus `last_sequence`, the witness/custody root hash and the policy/contract fingerprints in the `roko.safety-provenance@1` checkpoint extension. Rebuild it on every checkpoint save the way `refresh_gate_verdicts` does, not through the write-once `register_extension`. Design choice for the per-effect records:
   - (a) Append them to the existing per-plan ledger `.roko/state/graph/<plan>/activities.jsonl` as a new record kind. This is one durable log, which the original packet requires ("do not write a second JSONL log beside the canonical host ledger").
   - (b) Reuse `WitnessLogger` and `CustodyLogger` against `.roko/witness.jsonl` and the custody log, and store only their root hash in the checkpoint. Less new code, and `roko knowledge custody` works immediately, but it is a second log and not atomic with the checkpoint.
   - Recommendation: (b) for the custody/witness records, because it is the only way the existing read-side commands show anything, plus the checkpoint extension for the taint index and root hash. Verify the root on restore. If the owner wants the single-ledger rule kept, choose (a) and change `custody.rs` to read from it. Either way, write and fsync the intent before the effect.
4. Restore: when a Graph run resumes (checkpoint load in `graph_checkpoint.rs`), decode the extension, rebuild `TaintTracker`, and walk the witness/custody chain to the stored root, all before any new task is admitted. An unknown version, missing root, broken parent, taint downgrade or hash mismatch is terminal safety corruption: stop the run with a clear error and do not reset to trusted. Decide whether `register_known_namespaces` should mark `EXT_SAFETY_PROVENANCE` as required. Today it is optional, so an old checkpoint without it restores silently; keep that for legacy checkpoints if you flip it.
5. Replay idempotency: the key is `(run_id, activity/attempt, effect index, phase)`. On resume, a matching acknowledged intent or outcome is reused, not duplicated. Whether a non-idempotent external effect already happened stays with the activity/receipt ledger (`prepare_receipt`/`commit_receipt` in `snapshot.rs`). An acknowledged intent alone does not prove it.
6. Wire the sink into `build_tool_dispatcher_with_audit` through the provider options, the same way `tool_audit` is threaded, so the Graph path gets it. Leave `roko chat`, `serve` and ACP on a no-op sink unless it is cheap to add them.
7. Tests (names are suggestions, used by the verify command):
   - `roko-agent`: `safety_provenance_intent_recorded_before_handler`, which asserts the handler does not run when `record_intent` fails;
   - `roko-agent`: a redaction test proving no raw arguments or output reach the record;
   - `roko-cli`: `safety_provenance_restores_taint_after_restart`, which writes a checkpoint, reloads, and checks that `get_level`/`derived_from` survive;
   - tamper, downgrade, missing-parent and unknown-version restore tests that must fail closed.

## Done when

- A tool call through `ToolDispatcher::dispatch` with a sink attached produces an acknowledged intent record before the handler runs and a terminal outcome or denial record after it. If the intent write fails, the handler does not run.
- A Graph plan run writes a `roko.safety-provenance@1` extension into `checkpoint.json`. After a process restart and resume, taint ancestry and policy fingerprints are restored before any task is scheduled.
- A corrupted, downgraded or unknown-version extension makes the resume fail closed with a diagnostic.
- Durable records hold hashes, IDs, levels and reason codes only.
- `roko knowledge custody list` shows records from a real run (if option (b) is chosen).
- Verify, replacing the current grep, which demands `WitnessLogger::` and `log_chained(` call sites and would push toward the second log the design avoids:
  `grep -rqw 'EXT_SAFETY_PROVENANCE' crates/roko-cli/src && grep -rqw 'fn safety_provenance_intent_recorded_before_handler' crates/roko-agent/ && cargo test -p roko-agent safety_provenance_intent_recorded_before_handler && grep -rqw 'fn safety_provenance_restores_taint_after_restart' crates/roko-cli/ && cargo test -p roko-cli safety_provenance_restores_taint_after_restart`

## Notes

- Safety- and persistence-critical: this changes the tool dispatch hot path and checkpoint restore. It is not suitable for FAST mode. Keep the safety core free of filesystem and checkpoint types; put I/O in the host adapter.
- Fail closed is a project rule: "missing or unknown safety contracts fail closed". Do not add a code path that resets taint to trusted after a load error.
- Do not add model-based trust classification. This item persists the lattice decisions that already exist.
- Do not change the other EXT_* rows in `snapshot.rs`.
- Size L (the original packet estimated 3-5 days). It conflicts with concurrent work in `crates/roko-agent/src/dispatcher/mod.rs` and `crates/roko-cli/src/graph_checkpoint.rs`, so do not run it in parallel with other items anchored there.
- No hard dependencies remain open.
- 2026-10-01 (wk-tamper): Plan steps 1-2 on work/gap-7147bb; cargo verification deferred to the batch check. The
  premise still held at 6531d787e.
  - `safety/provenance_sink.rs` adds `SafetyProvenanceSink` (`digest_key`, `record_intent` returning a
    `ProvenanceAck`, `record_outcome`). Records carry IDs, keyed BLAKE3 digests (`ContentHash::keyed`, new in
    roko-core; arguments are digested as RFC 8785 canonical JSON), taint levels and `tool_error_kind` reason codes,
    nothing else. `track_intent` and `track_outcome` keep a `TaintTracker`: arguments taken in a tainted turn carry
    its taint, and a result inherits from its arguments. `MemoryProvenanceSink` is the deterministic fake.
  - `ToolDispatcher::with_provenance_sink`: once every safety stage has passed and the handler is resolved,
    `record_intent` must succeed, or the call returns `PermissionDenied` without running. Every call then records
    an outcome: succeeded, failed, or denied with its reason code (`provenance_intent_failed` for a refused intent).
  - Tests: `safety_provenance_intent_recorded_before_handler`, `safety_provenance_records_hold_no_arguments_or_output`,
    `safety_provenance_records_a_denied_call`, `safety_provenance_propagates_taint_to_the_result` and
    `arguments_digest_ignores_key_order_and_depends_on_the_key` (roko-agent lib); `keyed_hash_depends_on_its_key`
    (roko-core).
  Still open: steps 3-7 (the roko-cli host sink and the `roko.safety-provenance@1` extension, restore and fail-closed
  checks, replay idempotency, threading the sink through `build_tool_dispatcher_with_audit`, the restart tests).
- 2026-10-01 (wk-tamper): Plan step 3, option (b), on work/gap-7147bb; cargo verification deferred to the batch check.
  - `roko-cli/src/safety_provenance.rs`: `GraphProvenanceSink` writes each intent and outcome as a witness vertex
    in `.roko/witness.jsonl` (an outcome's parent is its intent's vertex, whose id is the intent's ack) and as a
    custody record chained with `custody::log_chained` (SHA-256; it now returns the new head) in
    `.roko/custody.jsonl`. Both files are synced to disk before `record_intent` returns, so the handler runs only
    after its intent is on disk. The sink keeps the `TaintTracker`. Its digest key is
    `.roko/state/safety-provenance.key` (32 random bytes, created 0600; a malformed key fails closed). A reopened
    sink extends both chains.
  - `GraphProvenanceSink::summary` gives the record count, both chain heads and the taint index
    (`TaintTracker::to_json`/`from_json`, new). `PreparedGraphCheckpoint::attach_safety_provenance` makes every
    manifest write (`persist_manifest`, and best effort in `finish_with_status`) rebuild `EXT_SAFETY_PROVENANCE`
    from it.
  - Tests: `graph_provenance_sink_writes_synced_witness_and_custody_chains`,
    `graph_provenance_sink_refuses_a_bad_key_file`, `checkpoint_writes_store_the_safety_provenance_summary`
    (roko-cli lib); `tracker_json_roundtrips_state_and_refuses_other_values` (roko-agent lib).
  Still open: the policy and contract fingerprints in the summary; step 4 (restore before scheduling: rebuild the
  tracker, walk the witness and custody chains to the stored heads, and fail closed on a mismatch, downgrade or
  unknown version); step 5 (replay idempotency); step 6 (no run attaches the sink yet: thread it through
  `AgentOptions` into `build_provider_tool_dispatcher`, then `DispatchFactory`/`dispatch_v2` and `run_one_plan`,
  which should also call `attach_safety_provenance`); `safety_provenance_restores_taint_after_restart`.
- 2026-10-02 (wk-tamper): Plan step 4 on work/gap-7147bb; cargo verification deferred to the batch check.
  - `PreparedGraphCheckpoint::open_safety_provenance` decodes `roko.safety-provenance@1` through
    `stored_safety_provenance`, which fails closed on another version of the namespace or an undecodable value. It
    then restores the sink with `GraphProvenanceSink::resume`, attaches it and writes the manifest. A run calls it
    before any task runs.
  - `resume` fails closed when the custody chain does not verify (`custody::chain_violations`, which
    `cmd_custody_verify` now shares), when a stored head is not in its log, or when a provenance custody record names
    a missing or altered witness vertex, or one with a missing parent. It also fails when the stored taint index is
    lower than what the run's records up to the stored custody head prove (`TaintTracker::levels`, new).
  - The run's records after the stored head, written after the last save, are tracked on top, so a crash between
    saves loses no taint. A checkpoint without the extension (fresh, or older) rebuilds the run's taint from the
    logs. Nothing resets to trusted while records exist.
  - Appends within one process are serialized (`APPEND_LOCK`). Two processes appending at once can still fork the
    custody chain, and the next resume then fails closed.
  - `register_known_namespaces` keeps the extension optional: older checkpoints have none, and the restore needs
    none.
  - Tests (roko-cli lib): `safety_provenance_restores_taint_after_restart`,
    `safety_provenance_restore_tracks_calls_after_the_last_save`, and the four fail-closed tests for a tampered
    custody log, a missing witness root, a taint downgrade and an unknown version.
  Still open: step 6 (no run opens the sink yet), step 5, and the policy fingerprints.
- 2026-10-02 (wk-tamper): Plan step 6 on work/gap-7147bb; cargo verification deferred to the batch check.
  - `AgentOptions::provenance_sink` (roko-agent) reaches `build_provider_tool_dispatcher`, which every API-provider
    tool loop uses (Anthropic, OpenAI-compatible, Cerebras, Gemini, Perplexity). The exhaustive `AgentOptions`
    literals in roko-agent, roko-cli tests, roko-dreams and roko-serve set it to `None`.
  - `run_graph_plan_body` gives the shared factory a `ProvenanceSinks` registry, and the factory hands it to each
    `AgentDispatcherV2`. `agent_options` picks the sink of the request's run (from its attempt key).
  - `run_one_plan` calls `checkpoint.open_safety_provenance` right after the checkpoint is prepared, so a resumed
    run's restore runs before any task. It registers the sink for the plan's lifetime, and a failed restore stops
    the plan. Every Graph plan run now records its API-provider tool calls.
  - CLI providers (Claude CLI, Codex CLI) run their own tool loops, so their calls are not recorded. Chat, serve and
    ACP attach no sink.
  - Tests: `agent_options_carry_the_runs_safety_provenance_sink` (roko-cli dispatch_v2) and
    `provenance_sinks_hold_a_runs_sink_while_it_is_registered` (roko-cli lib).
  Still open: step 5 (replay idempotency), the policy and contract fingerprints, and a live run showing
  `roko knowledge custody list` records (Done when).
- 2026-10-02 (wk-tamper): Plan step 5 on work/gap-7147bb; cargo verification deferred to the batch check.
  - A provenance record's identity is the hash of its canonical JSON (RFC 8785): call IDs (run, task, attempt,
    turn, call), argument digest, taint, and for an outcome its verdict, reason and result digest.
    `GraphProvenanceSink` acknowledges an intent it already holds with its first record's id, and skips an outcome
    it already holds, so neither is written twice. On resume it loads the run's records from the logs.
  - Whether an external effect happened stays with the activity and receipt ledger; an acknowledged intent does
    not prove it. A Graph resume re-dispatches with new S01 attempt keys, so a replay of the same attempt's calls
    only arises from a provider re-sending a call.
  - Test: `graph_provenance_sink_records_a_replayed_record_once` (roko-cli lib).
  Still open: the policy and contract fingerprints in the summary, and a live API-provider run showing
  `roko knowledge custody list` records, as the Done when asks.

## Original notes

[blocked] Blocked on #208 and #251; #282's registry contract is already frozen and aggregate #282 completion follows… — safety provenance is lost on restart and forensic components are not populated by live dispatch. `crates/roko-agent/src/safety/taint_propagation.rs::TaintTracker` maintains a…

Imported without verification from:
- `tmp/backlog/archive/351-durable-taint-witness-and-custody-provenance.md#351 — Persist Taint, Witness, and Custody Provenance Across Restart`

How to verify: Check: Every live tool/dispatch effect has an acknowledged pre-effect custody/witness record and a terminal result/denial record.; Signal-level taint ancestry and policy fingerprints survive process restart.; Restore occurs before new scheduling… [evidence: own status: Blocked on #208 and #251; #282's registry contract is already frozen and aggregate #282 completion follows…]

Verified 2026-09-28: still true. The cited files exist under crates/roko-agent/src/safety/. `TaintTracker` has atomic save/load (taint_propagation.rs:277-338), but no production code instantiates or restores it. `WitnessDag` and `WitnessLogger` have no users outside witness.rs (only re-exported at safety/mod.rs:110). `CustodyLogger` is used only by the read-side `roko knowledge custody list/show/verify` commands (crates/roko-cli/src/custody.rs:75, :150; commands/knowledge.rs:282-290). Live dispatch neither writes pre-effect witness/custody records nor persists taint ancestry.
