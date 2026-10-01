+++
id = "bug-b14145"
kind = "bug"
title = "Hermes parser keeps only the first call of a tool_calls wrapper block"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::tool_call_from_value"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn parse_handles_tool_calls_array_wrapper_with_two_entries' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::parse_handles_tool_calls_array_wrapper_with_two_entries"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:21Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T17:37:10Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

`tool_call_from_value` (`translate/hermes.rs:217-227`) unpacks a `{"tool_calls": [...]}` body but returns only the first element; the rest are dropped without a warning, although the module doc (`:36`) says such arrays are unpacked.
Fix: return every entry in order with sequential ids; extend `parse_handles_tool_calls_array_wrapper` (`:479`) to two entries.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  `tool_call_from_value` and `parse_tool_call_body` return every call, so a `{"tool_calls": [...]}` wrapper gives
  one call per entry, in order. An entry without a `"name"` is skipped. Ids continue from the calls already parsed,
  across blocks too. Test: `parse_handles_tool_calls_array_wrapper_with_two_entries`.
