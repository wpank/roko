+++
id = "bug-3e3d08"
kind = "bug"
title = "Retry feedback for a C++ doctest failure names only part of it: GateFeedback::from_raw doesn't recognise doctest, Catch2 or gtest failure lines, step_failure keeps a blind 30-line tail, and the deterministic diagnosis repeats one fragment (Undertow phase 4.2 run 1447385e T1#1: 2 failing cases, the retry named 1 in full and the other by half a line)"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/dispatch", "roko-learn"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "189d23bf2"
source = "round-v7-hermes-pilot#P-p4-2-run-1447385e"
discovered_from = "agent:hermes-pilot (round v7, phase 4.2 run 1447385e, the lead's ask)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-learn/src/error_enrichment.rs", "crates/roko-cli/src/graph_execution/plan_handback.rs"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90", "bug-fc05a3", "gap-54089d", "bug-1e272c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn retry_feedback_names_every_failing_doctest_case' crates/roko-cli/src && cargo test -p roko-cli --lib -- retry_feedback_names_every_failing_doctest_case"

[[verify]]
command = "grep -rqw 'fn fallback_diagnosis_repeats_no_fragment' crates/roko-learn/src && cargo test -p roko-learn --lib -- fallback_diagnosis_repeats_no_fragment"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T14:51:57Z"
commit = "189d23bf2"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T12:21:23Z"
forced = false
evidence = "189d23bf2: roko_learn::failure_text reads a failed check's output (failure, progress and summary lines; failing cases by doctest's TEST CASE:, Catch2's rules, GoogleTest's [ RUN ], cargo's ---- stdout ---- and pytest's ____; each failure with the lines under it), and failure_excerpt keeps each case's header and first failure with why, an also: line per further failure, then the summary, within 2,400 chars. step_failure uses it instead of the last 30 lines (kept for output with no failure line); GateFeedback::from_raw lists TEST CASE:, C++ failed assertions and GoogleTest's [  FAILED  ] Suite.Test once each and never a summary line; the rendered raw excerpt is 4 KB; key_line takes the first C++ failed assertion; fallback_diagnosis names each failing case once with file:line, assertion and count; the hand-back excerpt shares the reader. Verify: retry_feedback_names_every_failing_doctest_case (phase 4.2's best-tree output: P4.2.01 with lfo2_phase and P4.2.04 with FINE, SUB, SUB OCT and TRACK; 4 failing tests, no summary line) and fallback_diagnosis_repeats_no_fragment pass; roko-learn failure_text + error_enrichment 9/9; roko-cli graph_task_dispatch:: + plan_handback + dispatch::prompt_builder 555/555; nightly fmt and clippy -p roko-cli -p roko-learn -D warnings clean."
+++

## Problem

A task's retry learns why its last try failed from `GateFeedback`
(`crates/roko-cli/src/dispatch/prompt_builder.rs`). `render_gate_feedback_sections` renders it as three parts: a
diagnosis, the failing tests parsed from the output, and a raw excerpt. When a C++ project's checks are doctest
binaries, all three parts miss part of the failure.

**The case:** round v7, Undertow phase 4.2, run `1447385e`. The plan is `p4-2-the-editor-laid-out-as-designed`, on
roko d286361c4 with the OpenAI ladder. Task T1 (the editor layout), try 1, ran on gpt-5.4.

**The failure.** T1#1's first verify step, `tools/spec.sh 4_2 layout`, failed 2 of its 5 doctest cases:
- **P4.2.01**, "every control sits in its own panel, clear of the CHEST, the pump field, the GUT and each other".
  - It fails at `layout_test.cpp:109`: `CHECK( !b.intersects(otherBounds) )`.
  - It logs: "`lfo2_phase` (MOD/05) its bounds 992 468 32 32 overlap `lfo2_shape`'s 988 444 40 40".
- **P4.2.04**, "every control has the design's short label, drawn under it".
  - It fails at `layout_test.cpp:219`: `CHECK( found != nullptr )`, with values `CHECK( nullptr != nullptr )`.
  - It fails for 4 controls: osc_fine "FINE", sub_level "SUB", sub_octave "SUB OCT" and filter_track "TRACK".
  - Each failure logs where the label must be: "centred on it within 10 px, starting at most 24 px below it, inside
    OSC/01 16 56 280 160".
- **Which output the retry was cut from:** the re-run on the best checked tree (gap-56f761). There, 5 of 2777
  assertions failed, in 59 lines of output.
  - On the try's final tree the same 2 cases failed, with 58 of 2724 assertions (57 labels), in about 311 lines.

**What the retry was told:**
1. **The diagnosis was garbled.** The cheap model (glm-4-7, whose provider was out of balance) failed Exhausted, so
   `fallback_diagnosis` (`crates/roko-learn/src/error_enrichment.rs`) wrote it: "The primary error is [doctest] test
   cases: 5 | 3 passed | 2 failed | 0 skipped. The retry should fix found != nullptr ) is NOT correct found != nullptr
   ) is NOT correct."
   - `extract_mismatch` looks for Rust's type-mismatch words ("expected ", "found "). The test's variable `found`
     matched them, and `take(2)` repeated the fragment.
   - `extract_file_line` reads only `.rs:` locations, so the C++ file and line never appear.
   - The diagnosis does not mention the overlap.
2. **"## Failing tests" held only doctest's two summary lines.**
   - `from_raw` counts a line as a failing test only when it holds "test" and "failed" or "panicked". doctest's
     `file.cpp:219: ERROR: CHECK( … ) is NOT correct!` and `TEST CASE:` lines hold neither.
   - Catch2's `FAILED:` and gtest's `[  FAILED  ]` lines match only when a path or test name happens to contain
     "test". Their assertion and expansion lines never match.
3. **The raw excerpt kept only one case whole.**
   - Since bug-59ed90 the excerpt is rendered unless compile errors were parsed. The earlier finding that the retry
     drops the raw text is out of date, and the round's memory note has been corrected.
   - The excerpt is `step_failure`'s cut (`crates/roko-cli/src/graph_task_dispatch/verification.rs`): the step's label
     line and the last 30 lines of its output.
   - Of the 59 lines, the last 30 hold P4.2.04's header and its 4 label failures with their expected positions.
   - Of P4.2.01 they hold only the dangling "its bounds 992 468 32 32 overlap `lfo2_shape`'s 988 444 40 40". The
     control it names (`lfo2_phase`) is 1 line above the cut, the check 3 lines, and the case name 5 lines.
   - Had the final tree's output been cut instead, the last 30 lines would hold 5 of its 57 label failures (ROOT,
     SCALE, MUTATE, EVOLVE, SEED) and nothing of P4.2.01.

**So the retry named one failing case in full and the other by half a line.** The structured parts named neither.

T1#2 (gpt-5.4, 59 turns, $1.03) passed anyway. Before running anything, it moved all 5 controls, working out
`lfo2_phase` from the coordinates and Theme.h's cell table. A weaker model, or a failure that carries no coordinates,
would not get there.

**Evidence:** `tmp/hermes-roko-v7/pilot/evidence/p4-2-t1-1/`, taken from Undertow's `.roko/events.jsonl`.
- `gate-result-1.txt`: the final tree's result, its last lines.
- `gate-result-2.txt`: the best-tree re-run, whole. This is the text the retry was cut from.
- `gate-output-lines.txt`: the streamed lines.
- `retry-feedback.json`: the `retry_feedback` event, with `failing_steps`, `diagnosis` and `errors`.

The prompt itself isn't kept: the verdict holds only `request_sha256`. The cut above is reconstructed from
d286361c4's `step_failure` and `render_gate_feedback_sections`. The full account is in
`tmp/hermes-roko-v7/findings/hermes-pilot.md` §P, "T1#1's failure in detail".

## Why it matters

Every phase left in round v7 (4.3 and 5) is C++ checked by doctest binaries. The 30-line tail favours whichever case
doctest prints last. A retry told about one of two failing cases fixes one and fails again on the other, and each
round trip costs a strong-rung try (about $1 here).

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: `step_failure` (the label and the last 30 lines) and the
  `raw_for_feedback` join.
- `crates/roko-cli/src/dispatch/prompt_builder.rs`: `GateFeedback::from_raw` (the line classes) and
  `render_gate_feedback_sections` (the first 2 KB of the raw output).
- `crates/roko-learn/src/error_enrichment.rs`: `fallback_diagnosis`, `extract_mismatch` and `extract_file_line`.
- The failure-first excerpt to reuse, from bug-fc05a3: `excerpt` and `is_failure_line` in
  `crates/roko-cli/src/graph_execution/plan_handback.rs`.

## Plan

- **Replace `step_failure`'s blind tail with a failure-first excerpt,** as bug-fc05a3's `excerpt` does for a
  hand-back.
  - Drop build progress lines.
  - Keep each failing case's header (doctest's `TEST CASE:`, Catch2's test name, gtest's `[ RUN      ]`).
  - Keep its first failing assertion, with that assertion's `values:` and `logged:` lines.
  - Then the summary, within the budget.
- **Teach `from_raw` the C++ failure lines,** or share `is_failure_line` with it: doctest's `ERROR: CHECK(…) is NOT
  correct!`, Catch2's `FAILED:`, and gtest's `Failure` and `[  FAILED  ]`. Then "## Failing tests" names the cases, not
  the summary lines.
- **Fix `fallback_diagnosis`:** read C++ (and other) `file:line` locations, drop repeated fragments, and lead with the
  first failing assertion rather than a summary line.

## Design

roko-exact, 2026-10-26, with the lead's call: failure first, as bug-fc05a3's hand-back excerpt.

- **One reader, in roko-learn.** A new `roko_learn::failure_text` module holds it, so roko-learn's fallback and
  roko-cli share it.
  - `is_failure_line` adds doctest's `ERROR:`, Catch2's `FAILED:` and GoogleTest's `Failure`.
  - `is_progress_line` adds CMake, make and ninja progress to cargo's.
  - `is_summary_line` covers doctest's, Catch2's, cargo's, CTest's, GoogleTest's and pytest's count lines.
  - `failed_cases` reads each failing case, with its header and its failures. Each failure keeps the lines under it:
    `values:`, `logged:`, Catch2's expansion, GoogleTest's `Which is:`. The cases come from doctest's `TEST CASE:`,
    Catch2's name between dashed rules, GoogleTest's `[ RUN      ]`, cargo's `---- name stdout ----` and pytest's
    `____ name ____`.
- **The step's entry (`step_failure`).**
  - `failure_excerpt` cuts it to 2,400 chars. It starts with the failure lines printed before the first case
    (CTest's `***Failed`), then gives each failing case its header and its first failure, with the lines under it.
  - As far as the budget goes, each further failure gets one `also:` line, using what tells it apart (its own
    assertion, or what it logged). Then come at most 3 summary lines.
  - A further failure never crowds out another case.
  - Output with no failure line keeps its last 30 lines, as before.
- **`from_raw`.** "## Failing tests" takes a doctest `TEST CASE:`, a C++ failed assertion and GoogleTest's
  `[  FAILED  ] Suite.Test`, each once, and never a summary line. The raw excerpt rendered goes from 2 KB to 4 KB,
  so one step's entry shows whole.
- **`key_line`.** It takes a C++ runner's first failed assertion before an assertion scan, and never a summary line.
  It used to give `[doctest] assertions: …`.
- **`fallback_diagnosis`.**
  - With failing cases, it names each case once: its name, `file:line` and assertion, and how many failures it had.
    For phase 4.2: "2 test cases failed: P4.2.01 … at layout_test.cpp:109: CHECK( !b.intersects(otherBounds) ); P4.2.04
    … at layout_test.cpp:219: CHECK( found != nullptr ), 4 failures."
  - Otherwise it reads any `file.ext:line` and a C++ `: error:` headline, skips summary lines, and says a mismatch
    fragment once.
- **The hand-back excerpt** (`plan_handback::excerpt`) uses the same reader.

## Done when

The tests in `[[verify]]` pass:
- Given phase 4.2 T1#1's best-tree output (`gate-result-2.txt`), the retry's rendered feedback names both failing
  cases: P4.2.01 with `lfo2_phase`, and P4.2.04 with its 4 labels.
- The fallback diagnosis repeats no fragment.
