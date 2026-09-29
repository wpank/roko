---
plan: workspace-doctor-improvements
---

# Workspace doctor improvements

`roko doctor` covers config validity, provider credentials, disk health, layout
integrity, and provider credits. Two useful diagnostics are missing:

**Workspace map snapshot.** `generate_workspace_map_pub` already produces a
crate-tree summary to seed agent context. Operators debugging agent context
issues currently have no way to inspect what map an agent actually received
without triggering a full dispatch. Adding a `check_workspace_map` check writes
the current map to `.roko/workspace-map.md` during each doctor run and reports
its byte size, giving operators a durable, inspectable snapshot.

**Ignored-tests ledger.** The system prompt builder already references
`.roko/plans/ignored-tests.md` as the canonical ledger for `#[ignore]`-marked
tests (compose/templates/common.rs:231). There are currently 8 such markers
across the workspace, none of them tracked. Adding `check_ignored_tests_ledger`
counts the markers in `crates/**/*.rs` and warns when the ledger file is absent,
making the accumulation of ignored tests visible during routine health checks.

Both checks run sequentially: T01 adds the workspace map check, T02 depends on
it and adds the ignored-tests ledger check immediately after it in `run_checks`.
All changes are confined to `crates/roko-cli/src/doctor.rs`.
