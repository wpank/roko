+++
id = "gap-d76b1b"
kind = "gap"
title = "Decision 7103's audit-on-by-default outcome was never implemented: enabled stays false, no --no-audit flag"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-core/config", "roko-cli/audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK59 gap-147c4d)"
discovered_from = "gap-147c4d, decision 7103"
anchors = ["crates/roko-core/src/config/audit.rs::AuditConfig", "crates/roko-cli/src/audit/worker.rs", "crates/roko-cli/src/graph_task_dispatch/audit_select.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_audit_config_is_enabled_at_the_decided_rate' crates/roko-core/ && cargo test -p roko-core default_audit_config_is_enabled_at_the_decided_rate"
+++

## Problem

Decision 7103 (`tmp/cybernetic-harness/DECISIONS.md` D7-audits) is accepted: "Option (b): audit 10% of passes on
real work by default (floor 5%, at most $0.30 per audit and 12% of run spend, `--no-audit` to opt out), switched
on once the audit worker lands (7123)." The audit worker has landed: `crates/roko-cli/src/audit/worker.rs` exists
and `graph_task_dispatch/audit_select.rs::a_planted_test_detection_yields_an_audit_result_with_an_a1_finding`
(backlog task 7123's own named test) passes. But:

- `crates/roko-core/src/config/audit.rs:71` still defaults `enabled: false` — no task flipped the default once
  7123 landed.
- There is no `--no-audit` CLI flag anywhere (`grep -rn 'no.audit' crates/roko-cli/src/main.rs crates/roko-cli/src/commands/` is empty) — the decision's own opt-out mechanism doesn't exist.

So decision 7103's precondition is met but its outcome was never implemented.

## Why it matters

Goal: cybernetic (M4 deep audits). Every estimate S05 depends on needs a logged π for each green unit (7121's own
"Why it matters"), and that only happens when `[audit] enabled` is true. Right now it is false everywhere by
default, so no real workspace is actually producing audit selections or results despite the worker existing and
despite Will having already decided this should be on.

## Where

- `crates/roko-core/src/config/audit.rs::AuditConfig::default` (line 71, `enabled: false`).
- The CLI flag surface: `crates/roko-cli/src/main.rs` / `crates/roko-cli/src/commands/plan.rs` / `run_cmd.rs`
  (wherever other per-run opt-out flags like the resolved `--no-holdout`, gap-29fe0a, are/will be wired — mirror
  that pattern for `--no-audit`).
- The audit worker and selection path this unblocks: `crates/roko-cli/src/audit/worker.rs`,
  `crates/roko-cli/src/graph_task_dispatch/audit_select.rs`.

## Current state

7123 (the audit worker) and 7121 (the lottery draw) are both implemented and tested. 7113 (the `[audit]` config
section) exists with `enabled: false` as its default. Nothing in the backlog's 7xxx (M4 deep-audits) range flips
that default or adds the opt-out flag — decision 7103 names both as part of its accepted outcome but no task
implements either.

## Plan

1. Flip `AuditConfig::default().enabled` to `true` (or compute it from `rho = 0.10` with a 0.05 floor, per the
   decision's exact numbers), gated behind the test-safety prerequisite below.
2. Add a `--no-audit` flag to whichever `roko run`/`roko plan run` surface already carries `--no-holdout` (once
   gap-29fe0a lands, follow its same wiring pattern for consistency), setting `enabled = false` for that run.
3. **Before step 1 ships**, audit every test that constructs a default config without pinning `ROKO_AUDIT_HOME`
   (or an explicit `[audit]` override) to a temp dir — once `enabled` defaults to `true`, any such test would
   write real audit records to the operator's actual `~/.roko/audit` vault instead of a fixture directory. Grep
   `crates/` for config construction in tests that doesn't set `ROKO_AUDIT_HOME`/`audit.enabled = false` first, and
   fix those before flipping the default, not after.

## Done when

- A workspace with no explicit `[audit]` section runs audits at ρ = 0.10 (floor 0.05) by default.
- `--no-audit` (or the plan-run equivalent) turns it off for one run.
- No test in the workspace writes to the real `~/.roko/audit` as a side effect of the new default.
- The `[[verify]]` command passes.

## Notes

- Do not flip the default before the test-safety sweep (step 3) is done — this is explicitly called out because
  getting the order wrong would pollute the real audit vault during `cargo test --workspace`.
- Related: 7121, 7123 (both done, both depend on `[audit] enabled` actually being true to run for real).
