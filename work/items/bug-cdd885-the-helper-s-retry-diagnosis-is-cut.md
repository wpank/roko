+++
id = "bug-cdd885"
kind = "bug"
title = "The helper's retry diagnosis is cut at 320 characters mid-sentence: all four of phase 5's model-written diagnoses end mid-word (\"(and also `s\"), dropping the second sentence that lists what else fails; the prompt also asks for a \"type mismatch\" on test failures"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-learn/error_enrichment"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 run-analyst and the pilot (tmp/hermes-roko-v7/findings/run-analyst.md, section 10; Undertow run 1f677933), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-learn/src/error_enrichment.rs::normalize_diagnosis", "crates/roko-learn/src/error_enrichment.rs::enrich_error_digest"]
links = { depends_on = [], blocks = [], related = ["bug-3e3d08", "bug-e33b7e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_long_diagnosis_ends_on_a_whole_sentence' crates/roko-learn/src && cargo test -p roko-learn a_long_diagnosis_ends_on_a_whole_sentence"
+++

## Problem

The helper's retry diagnosis is cut at 320 characters, mid-sentence and often mid-word.

**Every model-written diagnosis in phase 5's run was cut.** In Undertow run 1f677933 all four are exactly 320
characters (the run's event index, `retry_feedback.diagnosis`):
- T1#1's ends "… the type/magnitude mismatch is spectral content: this preset is";
- T1#2's ends "… This is a preset-level gain mismatch, not a code-logic type error";
- T4#1's ends "… and also `sub-pure-foundation.wav` and `rumbl";
- T4#2's ends "… for `artifacts/audio/init.wav` (and also `s".

The second sentence is where the diagnosis lists what else fails, and the cut drops it. Phase 4.3's diagnoses were
191 and 278 characters, and stood whole.

**The prompt is compiler-centric.** It asks the helper to "Diagnose this compilation/test error in 2 sentences. Be
specific about which file, line, and type mismatch." For a spectral test failure that yields "the type/magnitude
mismatch is spectral content" and "not a code-logic type error".

## Why it matters

- The diagnosis heads the retry's feedback ("Fix these exact errors before doing anything else"). A cut list of
  failing files tells the retry less than the raw output below it.
- The raw output is now short and failure-first (bug-3e3d08), so the diagnosis should add to it, not trail off.
- Goal: golden-path.

## Where

- `crates/roko-learn/src/error_enrichment.rs::normalize_diagnosis`: keeps `first_sentences(…, 2)` and then
  `truncate_chars(…, DIAGNOSIS_CHAR_LIMIT)`, where `DIAGNOSIS_CHAR_LIMIT` is 320. The cut is a plain character
  cut.
- `crates/roko-learn/src/error_enrichment.rs::enrich_error_digest`: the prompt.

## Current state

The fallback diagnosis was reworked by bug-3e3d08; this is the model path.

## Plan

1. When two sentences run past the limit, keep whole sentences:
   - the first alone when it fits;
   - else cut at a word boundary and end with "…".
   - Or raise the limit to about 600: the feedback section has room, now that the raw output is short.
2. Make the prompt name test failures too, e.g. "which file, line and failed check, and what it measured against
   what it expected", not only "type mismatch".
3. Add the test `a_long_diagnosis_ends_on_a_whole_sentence`.

### Design (roko-learnguard; the lead approved it on 2026-10-26)

- **The limit:** `DIAGNOSIS_CHAR_LIMIT` goes from 320 to 600. Phase 5's two-sentence diagnoses run about 350 to 450
  characters, so they now stand whole.
- **The cut:** `normalize_diagnosis` keeps as many whole sentences as fit, up to two.
  - When even the first sentence is longer than the limit, it is cut at its last word boundary and ends with "…".
  - So a diagnosis ends on a sentence or on a marked cut, never mid-word.
- **The prompt:** it moves into `diagnosis_prompt` and asks which file and line, or which test or check, failed, what
  it found against what it expected, and what else fails. The compiler-only "type mismatch" goes.
- **Tests:**
  - `a_long_diagnosis_ends_on_a_whole_sentence`: phase 5's T4#2 shape stands whole; with two long sentences only
    the first is kept; one over-long sentence ends with "…" on a word boundary;
  - `the_diagnosis_prompt_asks_what_failed_against_what_it_expected`.

## Done when

- The `[[verify]]` passes.
- The next run's diagnoses end on a sentence, or on a marked cut.

## Notes

- Evidence: `tmp/hermes-roko-v7/findings/run-analyst.md`, section 10, and the pilot's phase 5 notes
  (`DIAGNOSIS_CHAR_LIMIT`).
- Related: bug-3e3d08 (done: the raw output and the fallback), bug-e33b7e (which helper writes it).
