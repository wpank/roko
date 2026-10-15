+++
id = "bug-429315"
kind = "bug"
title = "Demurrage balance tax charges an entry's whole age on every pass"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-neuro/knowledge"]
created = 2026-09-28
updated = 2026-10-15
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-neuro/src/knowledge_store/gc.rs::apply_demurrage", "crates/roko-neuro/src/lib.rs:703", "crates/roko-serve/src/lib.rs:2423"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/pub fn apply_demurrage(&self)/,/^    }/p' crates/roko-neuro/src/knowledge_store/gc.rs | grep -q 'signed_duration_since(entry.created_at)' && cargo test -p roko-neuro apply_demurrage_charges_only_elapsed_interval  (current `cargo test -p roko-neuro knowledge_store` passes while the bug exists)"
+++

`apply_demurrage` (`knowledge_store/gc.rs:203`) computes the tax from `now - created_at` (`:208-210`) on every pass instead of the time since the previous pass.
An entry is charged for its full lifetime again on each run, so old entries deplete (and freeze after 7 days) far faster than the configured rate.
Fix: store a last-taxed timestamp and tax only the elapsed interval; test that two passes an hour apart charge one hour.

2026-10-15 (status-quo assessment, finding 7): This held bug still runs: `roko serve` starts the knowledge-demurrage timer unconditionally (`crates/roko-serve/src/lib.rs:469`, `start_demurrage_timer` at `:2632`; re-checked at b42578bab), so the hold (dec-e70592) stops work on the bug but not the bug itself. Will's decision 12 (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`): build task 9231 (`tmp/backlog/2026-10-02-complete-and-wire/9231-held-serve-starts-no-demurrage-timer-by-default.md`) so that `roko serve` starts no demurrage timer by default. That stops this bug from running; the bug itself stays held with the rest of the held subsystems. The 9231 item is filed separately (from decision 12); link it here as `blocks`/`related` when it exists. Symptom here: each pass taxes an entry's balance for its whole age. Will's decision 12 (2026-10-15) turns the timer off by default: gap-8752ce.

2026-10-15 (status-quo decision 12, tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md): this bug stays held, but Will released backlog 9231 alone so a default `roko serve` stops running it: gap-8752ce puts start_demurrage_timer (crates/roko-serve/src/lib.rs:2632, spawned unconditionally at :469 at b42578bab) behind an off-by-default knowledge-decay feature. Once gap-8752ce is done, this bug only runs in a build with that feature. Will's decision 12 (2026-10-15) turns the timer off by default: gap-8752ce.
