+++
id = "bug-4cb029"
kind = "bug"
title = "Quarantine routes read a file the runtime never writes"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/safety"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/safety.rs::quarantine_handler", "crates/roko-serve/src/routes/safety.rs::incidents_handler", "crates/roko-agent/src/tool_immune.rs::QUARANTINE_VAULT_RELATIVE_PATH"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "join(\"quarantine.json\")" crates/roko-serve/src/routes/safety.rs'

[[verify]]
command = "! grep -q 'join(\"quarantine.json\")' crates/roko-serve/src/routes/safety.rs && grep -q 'quarantine_vault_path' crates/roko-serve/src/routes/safety.rs && cargo test -p roko-serve routes::safety::tests::quarantine_route_sees_entry_written_by_immune_layer"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in b6906c9f8. Quarantine and incident routes read roko_agent::quarantine_vault_path; a missing vault reports vault_exists=false. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

The safety quarantine handlers read a `quarantine.json` file (`routes/safety.rs:48`, `:94`), but the tool immune layer persists its vault at `.roko/immune/quarantine-vault.json` (`roko-agent/src/tool_immune.rs:33`).
The routes always report an empty quarantine while the runtime is quarantining tool results.
Fix: read the vault through the runtime's path helper and type, and add a route test that sees an entry the immune layer wrote.

## Notes

- 2026-09-30 (wk-serve-sec): Implemented on `work/bug-928add` at `5257fe5e2`; cargo verification deferred to the batch check. Both routes read `roko_agent::quarantine_vault_path(workdir)` with `QuarantineVault::load`; a missing vault returns `vault_exists = false` and a `reason`, an unreadable or invalid one returns 500 instead of an empty default. The route test `routes::safety::tests::quarantine_route_sees_entry_written_by_immune_layer` records entries with the same path helper, type and calls as `tool_immune::update_vault` (which is `pub(crate)`; `screen_tool_result` is not reachable from roko-serve). Left as found: `entries` and `/incidents` list only `Pending` entries, so results the boundary escalates at once (`escalation_required`) are counted in `escalated` but not listed; `QuarantineVault` has no accessor for all entries. Plan-run attempts root their vault at the task lease (`graph_task_dispatch.rs` `immune_root: Some(effective_workdir)`, `graph_task_dispatch/streaming.rs` `immune_root: Some(lease.path)`), so worktree-rooted entries do not reach this workspace-level route.
