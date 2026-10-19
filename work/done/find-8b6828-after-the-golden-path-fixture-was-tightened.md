+++
id = "find-8b6828"
kind = "finding"
title = "After the golden-path fixture was tightened, 2 of 3 independent reviews still count a T5 escape: initials keeps only the first character of a multi-character upper case (ß gives S), which no accept test covers"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["self-hosting"]
created = 2026-10-18
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "74a24a836"
source = "gp2 golden-path rerun reviews (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "find-4003b9"
anchors = ["crates/roko-cli/tests/fixtures/golden_path/plan/accept/alpha_initials.rs", "crates/roko-cli/tests/fixtures/golden_path/plan/accept/beta_badge.rs"]
links = { depends_on = [], blocks = [], related = ["find-4003b9", "gap-f30b8e", "find-5bcba7", "find-2ba86d", "spec-f09094"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qF 'initials(\"ßeta omega\"), \"SSO\"' crates/roko-cli/tests/fixtures/golden_path/plan/accept/alpha_initials.rs && grep -qF 'badge(\"ßeta\"), \"[SS] ßeta\"' crates/roko-cli/tests/fixtures/golden_path/plan/accept/beta_badge.rs && cargo test -p roko-cli --test golden_path_acceptance golden_path_fixture"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T12:18:32Z"
commit = "74a24a836"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-19T10:30:50Z"
forced = false
evidence = "74a24a836 tightens the golden-path fixture once more (option 2, as find-4003b9 did): crates/roko-cli/tests/fixtures/golden_path/plan/accept/alpha_initials.rs adds initials(\"ßeta omega\") == \"SSO\" and accept/beta_badge.rs adds badge(\"ßeta\") == \"[SS] ßeta\"; tasks.toml's T5 and T6 counts go 3 to 4; plan.md's \"Changed after the freeze (2026-10-19)\" says what and why; no task's description, files, order or tier changed. Checked at 74a24a836 through heavy.sh: on a seed copy with the replay's alpha and beta (hb-escapecheck.sh), both new cases pass on the replay solution (4 of 4 each) and fail on runs 1 and 3's escape (`to_uppercase().next()`: 3 of 4 each, only the new case failing); golden_path_fixture_is_red_on_the_seed and golden_path_fixture_plan_merges_green pass (cargo test -p roko-cli --test golden_path_acceptance: 2 passed, live ignored); `roko plan validate plans/golden-path` in a seeded copy: 0 diagnostics. The [[verify]] fails before 74a24a836 (its greps find no ß case) and passes after. Whether reviews now find no escape is gap-f30b8e's live rerun."
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

## Done (2026-10-19): option 2

The fixture was tightened once more, as find-4003b9's option 2 did on 2026-10-18:

- T5's `accept/alpha_initials.rs` adds `initials("ßeta omega") == "SSO"`, and T6's `accept/beta_badge.rs` adds
  `badge("ßeta") == "[SS] ßeta"`; their counts in `tasks.toml` go from 3 to 4.
- The frozen plan's `plan.md` says what changed after the freeze and why ("Changed after the freeze
  (2026-10-19)"), and `tasks.toml`'s header comment names it. No task's description, files, order or tier
  changed.

Checked before any live run: the new cases pass on the fixture's replay solution and fail on the escape the
reviews found (`to_uppercase().next()`, runs 1 and 3), while the older cases pass on both; both fixture
tests pass (`golden_path_fixture_is_red_on_the_seed`, `golden_path_fixture_plan_merges_green`); and
`roko plan validate` reports 0 diagnostics. The live pass^3 on this fixture is gap-f30b8e's.
