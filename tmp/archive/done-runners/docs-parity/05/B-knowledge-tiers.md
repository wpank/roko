# B — Knowledge Tiers

Audit-corrected parity view of `docs/05-learning/01-playbook-system.md` and `02-skill-library-voyager.md`.

---

## What Is Already Shipped

- `PlaybookStore`, `PlaybookRules`, and `SkillLibrary` are real and already persisted.
- prompt injection from learned skills is real.
- rule confidence updates are real.
- `roko-neuro` already has a live **tier progression** system; knowledge tiers are not hypothetical.

## What The Old Parity Material Missed

- the main runtime gap is **not** missing tiers or missing skills,
- the real gap is that production learned-context wiring still underuses the richer rule and skill selectors already implemented,
- `MatchContext` exists with files/tags/category/error signature support, but the main callsite still fills mostly `role`,
- `SkillQuery::select` exists, but the main path still leans on `search_by_tag(role)`.

## Corrected Status

### Shipping

- playbooks and playbook outcome tracking
- playbook-rule matching and confidence updates
- skill extraction and skill injection
- tier progression in `roko-neuro`

### Ship Soon

- richer `MatchContext` population from the orchestrator
- a typed heuristic calibration struct layered onto the existing heuristic/tier flow
- HDC fingerprint on `Engram` as the clean bridge into knowledge retrieval

### Deferred

- demurrage as the canonical memory model
- worldview clustering and dissonance algebra
- replication-ledger or constitutional-constraint layers

## Practical Rewrite Guidance

When touching the knowledge-tier docs:

1. describe `TierProgression` as existing today,
2. describe the production gap as **thin callsites**, not missing architecture,
3. add a caveat around `prune_stale` when discussing monotonic growth,
4. keep demurrage and worldview ideas in explicit future-work sections.

## Batch-Ready Follow-Ups

- carry forward: HDC fingerprint bridge on `Engram`
- `L1`: populate richer learned-context metadata in the main orchestrator path

## Source Anchors

- `crates/roko-learn/src/playbook.rs:77` — `Playbook`
- `crates/roko-learn/src/playbook_rules.rs:66` — `Rule`
- `crates/roko-learn/src/playbook_rules.rs:116` — `MatchContext`
- `crates/roko-learn/src/skill_library.rs:395` — `SkillQuery`
- `crates/roko-learn/src/skill_library.rs:1119` — `search_by_tag`
- `crates/roko-learn/src/skill_library.rs:1543` — `select`
- `crates/roko-cli/src/orchestrate.rs:8208` — `build_learned_context`
- `crates/roko-neuro/src/tier_progression.rs:167` — `TierProgression`
- `crates/roko-neuro/src/tier_progression.rs:207` — `analyze`

## Bottom Line

The knowledge-tier stack already exists. The parity refresh should stop implying that major tiering concepts are unbuilt and instead focus on the narrower truth: production still needs to use the richer selectors and tier hooks that are already in the codebase.
