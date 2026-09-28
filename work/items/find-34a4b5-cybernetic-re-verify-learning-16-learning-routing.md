+++
id = "find-34a4b5"
kind = "finding"
title = "[cybernetic re-verify: learning] 16 learning/routing closures wired into deleted Runner-v2 event_loop.rs"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-learn"]
created = 2026-09-06
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
anchors = ["crates/roko-learn/src/hindsight.rs::HindsightRelabeler", "crates/roko-compose/src/attention.rs::ModelAttentionCurves", "crates/roko-learn/src/active_inference.rs::BeliefState", "crates/roko-learn/src/aggregate.rs", "crates/roko-cli/src/knowledge_helpers.rs:481", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/dispatch/factory.rs"]
links = { depends_on = [], blocks = [], related = ["gap-5fb9a7", "reg-ff6e1a", "reg-c7ecf6", "q-1faa0c", "find-4b4344"], supersedes = [], duplicate_of = "" }
+++
Checked done 2026-09-06 with Files in runner/event_loop.rs, same period Runner-v2 was deleted: P0-01,P0-02,P0-04,P0-07,P0-09,P0-13,P1-09,P1-19,P1-20,P1-22,P2-15,P3-17,P3-32,P4-04,P4-17,P4-19. Wiring may not have survived Graph cutover.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code`

A source claims this was fixed; confirm against current code before closing.

Some cited files are gone: `section_bandit.rs/section_effect.rs`.

How to verify: For each ID, grep the named symbol for call sites under graph_execution/ or roko-graph (not runner/); check .roko/learn artifacts update after a Graph plan run.

Verified 2026-09-28: symbol scan of non-test code. Survived on live paths: section_effect (dispatch/prompt_builder.rs, roko-learn feedback_service.rs), FormatBandit (dispatch/factory.rs), active-inference BeliefState (roko-learn cascade_router.rs), apply_neuro_hints (knowledge_helpers.rs:481/544). Lost or unwired: HindsightRelabeler (only hindsight.rs; see gap-5fb9a7), ModelAttentionCurves (only roko-compose attention.rs), no efe-belief.json persistence anywhere, score_prompt_efficiency and CompoundingMetric no longer exist, and playbook_hit_rate is only computed in roko-learn aggregate.rs. Each lost closure needs its own item or a re-wire.
