+++
id = "q-63c19f"
kind = "question"
title = "No production writer for learn/gate-gaming-alerts.jsonl; may audit alerts leave the vault as a redacted mirror?"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/audit"]
created = 2026-10-04
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "b883db4e6"
source = "wave-15 follow-up reports 2026-10-04 (gap-54b2b2, gate 16a)"
discovered_from = "gap-54b2b2 (open, work/bug-19ae56; own Progress note names this finding)"
anchors = ["crates/roko-learn/src/gate_gaming.rs::GateGamingDetector", "crates/roko-cli/src/audit/worker.rs::GamingWatch"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-15
at_ts = "2026-10-15T16:30:19Z"
commit = "b883db4e6"
by = "status-quo-2026-10-15"
executor = "claude-session"
via = "manual"
forced = true
evidence = "2026-10-15 decision (Will delegated it to the coordinator's recommendation, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md, decision 9; he can veto it): Option C: no. Audit gate-gaming alerts stay in the vault, with no redacted mirror and no vault read from roko diagnose. Instead, diagnose labels its gate_gaming_alerts as the legacy judge-feed source, so an empty list no longer reads as 'no gaming' (follow-up). Why: The vault sits outside every workdir so agents cannot reach it (crates/roko-core/src/audit_home.rs:1-14), and GamingWatch writes its alerts there (crates/roko-cli/src/audit/worker.rs:302). Agents can run roko diagnose from a shell, so option B (diagnose reads the vault) would carry vault-derived data across that boundary. Option A (mirror into .roko/learn) would put alert timing in an agent-readable file, which can reveal which attempts were audited and undermines the lottery. Audits are off by default (crates/roko-core/src/config/audit.rs:71, enabled: false), so no vault alerts exist yet and nothing is lost today. The one real harm is that diagnose (crates/roko-cli/src/commands/diagnose.rs:519-540) shows a stale file as if it were current; a label fixes that cheaply. Follow-up: gap-610928. If vetoed: With option A, a new item adds a minimal redacted row (model, timestamp, deltas) to .roko/learn from GamingWatch, plus a test that no hidden-test content or selection id leaks. The label follow-up is then dropped."
+++

## Problem

Nothing in production writes `.roko/learn/gate-gaming-alerts.jsonl` any more, so `roko diagnose`
shows only alerts older runs wrote, even though a real (and arguably better-grounded) detector
is actively running elsewhere. `GateGamingDetector::observe_and_detect`
(`crates/roko-learn/src/gate_gaming.rs:295-322`), the only thing that calls `append_alert`
(the writer, line 332), has zero production call sites anywhere in `crates/` — only its own
module doc comments and its own test reference it. Decision 4108 removed the judge feed (the
LLM quality-rating helper call `observe_and_detect`'s `quality_score` needed): a regression test
in `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs:713-714` pins the now-correct
behavior directly — `assert!(!alerts.exists(), "{} was written", alerts.display())` against
`.roko/learn/gate-gaming-alerts.jsonl`.

Gate-gaming detection itself didn't go away: `GamingWatch` (`crates/roko-cli/src/audit/worker.rs:257-266`)
wraps the same `GateGamingDetector`, but points it at
`vault.incidents_dir().join("gate-gaming-alerts.jsonl")`
(`crates/roko-core/src/audit_home.rs:151-155`) — a file under the audit *vault's* own root
(alongside `hidden/`, the hidden test suites that must stay secret from the agent under audit),
not the general workspace's `.roko/learn/`. Its detection signal is arguably more trustworthy
than the old judge feed's LLM self-rating: "each settled attempt adds its gate verdict at
weight 1, and each audited label adds quality 1 − Y at weight 1/π_i" (`worker.rs:250-251`) —
i.e. it scores against S05's real audited ground truth, not an LLM's opinion of its own work.
`roko diagnose`'s `run_gaming_alerts` (`crates/roko-cli/src/commands/diagnose.rs:519-540`) only
ever reads the `learn/` path, so it never sees these.

## Why it matters

Goal: truth, the one diagnostic surface gate-gaming has (`roko diagnose`'s `gate_gaming_alerts`
field). The signal exists and may be more accurate than before, but it's trapped behind the
vault's access boundary, which exists for a good reason (the hidden test suites it also holds
must never leak to the audited agent) — so simply pointing `diagnose` at the vault's incidents
directory isn't safe without first deciding what, if anything, is safe to expose from it.

## Where

- `crates/roko-learn/src/gate_gaming.rs::GateGamingDetector`, `::append_alert` (the shared
  detector machinery; unchanged either way).
- `crates/roko-cli/src/audit/worker.rs::GamingWatch` (the vault-side instance actually running).
- `crates/roko-core/src/audit_home.rs::AuditVault::incidents_dir` (the vault boundary).
- `crates/roko-cli/src/commands/diagnose.rs::run_gaming_alerts` (the reader that only looks at
  the now-silent `learn/` path).

## Plan (decision needed)

- **Option A — redacted mirror.** When `GamingWatch` raises an alert, also append a
  deliberately minimal, redacted row (e.g. model slug, timestamp, pass/quality deltas — nothing
  that could reveal a hidden test's content or identity) to `.roko/learn/gate-gaming-alerts.jsonl`,
  so `diagnose` keeps working unchanged and the vault boundary stays intact for everything else.
- **Option B — point `diagnose` at the vault**, behind whatever access check already gates
  other vault reads, instead of (or in addition to) the `learn/` path. Simpler, but widens who
  can read vault-adjacent data and by how much needs its own review.
- **Option C — accept the gap.** Document that `roko diagnose`'s gate-gaming alerts are stale
  once an audited workspace exists, and point operators at the vault's own tooling (`roko
  knowledge custody` or similar) for current gaming signals instead.

## Done when

Will picks an option; if A or B, `roko diagnose` shows alerts the audit worker actually raised,
with a regression test proving the vault boundary isn't weakened for anything beyond the
specific fields the chosen option exposes.

## Notes

- 2026-10-04 (wave-15 follow-up, gap-54b2b2, gate 16a not yet merged): confirmed on
  `work/bug-19ae56` (gap-54b2b2's plan/run-id fix is implemented there too, unrelated to this
  finding) and independently on `main` at `7d82b944a` (every file cited above except the
  `gap-54b2b2` item text itself is unchanged by that branch's diff, so the vault-vs-learn split
  is verifiable on `main` directly). `gap-54b2b2`'s own Progress note names this exact finding
  almost verbatim; filed as `kind = "question"` per the instruction.

2026-10-15 (status-quo assessment at 7b6a5af44, finding 25; tmp/status-quo/2026-10-15/backlog/A02-21-22-honest-measurement.md row 2125, A08-71-m4-deep-audits.md lines 122-123): unchanged at HEAD, and the assessment counts backlog task 2125 (`roko diagnose` lists gate-gaming alerts) as REGRESSED, a hollow done. `run_gaming_alerts` (`crates/roko-cli/src/commands/diagnose.rs:521-541`) still reads only `.roko/learn/gate-gaming-alerts.jsonl`, which nothing in production writes: `5a3e2be50` (2026-10-02, task 7105) removed the Graph-side detector before PK08 closed, and `09d4787d4` (2026-10-03) moved the only live detector to the audit worker, which writes `vault.incidents_dir().join("gate-gaming-alerts.jsonl")` (`crates/roko-cli/src/audit/worker.rs:302`); only `roko audit status` reads that. The 2125 test passes only because it writes the learn-side file itself (`diagnose.rs:2670`), and `graph_execution/plan_runner.rs:5566` still lists `learn/gate-gaming-alerts.jsonl` among a run's outputs. Even a vault-side reader would show nothing today: the audit worker runs only with `[audit] enabled = true`, and the default is false (`crates/roko-core/src/config/audit.rs:71`); Will's 2026-10-15 decision 2 turns audits on at 10% only after q-394b7f's minimum-sample floor lands. Whatever this question decides, the 2125 verify should change with it, so it fails when no production writer feeds the file `roko diagnose` reads.
