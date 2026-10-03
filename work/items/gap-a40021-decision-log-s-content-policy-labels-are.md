+++
id = "gap-a40021"
kind = "gap"
title = "decision_log's content_policy labels are stale, error patterns collapse to one block item, and task_query_text reaches only load_group_context"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/decision-log", "roko-cli/prompt-builder"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK33 gap-aea13a)"
discovered_from = "gap-aea13a"
anchors = ["crates/roko-cli/src/graph_task_dispatch/decision_log.rs::content_policy", "crates/roko-cli/src/dispatch/factory.rs::ErrorPatternSelection", "crates/roko-cli/src/dispatch/prompt_builder.rs::load_group_context"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn error_pattern_content_decision_lists_one_candidate_per_key' crates/roko-cli/ && cargo test -p roko-cli error_pattern_content_decision_lists_one_candidate_per_key"
+++

## Problem

`crates/roko-cli/src/graph_task_dispatch/decision_log.rs` records three things about a prompt's content decisions
that are each less accurate than they look:

1. **Stale `content_policy` labels.** `content_policy()` (line 160) labels the `Knowledge` decision point
   `"keyword_overlap_top3"` and `ErrorPatterns` `"error_pattern_summary_top5"`. Both names describe a *generic*
   top-N-by-keyword/overview ranking. But PK33's own package (`gap-aea13a`, done) changed both mechanisms in the
   same backlog wave: task 4211 ("Rank knowledge by topic words, not task ids, roles and path words, and keep
   success notes out") replaced raw keyword overlap with a topic-word match for knowledge, and tasks 4209/4210
   ("Error-pattern store selects by task and command" / "Inject only the error patterns keyed to a task") replaced
   "the store's five leading patterns" with per-task/per-command keying. The labels were never updated to match,
   so a reader of `decisions.jsonl`/`content_decisions.jsonl` sees a policy name that no longer describes what
   actually selected the candidates.
2. **Error-pattern exposure collapses to one block item.** `ErrorPatternSelection` (`crates/roko-cli/src/dispatch/factory.rs:41-46`)
   already carries `keys: Vec<String>` — "the keys of the selected patterns, in display order" — alongside
   `text: String`, "the rendered block." But `prompt_builder.rs` creates exactly one `PromptItemDiagnostic` with
   `kind: ExposureItemKind::ErrorPattern` per prompt (the only occurrence of that kind in the whole file, line
   1263), regardless of how many keys `ErrorPatternSelection.keys` holds. `record_content_decisions`
   (`decision_log.rs:104`) groups by `item.kind.decision_point()`, so the `ErrorPatterns` decision point gets at
   most one candidate — the whole rendered block — never one candidate per actual pattern key. Multiple
   error patterns selected into the same block are indistinguishable in the content-decision record: there is no
   way to tell which of the `keys` were "chosen" versus merely rendered alongside them.
3. **`task_query_text` only reaches `load_group_context`.** `crates/roko-cli/src/dispatch/prompt_builder.rs` defines
   its own `task_query_text(task, ctx)` (line 2601) — distinct from `roko-neuro`'s unrelated same-named function in
   `context.rs:1111` — and calls it exactly once, at line 2735, inside `load_group_context` (defined at line 2673)
   to build `query`/`keywords` for scoring knowledge entries by topic overlap. No other function in
   `prompt_builder.rs` calls this version, so whatever richer task-query text 4211's topic-word work computes here
   is invisible to every other content decision point (playbooks, sections, error patterns) that could use the same
   topic signal instead of recomputing or approximating their own.

## Why it matters

Goal: learning (decision-log accuracy). S01 §4.5's content decision rows exist so a later audit or learning signal
can tell *why* a prompt's content was chosen. A stale policy label (1) misleads anyone reading the log about what
ranking actually ran. A collapsed error-pattern block (2) makes it impossible to credit or demote individual error
patterns from their actual selection outcome — exactly the kind of per-key feedback S01's content-decision design
intends for knowledge and playbooks, which error patterns lack. The unshared `task_query_text` (3) means the
topic-word signal 4211 just built for knowledge isn't available to the other decision points PK33 also touched,
so they may still rank by a weaker or inconsistent signal even after the same backlog wave improved knowledge's.

## Where

- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::content_policy` (line 160) and `content_decision` (line 178).
- `crates/roko-cli/src/dispatch/factory.rs::ErrorPatternSelection` (lines 41-46).
- `crates/roko-cli/src/dispatch/prompt_builder.rs`: the single `ExposureItemKind::ErrorPattern` construction (line
  1263), `task_query_text` (line 2601) and `load_group_context` (line 2673, calls it at line 2735).

## Current state

All three are present at HEAD as described. `gap-aea13a` (PK33, done, `5bb643122`) changed the knowledge and
error-pattern selection mechanisms themselves but did not touch `decision_log.rs`'s labels or the exposure-item
granularity for error patterns, and did not thread `prompt_builder.rs`'s `task_query_text`/topic-word helpers to
other decision points.

## Plan

1. Update `content_policy()`'s two stale arms to name the actual current mechanism (e.g. `"topic_word_top3"` for
   Knowledge, matching 4211's wording, and a name reflecting per-task/command keying for ErrorPatterns, matching
   4209/4210) — check both tasks' files for the exact terms they use, so the label and the real mechanism agree.
2. Give error patterns one `PromptItemDiagnostic`/`ExposureItemKind::ErrorPattern` per key in
   `ErrorPatternSelection.keys`, each carrying that key's id, so `record_content_decisions` records one candidate
   per pattern (matching how Knowledge and Playbooks already work) instead of one opaque block.
3. Either move `prompt_builder.rs`'s `task_query_text`/topic-word helpers to a shared location all decision points
   can call, or confirm (and document) that the other decision points intentionally use a different signal.

## Done when

- `content_policy()`'s labels name what each decision point's ranking actually does today.
- A content-decision row for `ErrorPatterns` lists one candidate per selected pattern key, not one block.
- The `[[verify]]` command passes.

## Notes

- Filed as one item: all three observations are about the same module's recorded-content accuracy, discovered
  together while re-reading PK33's own change against `decision_log.rs`.
- `roko-neuro::context.rs`'s own `task_query_text` (line 1111) is unrelated — a different function, same name, in a
  different crate; do not confuse the two when fixing (3).
