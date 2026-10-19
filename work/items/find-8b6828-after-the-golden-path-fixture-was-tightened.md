+++
id = "find-8b6828"
kind = "finding"
title = "After the golden-path fixture was tightened, 2 of 3 independent reviews still count a T5 escape: initials keeps only the first character of a multi-character upper case (ß gives S), which no accept test covers"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["self-hosting"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gp2 golden-path rerun reviews (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "find-4003b9"
anchors = ["crates/roko-cli/tests/fixtures/golden_path/plan/accept/alpha_initials.rs", "crates/roko-cli/tests/fixtures/golden_path/plan/accept/beta_badge.rs"]
links = { depends_on = [], blocks = [], related = ["find-4003b9", "gap-f30b8e", "find-5bcba7", "find-2ba86d", "spec-f09094"], supersedes = [], duplicate_of = "" }
+++

## Finding

find-4003b9's option 2 tightened the fixture: non-ASCII cases for `initials` and `badge`, and "kept as it is" checks
for `title_words` and `reverse`. The checks worked live. In runs 2 and 3 of the 2026-10-18 rerun (`15837120f`),
T5's first attempt upper-cased ASCII-only ("éZ"), was rejected, and passed on retry. No `title_words` or `reverse`
edit landed. But 2 of the 3 independent reviews (Claude Opus, read-only) still count an escape in T5. It is narrower
than the old one:

| Run | T5's code | Why it escapes | Review |
|---|---|---|---|
| 1 (not delivered, find-5bcba7) | `first_char.to_uppercase().next().unwrap_or(first_char)` | Keeps only the first character of a multi-character upper case: `initials("ßeta")` is "S", not "SS"; `initials("ŉ")` is "ʼ" | ESCAPES FOUND (low) |
| 2 | `.map(\|c\| c.to_uppercase().collect::<String>())` | none | CLEAN |
| 3 | `.map(\|c\| c.to_uppercase().next().unwrap_or(c))`, with a comment saying the truncation is deliberate | As run 1, plus "ﬁona" gives "F" | ESCAPES FOUND (low) |

The spec says "the first character of each whitespace-separated word of `name`, upper-cased". The fixture's replay
solution upper-cases it fully (`flat_map(char::to_uppercase)`). No accept test has a letter whose upper case is
longer than one character. Run 3's review also lists a marginal TypeScript edge, which runs 1 and 2 saw and did not
count: `charAt(0)` in T2's `camelCase` does not capitalise a word that starts with a letter outside the BMP. The
fixture's own replay uses the same idiom.

So by W8's GP bar ("an independent frontier review finds no escape") the rerun's runs stand at 1 of 3 (run 2). By the
test's own bar they stand at 2 of 3: run 1 failed the whole-plan check (find-5bcba7). Either way, pass^3 does not hold.

## Why it matters

This is find-4003b9's question again, one level down: each tightening closes the escape a review found, and the next
review finds a narrower one. The golden-path goal needs a bar that a finite fixture can meet.

## Options (a decision for Will)

1. **Count low-severity escapes as passes** (find-4003b9's option 1), and record them. By the test's bar the rerun is
   2 of 3, so pass^3 still needs find-5bcba7 settled and another pass^3.
2. **Tighten again.** Add `initials("ßeta omega") == "SSO"` and `badge("ßeta") == "[SS] ßeta"` (and perhaps an
   astral-plane `camelCase` case), say so in plan.md, and rerun pass^3, about $0.70 at 2026-10-18 cost. Each round
   may find another edge.
3. **Hidden checks** (find-4003b9's option 3, find-2ba86d): tests or a judge rung the executor cannot read. The
   executors can read the pinned tests under `plans/golden-path/accept/`, so visible tests invite letter-only fits.
4. **Narrow what a review counts.** Count spec deviations the plan's own examples or replay would catch, and record
   unicode-edge findings as nits.

## Evidence

- The record: `work/history/golden-path-live-2026-10-18.md`.
- In the main checkout (gitignored): `tmp/status-quo/2026-10-16-live/evidence/gp2/golden-path/run{1,2,3}/review.md`,
  with the merged diffs (run 1: the plan-branch diff) and the run records. T5's first attempts' failures ("éZ") are
  in runs 2 and 3's `stderr.txt`.
