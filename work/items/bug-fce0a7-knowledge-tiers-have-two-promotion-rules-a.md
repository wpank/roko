+++
id = "bug-fce0a7"
kind = "bug"
title = "Knowledge tiers have two promotion rules: a pass that surfaces an entry re-tiers it through evaluate_tier_progression_v2 (transient to working at 1 confirmation, working to consolidated at 2), a repeat through the memory spec's 2 confirmations and 3 contexts, so every runtime lesson is Working at admission on its own pass (all 46 admissions in Undertow's store)"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-neuro/tier-progression", "roko-neuro/knowledge"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 roko-learnguard (gap-1220c5's review; Undertow .roko/neuro copy of 2026-10-27 01:24), the lead's ask"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-neuro/src/tier_progression.rs::evaluate_tier_progression_v2", "crates/roko-neuro/src/tier_progression.rs::promotion_threshold", "crates/roko-neuro/src/knowledge_store/pending.rs::confirm_by_repeat", "crates/roko-neuro/src/knowledge_store/pending.rs::note_surfacing", "crates/roko-neuro/src/lifecycle.rs::build_runtime_entry", "crates/roko-core/src/config/learning.rs::KnowledgeProgressionConfig"]
links = { depends_on = [], blocks = [], related = ["gap-1220c5", "reg-06ae9f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn both_confirmation_paths_promote_by_one_rule' crates/roko-neuro/src && cargo test -p roko-neuro both_confirmation_paths_promote_by_one_rule"
+++

## Problem

Knowledge tiers move by two rules that disagree, and the looser one promotes every runtime lesson on its own pass.

- **A surfacing pass** re-tiers through `TierProgression::evaluate_tier_progression_v2`. This happens in
  `lifecycle.rs::promote_runtime_entries`, through `pending.rs::confirm_by_surfacing` and `note_surfacing`. Its
  `promotion_threshold` compares the entry's cumulative `confirmation_count` with these values:
  - Transient: 1;
  - Working: 2;
  - Consolidated: 5.
  It moves an entry one tier per evaluation, and it runs on every pass that surfaces the entry, whether or not the
  pass counted.
- **A repeat** promotes inside `pending.rs::confirm_by_repeat`, from `crud.rs::ingest` and held repeats. Its rule
  matches the memory spec: 2 confirmations for Transient to Working, and 3 distinct contexts for Working to
  Consolidated.
- **The spec** says Transient to Working takes 2+ confirmations with confidence >= 0.5, and Working to Consolidated
  takes 3+ distinct contexts with confidence >= 0.6. Sources:
  - `docs/v3/09-MEMORY.md`, the tier diagram;
  - `docs/v3/depth/09-memory/four-validation-tiers.md`;
  - `docs/v2/06-MEMORY.md`, the promotion table.
  `[learning.knowledge]`'s `transient_confirmations = 2` and `working_contexts = 3` (`KnowledgeProgressionConfig`)
  say the same. `docs/v3/depth/21-config/01-schema-sections.md` documents them as "No effect", and nothing reads them.
- **Neither rule checks the spec's confidence floors.**
- **So one pass makes every runtime lesson Working.** `lifecycle.rs::build_runtime_entry` creates a lesson Transient,
  with `confirmation_count = 1` for its own gate pass. In the same observation, `promote_runtime_entries` re-tiers it
  through v2. The pass doesn't count again, since the lesson already holds its context, but 1 meets Transient's
  threshold, so the lesson is Working at admission. One more surfacing pass makes it Consolidated, at a count of 2 with
  2 contexts. The spec wants a second confirmation for Working and a third context for Consolidated.
- **This undercuts gap-1220c5.** gap-1220c5 made each pass count once, but the surfacing rule promotes on a count
  of 1.

**On a copy of Undertow's store** (taken 2026-10-27 01:24; 57 entries, 52 lifecycle receipts):
- **All 46 lifecycle admissions** (`light_admitted`) re-tiered their lesson at once: every admission receipt has
  `promotion_updates` >= 1.
- **45 of those lessons are in the store.**
  - One is still Working with a count of 1 and its own context only: phase 5.2's `runtime_4e1e06356ebe55ea`,
    created 2026-10-26T22:01:11Z. Under the spec it would be Transient.
  - The other 44 are Consolidated. Each has 3+ contexts, reached through the confirmations gap-1220c5 has since
    stopped counting. 27 of them sit below the spec's 0.6 confidence floor.

**No recorded decision covers the thresholds.**
- `work/DECISIONS.md` names none.
- "P2-15", which the repeat path's comment cites, is the cybernetic programme's "Tier progression events" item:
  logging tier moves, not a rule (`work/done/find-34a4b5-…`). The comment misattributes the rule.
- reg-06ae9f wired the surfacing path for Graph runs (`VerifiedKnowledgeSink`). It restored what the deleted
  runner's `NeuroKnowledgeIngestor` did, which ran `evaluate_tier_progression_v2`, and chose no thresholds.
- `evaluate_tier_progression_v2` and `promotion_threshold` date from the 2026-04-26 workspace checkpoint, with no
  spec reference.

## Why it matters

- **Tier scales an entry's half-life** (`KnowledgeTier::multiplier`): 0.1x for Transient, 0.5x for Working, 1.0x for
  Consolidated. Freshness, and with it retrieval, decays by that half-life. So a lesson one pass old keeps five times
  the half-life the spec would give it, with the evidence the spec asks of a Transient entry.
- Two rules for one count make tier changes depend on which path ran last.
- Goal: learning.

## Where

- `crates/roko-neuro/src/tier_progression.rs::evaluate_tier_progression_v2`, `::promotion_threshold`.
- `crates/roko-neuro/src/knowledge_store/pending.rs::confirm_by_surfacing`, `::note_surfacing`, `::confirm_by_repeat`.
- `crates/roko-neuro/src/lifecycle.rs::build_runtime_entry`, `::promote_runtime_entries`.
- `crates/roko-core/src/config/learning.rs::KnowledgeProgressionConfig` (unread).
- `docs/v3/09-MEMORY.md` (the tier diagram), `docs/v3/depth/09-memory/four-validation-tiers.md`.

## Plan

**A design call comes first:** which rule wins.
1. **The spec's rule everywhere.** Both paths promote by confirmations and contexts, 2 and 3. Read them from
   `[learning.knowledge]`, so the documented knobs take effect, and decide whether the confidence floors apply. A
   runtime lesson then stays Transient until a second pass confirms it.
2. **v2's rule everywhere.** The repeat path uses `promotion_threshold` too, and the spec, the config docs and the
   `KnowledgeProgressionConfig` fields change to match.

Then:
- one function decides a tier move from an entry's confirmations and contexts, and both paths call it;
- the misattributed "(P2-15)" comment in `confirm_by_repeat` goes;
- a test proves that both paths promote the same entry the same way.

## Done when

- The `[[verify]]` passes: a surfacing pass and a repeat with the same counts leave an entry on the same tier.
- The rule that wins is the one the spec documents.

## Notes

- Found by roko-learnguard in gap-1220c5's review (round v7): a new test assumed one pass couldn't make a transient
  entry Working, and it could. Filed at the lead's ask, without code.
- Related: gap-1220c5 (what counts as a confirmation), reg-06ae9f (the Graph path's surfacing confirmations).
