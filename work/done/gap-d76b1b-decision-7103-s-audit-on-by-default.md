+++
id = "gap-d76b1b"
kind = "gap"
title = "Decision 7103's audit-on-by-default outcome was never implemented: enabled stays false, no --no-audit flag"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-core/config", "roko-cli/audit"]
created = 2026-10-03
updated = 2026-10-18
last_verified = 2026-10-18
last_verified_rev = "22ac6f06b"
source = "wave-7 follow-up reports 2026-10-03 (PK59 gap-147c4d)"
discovered_from = "gap-147c4d, decision 7103"
anchors = ["crates/roko-core/src/config/audit.rs::AuditConfig", "crates/roko-cli/src/audit/worker.rs", "crates/roko-cli/src/graph_task_dispatch/audit_select.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn default_audit_config_is_enabled_at_the_decided_rate' crates/roko-core/ && cargo test -p roko-core default_audit_config_is_enabled_at_the_decided_rate"

[closed]
at = 2026-10-18
at_ts = "2026-10-18T19:03:30Z"
commit = "22ac6f06b"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-18T17:10:15Z"
forced = false
evidence = "22ac6f06b, decision 7103 (b), after q-e916e4 (8f52029eb): crates/roko-core/src/config/audit.rs AuditConfig::default() now has enabled = true; rho 0.10, eps_floor 0.05, per_audit_usd 0.30, per_audit_cpu_secs 900, budget_frac 0.12 and phase_b_rate 0.5 already matched, so a workspace without [audit] audits. --no-audit on roko run and roko plan run reaches apply_run_switches (plan_runner.rs) through RunArgs/PromptRun/GraphPlanRunParams.no_audit, as --no-holdout does, and sets [audit] enabled = false for the run; plan run --no-audit refuses when a server owns the workspace. The per-audit cap now reaches the call: B1's FactoryAuthor and B3's review (CheapFactoryAgent through the new ReviewAgent) carry an AuditAllowance (audit/worker.rs) of the check's usd_left, min(per_audit_usd, budget_frac*run spend - spent), which stops roko's tool loop once spent, refuses after an unpriced call, and gives the Claude CLI --max-budget-usd; Codex/Gemini/Cursor CLI reviewers take no limit (gap-ccbf26), and one call can still overshoot (filed as a new item). Test-safety sweep: dispatcher fixtures go through fence_audits/fixture_config, plan-run fixtures write [audit] enabled = false, spawned roko gets ROKO__AUDIT__ENABLED=false (Isolated, 8 spawners); a temporary probe at the 3 vault-resolving sites, over all roko-cli lib and bin unit tests with ROKO_AUDIT_HOME on scratch, found 55 unfenced runs before and 0 after, and ~/.roko/audit does not exist after the runs. Verify: cargo test -p roko-core default_audit_config_is_enabled_at_the_decided_rate, 1 passed. Also: cargo test -p roko-core --lib 2042 passed; cargo test -p roko-cli --lib 3827 passed and --bin roko 472 passed (new: audits_run_by_default_and_no_audit_turns_them_off, a real in-process roko run drawing its pass at pi 0.10; no_audit_flag_turns_the_runs_audits_off; audits_spend_within_the_per_audit_cap_and_the_runs_budget; audit_allowance_refuses_calls_past_its_limit); 8 integration test binaries (17 tests) incl. audit_canary_drill pass; roko run/plan run --help list --no-audit; cargo +nightly fmt and cargo clippy -p roko-core -p roko-cli --no-deps -- -D warnings clean."
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

2026-10-15 (status-quo decision 2, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md): Will confirmed audits go on at 10% by default, but only after a minimum-sample floor stops the strictness ladder from escalating on noise (q-394b7f). Add links.depends_on = [bug-1e97dc] ('The audit strictness ladder steps up on sampling noise: it must ignore windows below a minimum effective sample') and parent = the epic 'turn the cybernetic layer on in safe stages' (stage 4). Do not claim this item before bug-1e97dc is done. Re-checked at b42578bab: [audit] enabled still defaults to false (crates/roko-core/src/config/audit.rs:71), no --no-audit flag, and gap-618548 (M4 verdicts into M1's E3) should land first so the first real audits also feed M1.

## Decision (2026-10-18, Will)

Unblocked: q-e916e4 was answered on 2026-10-18 (pool quiet windows), so the ladder can step down again. Turn audits on at the decided rate once q-e916e4's change lands.
