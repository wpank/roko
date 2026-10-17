+++
id = "gap-ca63ee"
kind = "gap"
title = "roko learn loops judges loops with S03's default parameters while the audit tick uses [learning.audit], so the two can disagree"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/learn"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "0dace1e16"
source = "gap-03fcd0 follow-up 2026-10-16"
discovered_from = "gap-03fcd0"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::run_with", "crates/roko-cli/src/commands/learn_loops.rs::loops_output"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-03fcd0", "spec-7a71d6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_loops_census_reads_learning_audit_params' crates/roko-cli/ && cargo test -p roko-cli --lib learn_loops_census_reads_learning_audit_params"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T04:09:43Z"
commit = "0dace1e16"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T02:10:49Z"
forced = false
evidence = "0dace1e16. state.rs AuditParams::configured(config, registry) is now the one mapping of [learning.audit], kappa_max included, used by LoopAuditor::params and by census::run_with; census::run_audited/run_in/run_with take the LearningAuditConfig and census::run stays the defaults wrapper (loop_census example). The fold level is alpha/K plus kappa_max: Tally's BenefitConfig takes the configured kappa_max and CensusState records it (serde default 2.0), so a census kept at another kappa_max is folded afresh. learn_loops.rs loops_output loads load_config_unified(workdir) and calls census::run_audited. Checks: the verify command exits 0 (learn_loops_census_reads_learning_audit_params: n_beta = 100, 150 opportunities, beta in the --json output equal to the tick's health row; defaults say n_below_n_beta); roko-learn loop_audit 56/56 incl. census_folds_benefit_at_the_configured_kappa_max; roko-cli learn_loops/loop_audit/decision_log/loop_canary 29/29; E1 (cargo test -p roko-cli --features fault-injection --test loop_audit_faults) passes; clippy -p roko-learn -p roko-cli --no-deps -D warnings clean. target/debug/roko --json learn loops on a scratch copy of the round-v6 workspace: L-know beta reason eps_below_min with defaults, beta est -0.3125 with [learning.audit] eps_min = 0.2, n_beta = 2."
+++

## Problem

`roko learn loops` (and roko-learn's `loop_census` example) judge every loop with S03 §4.10's default parameters,
while the audit tick judges with the workspace's `[learning.audit]`. `census::run_with` builds
`AuditParams { enforced_loops: K, ..AuditParams::default() }`, so N_ε, ε_min, N_β, N_null, δ_min, δ_ni and α come from
the defaults whatever `roko.toml` says. The tick's `LoopAuditor::params` maps `[learning.audit]` onto them. Neither
passes `[learning.audit] kappa_max` on: the census's benefit estimators always use `KAPPA_MAX` (2). So a workspace that
sets, say, `n_beta = 100` gets health rows with β from 100 opportunities while `roko learn loops` still says
`n_below_n_beta`, and the census can call a loop `live` or `harm` that the ledger does not, or the reverse.

## Why it matters

Since gap-03fcd0 the census prints `live`, `demoted`/`harm` and `flagged`/`null`, and ViabilityBench's `requires_live`
reads `roko learn loops --json`. The command should agree with the ledger it summarizes. Related: gap-03fcd0,
spec-7a71d6.

## Where

- `crates/roko-learn/src/loop_audit/census.rs::run_with`: the census's `AuditParams`.
- `crates/roko-learn/src/loop_audit/mod.rs::LoopAuditor::params`: the tick's, from `LearningAuditConfig`.
- `crates/roko-learn/src/loop_audit/census.rs::Tally::new`: the benefit estimators' `BenefitConfig` (α only).
- `crates/roko-cli/src/commands/learn_loops.rs::loops_output`: the CLI entry, which loads no config.

## Current state

The defaults equal `[learning.audit]`'s defaults, so the two agree unless a workspace changes the section.

## Plan

1. Let `census::run_with` take the `AuditParams` (or the `LearningAuditConfig`), with today's defaults as a wrapper.
2. Have `loops_output` load the workspace's `[learning.audit]` and pass it; keep the example on the defaults or give it
   a flag.
3. Carry `kappa_max` into the census's `BenefitConfig` (and into `CensusState`'s fit check, since the kept estimators
   depend on it).

## Done when

`learn_loops_census_reads_learning_audit_params` passes: with `[learning.audit] n_beta = 100` in `roko.toml`, `roko
learn loops --json` reports β for a loop with 150 opportunities, as the tick's health row does.

## Notes

Report-only: no executed policy changes.
