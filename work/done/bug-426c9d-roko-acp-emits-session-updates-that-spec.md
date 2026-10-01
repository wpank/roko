+++
id = "bug-426c9d"
kind = "bug"
title = "roko acp emits session updates that spec ACP clients drop"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp/protocol"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/types.rs::ToolCallKind", "crates/roko-acp/src/types.rs:701", "crates/roko-acp/src/types.rs:716", "crates/roko-acp/src/bridge_events/helpers.rs:48", "crates/roko-acp/src/bridge_events/mod.rs:993"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE '^\\s*Create,' crates/roko-acp/src/types.rs && ! grep -rqE 'SessionUpdate::(McpStatusUpdate|BudgetStatusUpdate)' crates/roko-acp/src --include=*.rs && cargo test -p roko-acp session_update_spec_conformance"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:11Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T17:36:42Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`ToolCallKind` has non-spec variants such as `Create` (`types.rs:741`), and roko sends non-spec `McpStatusUpdate` / `BudgetStatusUpdate` session updates (`:701`, `:716`); per a local probe, bare-text `tool_call_update` content is also rejected.
Spec SDK clients validate notifications and silently drop the ones that fail, so shell and edit calls vanish from client transcripts.
Fix: emit only spec kinds and content shapes (e.g. `create` -> `edit`, text wrapped as content blocks), carry extensions under `_meta`, and add golden fixtures validated against the spec schema.

## Notes

- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  Checked against the ACP v1 schema (release 1.24.1): `Create` is gone (`write_file` and added files are `edit`),
  `Terminal` serializes as `execute`, tool call content is sent as spec `ToolCallContent` (a bare unified diff
  becomes a fenced text block), locations are `{path, line}`, and MCP status and the budget ride on
  `session_info_update` under `_meta.roko`. `session_update_spec_conformance` checks every update roko emits against
  a vendored schema subset in `crates/roko-acp/tests/fixtures/`.
