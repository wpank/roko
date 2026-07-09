# B — Knowledge Tiers (Docs 01, 02)

Parity analysis of `docs/05-learning/01-playbook-system.md` and
`docs/05-learning/02-skill-library-voyager.md` vs the actual codebase.

---

## B.01 — `Playbook` and `PlaybookStep` structs

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §PlaybookStore schema — `Playbook { id, name, goal, steps, success_count, failure_count }` and `PlaybookStep { index, description, action_kind, expected_signals }`.
**Reality**: `crates/roko-learn/src/playbook.rs:44-53` defines `PlaybookStep` with the four fields verbatim. `crates/roko-learn/src/playbook.rs:77-98` defines `Playbook` with all four doc-claimed fields plus two additional ones the doc omits: `created_at_ms: i64` and `last_used_ms: Option<i64>`. Counter types are `u64`, not `u32` as the doc claims.
**Notes**: Minor drift — doc undercounts fields (6 vs 8) and mistypes counters (u32 vs u64). Substance matches.

---

## B.02 — `PlaybookStore` load/save/record_outcome operations

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §Operations — `save`, `load`, `load_all`, `record_outcome` on `PlaybookStore`.
**Reality**: `crates/roko-learn/src/playbook.rs:147-151` defines `PlaybookStore { root, tmp_counter, id_locks }`. All three operations present:
- `save()` at `playbook.rs:196` (atomic tempfile + rename)
- `load()` at `playbook.rs:235`
- `record_outcome()` at `playbook.rs:369`
- `list()` at `playbook.rs:329` (serves the `load_all` role)
Plus `delete()` at `playbook.rs:400` and `record()` at `playbook.rs:390` that the doc does not mention.
**Notes**: Doc names the list method `load_all`; code calls it `list`. Per-id async mutexes (`tokio::sync::Mutex`) confirmed at `playbook.rs:150` as described.

---

## B.03 — "0 tests for `playbook.rs`" concern

**Status**: DONE
**Severity**: —
**Doc claim**: (Task brief flagged `playbook.rs` as "0 tests — concerning".)
**Reality**: `playbook.rs` contains **17 `#[tokio::test]`** tests covering roundtrip save/load, concurrent updates, path-traversal rejection, empty-id rejection, success-rate math, relevance ranking, and delete. No `#[test]` (sync) tests, which is why a pattern limited to `#[test]` returned zero — but `#[tokio::test]` is the async counterpart and is present. Line citations: `playbook.rs:524`, `:552`, `:579`, `:634`, `:660`, `:736`, `:759`, `:770`, `:793`, among others.
**Notes**: The "concerning" framing in the brief was based on an incomplete grep; the crate is well-tested at the module level.

---

## B.04 — `Rule`, `Triggers`, `MatchContext` structs with globset matching

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §Rule Schema — `Rule` with `rule_id`, `title`, `body`, `triggers`, `confidence`, `validations`, `contradictions`, `last_applied`, `created_at`, `source_episodes`. `Triggers` with five lists (file_globs, tags, categories, error_signatures, roles). `MatchContext` with files/tags/category/error_signature/role.
**Reality**:
- `Triggers` at `crates/roko-learn/src/playbook_rules.rs:35-46` — all five fields present.
- `Rule` at `playbook_rules.rs:66-89` — all ten doc-claimed fields present with matching types.
- `MatchContext` at `playbook_rules.rs:116-127` — five doc-claimed fields present, but `role: String` is non-optional in code (doc shows `Option<String>`).
- `PlaybookRules` store at `playbook_rules.rs:173-176`.

Globset integration confirmed: `use globset::Glob` at `playbook_rules.rs:20`; runtime match at `playbook_rules.rs:598` (`Glob::new(glob_pat).compile_matcher()`).
**Notes**: Doc's `role: Option<String>` vs code's `role: String` is a minor contract drift but not semantic — empty string treated as unset.

---

## B.05 — Confidence dynamics: validate `+0.05`, contradict `−0.10`, ceiling `0.95`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §Confidence Dynamics — validation adds 0.05 capped at 0.95; contradiction subtracts 0.10 floored at 0.0. Asymmetric update rate (contradictions 2× validations).
**Reality**: `playbook_rules.rs:341-352` implements `record_outcome(rule_id, validated)`:
```rust
if validated {
    rule.confidence = (rule.confidence + 0.05).min(0.95);
    rule.validations = rule.validations.saturating_add(1);
} else {
    rule.confidence = (rule.confidence - 0.10).max(0.0);
    rule.contradictions = rule.contradictions.saturating_add(1);
}
```
Module header at `playbook_rules.rs:8-10` documents the constants. Test coverage at `playbook_rules.rs:1072-1087` (ceiling) and `:1094-1129` (floor at 0.0).

---

## B.06 — Confidence floor and minimum-confidence prune threshold

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 table row "Prune threshold" — rule removed if `confidence < min_confidence`. Doc 01 §Demotion — "configurable, default 0.10". Also §Confidence Dynamics table — "Contradiction … Floored at 0.0".
**Reality**: Contradiction floor is **0.0**, not 0.10 (see `playbook_rules.rs:348`). Prune method exists at `playbook_rules.rs:357-362`:
```rust
pub fn prune(&self, min_confidence: f64) -> usize {
    let mut guard = self.rules.write();
    let before = guard.len();
    guard.retain(|r| r.confidence >= min_confidence);
    before - guard.len()
}
```
`min_confidence` is supplied by the caller, not stored as a config default. No 0.10 constant found in `playbook_rules.rs`. The doc conflates "confidence floor during contradiction decay" (0.0) with "default prune threshold" (documented as 0.10 but not encoded anywhere in the crate).
**Fix sketch**: Add an `min_confidence: f64` field to `PlaybookRules` with a `0.10` default, or drop the "default 0.10" from the doc and note that prune threshold is caller-supplied.

---

## B.07 — TOML persistence at `.roko/learn/playbook-rules.toml`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 header — `Persistence: .roko/learn/playbook-rules.toml`. §Persistence Format — rules serialized as `[[rule]]` array-of-tables in TOML.
**Reality**: Path resolved in `crates/roko-learn/src/runtime_feedback.rs:127` — `playbook_rules_toml: root.join("playbook-rules.toml")`. TOML envelope at `playbook_rules.rs:160-164`:
```rust
#[derive(Serialize, Deserialize, Default)]
struct PlaybookRulesFile {
    #[serde(default, rename = "rule")]
    rules: Vec<Rule>,
}
```
Load path at `playbook_rules.rs:187-200` uses `toml::from_str`. Atomic save via tempfile + rename is implemented and covered by the roundtrip test at `playbook_rules.rs:806-823`.

---

## B.08 — `MatchContext` rule matching wired into orchestrate.rs

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 01 §Integration with Prompt Composition — `PlaybookRules::select(MatchContext)` receives files/tags/category/error_signature/role from the current task; top-N (3-5) rules injected into the system prompt.
**Reality**: Call site at `crates/roko-cli/src/orchestrate.rs:7096-7117` inside `build_learned_context`:
```rust
let match_ctx = MatchContext {
    files: Vec::new(),
    tags: Vec::new(),
    category: None,
    error_signature: None,
    role: role_tag.clone(),
};
let rules = self.learning.playbook_rules().select(&match_ctx, 5);
```
Only `role` is populated — `files`, `tags`, `category`, `error_signature` are hard-coded to empty. The OR-semantics matcher at `playbook_rules.rs:595-644` therefore only ever fires on role triggers. Rules whose triggers depend on file globs, tags, categories, or error signatures can never match in production today.
**Fix sketch**: Thread `task_def.files`, `task_def.tags`, `task_def.category`, and the last gate-failure signature (already tracked at `orchestrate.rs:6706-6716`) through `build_learned_context` into `MatchContext`.

---

## B.09 — `Skill` struct fields

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §Skill Schema — 17 fields across core identity, usage telemetry, and Voyager extraction (name, summary, prompt_template, required_tools, example_inputs/outputs, tags, success_rate, usage_count, description, plan_id, files, pattern, score, first_seen, last_matched, match_count, validated_count, task_category).
**Reality**: `crates/roko-learn/src/skill_library.rs:63-154` defines `Skill` with **28 fields** — all 17 doc-claimed fields are present, plus structured-contract fields the doc omits (`id`, `precondition`, `procedure`, `postcondition`, `confidence`, `source_episodes`, `validations`, `failures`, `task_categories` (plural), `created_at`, `last_validated_at`). Voyager fields explicitly labeled `§16.3.2-16.3.4` at `skill_library.rs:123`. `task_category` doc comment at `:151` confirms "≥70% tags + same category" dedup rule.
**Notes**: Doc's schema is out-of-date but accurate for the subset it lists. Consider updating doc 02 to document the structured-contract fields.

---

## B.10 — Skill library growth: "monotonic" claim vs `prune_stale`

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 02 §Monotonic Growth Property — "skills are added but never removed in normal operation … There is no automatic pruning based on low usage or low success rate." Only dedup and manual editing can remove skills.
**Reality**: `SkillLibrary::prune_stale(days)` at `skill_library.rs:1632-1671` removes skills whose `last_matched`/`first_seen` is older than `days` days, keeping at minimum 10 skills. The method is reachable (pub fn). No call site in `orchestrate.rs` was found during this audit, so it may be dormant, but it exists and has a test at `skill_library.rs:2398-2417`.
**Fix sketch**: Either doc 02 should acknowledge `prune_stale` as an age-based eviction mechanism that violates pure monotonicity, or the method should be removed / feature-flagged off for production. The doc's core thesis (Voyager-style accumulation) is architecturally honored but the "no automatic pruning" sentence is false.

---

## B.11 — `TemplatePatternGenerator` recipe builder

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §Template Pattern Generation — "`TemplatePatternGenerator` trait … standardized interface for generating skill templates from episode data … heuristic extraction (analyzing tool calls and file modifications) or LLM-based extraction".
**Reality**: Docs describe it as a trait; code defines **two** items:
- `PatternGenerator` trait at `skill_library.rs:600-603` — this is the actual abstraction.
- `TemplatePatternGenerator` unit struct at `skill_library.rs:607` — a concrete `PatternGenerator` impl (not a trait).

The struct's `generate()` at `skill_library.rs:610-655` builds a 4-line recipe from episode `extra` fields (`files`, `role`, `model`, `task_tags`, `verbal_reflection`). Recipe length is clamped to 750 chars via `MAX_PATTERN_CHARS` at `skill_library.rs:659`, matching doc 02's "≤750 chars" claim.
**Notes**: Doc 02 calls `TemplatePatternGenerator` a trait; it is actually a unit struct implementing the `PatternGenerator` trait. Pure naming drift, semantics intact.

---

## B.12 — Skill injection into prompts via `SystemPromptBuilder`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §Skill Retrieval and Injection — top-3 matching skills injected into the system prompt under "Recommended approach"; plus §Relationship — "Loop 5 (Skills→Prompts) describes how skills feed back into prompt composition".
**Reality**:
- `SystemPromptBuilder::with_skills(&[Skill])` at `crates/roko-compose/src/system_prompt_builder.rs:179` injects the skills layer.
- `role_prompts.rs:26` imports `Skill`; `:167-219` exposes `relevant_skills: Vec<Skill>` on `TaskContext` with a `with_relevant_skills` builder.
- Forwarding at `role_prompts.rs:282-283`: `if !self.relevant_skills.is_empty() { builder = builder.with_skills(&self.relevant_skills); }`
- Runtime query at `orchestrate.rs:7083-7094` uses `skill_library.search_by_tag(&role_tag)` and takes the top 3 into a `## Relevant Skills from Past Successes` section.

Test coverage confirms forwarding at `role_prompts.rs:593` (`relevant_skills_are_forwarded_into_the_prompt_builder`) and budgeting at `system_prompt_builder.rs:1510`.
**Notes**: Retrieval at runtime uses a simple `search_by_tag(role_tag)` path — **not** the richer `SkillQuery`-based `select()` that supports file hints and category filters (defined at `skill_library.rs:1127`). Similar to B.08, the matching surface used at runtime is narrower than what the code supports.

---

## B.13 — `ToolUsageProfile` / `ToolSequencePattern` / `ToolWarning` doc claims

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: The broader docs 05-learning INDEX and nearby sections allude to tool-usage profiles (sequence patterns, warning heuristics). Doc 02 hints at it via "heuristic extraction (analyzing tool calls and file modifications)".
**Reality**: Grep across `crates/` for `ToolUsageProfile|ToolSequencePattern|ToolWarning` returns zero matches. No struct, trait, field, or function by these names exists in the workspace. The `TemplatePatternGenerator` inspects `episode.extra` JSON rather than a typed tool-call record.
**Fix sketch**: Either (a) introduce a typed `ToolUsageProfile` struct that `TemplatePatternGenerator` consumes, or (b) remove tool-usage-profile language from the 05-learning docs and describe only what exists: episode-extra inspection.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 9 |
| PARTIAL | 3 (B.06 floor/prune conflation, B.08 partial `MatchContext`, B.10 `prune_stale` vs monotonic claim) |
| NOT DONE | 1 (B.13 tool-usage profiles) |

The knowledge-tier docs are largely faithful. Structs, confidence dynamics, globset triggers, TOML persistence, and `SystemPromptBuilder` injection are all real and tested. The main drifts are:
- **B.08 MatchContext under-populated** — file/tag/category/error_signature triggers can never fire in production because `build_learned_context` hard-codes them empty. This is the highest-impact gap in the tier and constrains the practical reach of playbook rules.
- **B.10 monotonic claim** — `prune_stale` contradicts doc 02's "no automatic pruning" but is not currently called from orchestrate; treat as either dead code or a doc correction.
- **B.03 test-count framing** — `playbook.rs` is well-tested (17 `#[tokio::test]`), not untested.

## Agent Execution Notes

### B.08 / B.12 — Learned-Context Activation

This is the highest-value runtime batch in `05`.

Recommended slice:

1. populate `MatchContext` with real task metadata,
2. decide whether learned skill retrieval should stay role-tag-only or move to `SkillQuery`,
3. add tests or runtime evidence that non-role triggers now work.

Acceptance criteria:

- playbook rules can match on more than `role`,
- the learned skill path is richer or explicitly justified,
- later agents do not need to reverse-engineer why the rule engine feels mostly dormant in production.

### B.10 / B.13 — Usually Contract Cleanup

Treat `prune_stale` and typed tool-usage-profile claims as truth-in-advertising work unless a later batch explicitly chooses to build those systems.
