+++
id = "gap-398dee"
kind = "gap"
title = "A phase-B audit call can cost more than its audit has left: nothing bounds one call before it starts, and B3's brief carries the whole test-file diff"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/audit"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gap-d76b1b worker 2026-10-18"
discovered_from = "gap-d76b1b"
anchors = ["crates/roko-cli/src/audit/b3.rs::brief", "crates/roko-cli/src/audit/b1.rs::FactoryAuthor", "crates/roko-cli/src/audit/worker.rs::AuditAllowance"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-d76b1b", "gap-73c98e", "gap-ccbf26"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn b3_brief_caps_the_test_changes' crates/roko-cli/ && cargo test -p roko-cli --lib b3_brief_caps_the_test_changes"
+++

## Problem

Decision 7103 caps each audit at `[audit] per_audit_usd` ($0.30) and all of a run's audits at `budget_frac` (12%) of
its model spend. Since gap-d76b1b, B1's author call and B3's review carry an `AuditAllowance` of what the audit has
left (`crates/roko-cli/src/audit/worker.rs::AuditAllowance`): roko's tool loop starts no call once the allowance is
spent, the Claude CLI gets what is left as `--max-budget-usd`, and a check given $0 makes no call.

What is still unbounded is the call that crosses the cap: its cost is known only once it returns. B3 sends the judge
a prompt whose diff is cut at 30 KB (`roko_core::defaults::DEFAULT_MAX_DIFF_BYTES`), but its brief
(`crates/roko-cli/src/audit/b3.rs::brief`) appends the whole test-file diff (`test_diff`), uncut. B1's author
(`crates/roko-cli/src/audit/b1.rs::FactoryAuthor::write`) may write as many output tokens as the model allows. On an
expensive reviewer or author, one review of a large test diff, or one long suite, can cost more than the $0.30 alone.

## Why it matters

Goal `cybernetic`, epic spec-7a71d6 stage 4. Audits run by default since gap-d76b1b, and the per-audit cap is part of
decision 7103's outcome. A cap that only holds between calls can be overrun by one call on a real run.

## Where

- `crates/roko-cli/src/audit/b3.rs::brief` and `test_diff`: the test-file diff goes into the brief in full.
- `crates/roko-cli/src/audit/b1.rs::FactoryAuthor::write`: no output limit on the author's reply.
- `crates/roko-cli/src/audit/worker.rs::AuditAllowance`: it refuses the next call only after one has spent the cap.

## Current state

At 22ac6f06b (gap-d76b1b), the allowance bounds every call after the first, and B1 and B3 make none with $0 left.
Nothing estimates a call's cost before it starts. A Codex, Gemini or Cursor CLI reviewer takes no spend limit at all
(gap-ccbf26).

## Plan

1. Cut B3's test diff in its brief at the judge's diff cap (or a share of it), and say so in the brief, as the judge
   marks a cut diff `...[truncated]`.
2. Optionally, before a B1 or B3 call, estimate its input cost from the prompt's size and the model's price row, and
   leave the check null ("the call would cost more than the audit has left") when the estimate alone exceeds
   `usd_left`.

## Done when

- `b3_brief_caps_the_test_changes` passes: a review whose test-file diff is larger than the cap sends at most the cap
  of it, marked as cut.
- The `[[verify]]` command passes.

## Notes

- Found while doing gap-d76b1b (2026-10-18), which made audits run by default.
- Do not change `per_audit_usd`, `budget_frac` or ρ (decision 7103).
