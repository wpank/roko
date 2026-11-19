+++
id = "bug-399240"
kind = "bug"
title = "roko learn all and roko learn efficiency rewrite the whole efficiency log on every call, outside the lock its appenders hold: a row a run appends meanwhile is lost, and since bug-808700 these views run beside roko serve's runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "S"
subsystem = ["roko-learn/efficiency", "roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "efb53e6e6"
source = "round-v8 roko-learnguard survey ($V8/roko-learnguard/survey, roko a0089c19c, a copy of Undertow's .roko)"
discovered_from = "agent:roko-learnguard (round v8)"
anchors = ["crates/roko-learn/src/efficiency.rs::null_ambiguous_gate_failed_entries", "crates/roko-cli/src/commands/learn.rs::print_learn_efficiency"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-808700", "gap-ea1576"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_gate_passed_migration_waits_for_the_append_lock' crates/roko-learn/ && cargo test -p roko-learn the_gate_passed_migration_waits_for_the_append_lock"

[[verify]]
command = "grep -rqw 'fn the_gate_passed_migration_leaves_a_clean_log_alone' crates/roko-learn/ && cargo test -p roko-learn the_gate_passed_migration_leaves_a_clean_log_alone"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T14:21:28Z"
commit = "efb53e6e6"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-11-12T13:21:01Z"
forced = false
evidence = "efb53e6e6. crates/roko-learn efficiency.rs: null_ambiguous_gate_failed_entries takes roko_fs::log_rotation::lock_jsonl, the lock the log's appenders and rotation hold, before it reads and holds it through the rename, as compact_jsonl_to_latest_sync does, and writes nothing when no line needs patching (every log written since P0-GA-1). Red first on cb18faf10: the_gate_passed_migration_leaves_a_clean_log_alone (main rewrote the clean log, its keys re-sorted) and the_gate_passed_migration_waits_for_the_append_lock (main's migration finished while the test held the lock) both failed. Green: both pass with the three null_ambiguous_gate_failed tests; roko-learn lib 1415/0; roko-cli lib and bins filtered to learn 55/0, 0/0 and 51/0; learn_paths_fixture 2/0, reflexes_cli 3/0; clippy -p roko-learn -D warnings and nightly fmt clean (tmp/hermes-roko-v8/roko-learnguard/logs/399240-chain.log). Live, $0: the branch's roko over copies of Undertow's and roko's workspaces: roko learn all and roko learn efficiency leave .roko/learn/efficiency.jsonl the same bytes in the same file (bin/changed.py); they create the empty efficiency.jsonl.lock the copies lacked, which both real workspaces already hold. Main b81f8a915 replaced the log on every roko learn all (survey-after/*/writes.txt)."
+++

## Problem

`roko learn all` and `roko learn efficiency` rewrite `.roko/learn/efficiency.jsonl` every time they run. On a copy of
round v7's Undertow workspace (roko a0089c19c, 2026-11-12), one `roko learn all` left the log's 372 records the same as
JSON and rewrote every line with its keys re-sorted.

- `print_learn_efficiency` runs audit #80's one-shot migration, `roko_learn::efficiency::null_ambiguous_gate_failed_entries`,
  on every call. The migration re-serialises every line and renames a `.tmp` sibling over the log even when it patches
  nothing. On the two logs checked it patches nothing: 0 of 372 rows on Undertow, 0 of 1,524 on roko's own. Every
  `gate_passed: false` row there records a failure.
- It does this without `roko_fs::log_rotation::lock_jsonl`, the lock that the log's appenders (`append_jsonl_line_sync`)
  and its rotation hold. So a row that a run appends between the migration's read and its rename is lost, and a
  rotation in that window puts the archived rows back into the live log.
- A CLI `roko plan run` holds the runner lock, not the workspace lock, so these views could always overlap one. Since
  bug-808700 (c92e4bee4) they also run beside the workspace's roko serve, which runs Hermes's plans and appends to this
  log.

## Why it matters

- Looking at what roko learned should not change it or lose a record of it. This log feeds the budget predictor's
  calibration (`budget_predictor::load_or_calibrate`), the efficiency trend (`roko_learn::aggregate`) and the homeostat's
  resolutions (`homeostasis::resolution`).
- The round v8 guides will tell people to run `roko learn all` while roko serve is up.
- Goal: learning.

## Where

- `crates/roko-learn/src/efficiency.rs`: `null_ambiguous_gate_failed_entries`.
- `crates/roko-cli/src/commands/learn.rs`: `print_learn_efficiency`, its caller.

## Plan

1. **Lock it.** The migration takes `lock_jsonl(path)` before it reads and holds it through the rename, as
   `compact_jsonl_to_latest_sync` does, so an append or a rotation waits for it.
2. **Write only what needs it.** When no line needs patching, as in every log written since P0-GA-1, it returns 0 and
   writes nothing.
3. **Tests:**
   - `the_gate_passed_migration_leaves_a_clean_log_alone`: the same bytes in the same file (inode).
   - `the_gate_passed_migration_waits_for_the_append_lock`: while the test holds the lock the migration waits, and a row
     appended meanwhile is still there afterwards.

## Done when

- Both `[[verify]]` commands pass.
- On a copy of Undertow's workspace, `roko learn all` leaves `.roko/learn/efficiency.jsonl` byte for byte as it was.

## Notes

- Found by roko-learnguard's round v8 survey: the survey changed only this file in the workspace copies, apart from
  the empty log and lock files (`$V8/findings/roko-learnguard.md`).
- Related: bug-808700 (the views read beside serve), gap-ea1576 (`learn all`'s text).
