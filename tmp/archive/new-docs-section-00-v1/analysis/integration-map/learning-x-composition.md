---
title: "Learning × Composition"
section: analysis
subsection: integration-map
id: im-learning-x-composition
source: 24-cross-section-integration-map.md (§6.2 M4, §3.1, §3.3)
missing-integration: M4
tier: 1
tags: [learning, composition, skills, prompts, skill-library, voyager, context-injection]
---

# Learning × Composition

**Direction**: 05-Learning → 03-Composition (skill injection into prompts); also 03-Composition → 05-Learning (playbooks, wired)  
**Status**: **Partially Wired** — `Kind::Playbook` Engrams flow from Learning to Composition; **Missing**: Skill library injection (M4 gap)  
**Interface**: `roko-learn::SkillLibrary` → `roko-compose::SystemPromptBuilder`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Playbook` | `roko-learn` | `roko-compose` | **Partial** |
| `Kind::Skill` (accumulated skills) | `roko-learn::SkillLibrary` | `roko-compose::SystemPromptBuilder` | **Missing** (M4) |
| `Kind::Episode` (outcome data) | Learning system | Playbook generation | **Partial** |
| Section/scaffold statistics | Learning runtime | Composer weights | **Partial** (Loop 3 in feedback loops doc) |

## The M4 Gap: Skill Library Not Injected

**Problem**: Accumulated skills from the Voyager-style skill library are not injected into agent prompts. The 100th modification to a crate costs the same as the 1st because proven approaches are not reused. Skills exist in the learning crate but have no consumer.

### Wiring Recipe

```rust
// In orchestrate.rs, before building system prompt:
let relevant_skills = skill_library.query_relevant(
    &task.description,
    QueryOptions { max_results: 5, min_success_rate: 0.7 }
)?;

if !relevant_skills.is_empty() {
    builder.add_section("proven_approaches", Section {
        content: render_skills(&relevant_skills),
        priority: Priority::High,
        max_tokens: 800,
    });
}
```

Estimated LOC: ~55 (source file 24, §6.2 M4).

**Cross-section enhancement**: Skills should also be indexed in Neuro (Section 06) with HDC fingerprints, enabling cross-domain skill retrieval. A skill learned for `roko-core` refactoring may be structurally similar to a `roko-agent` refactoring task — HDC similarity enables this transfer even though the crates share no vocabulary.

## Invariants of the Interaction

1. Skills injected into prompts must have `success_rate ≥ 0.7` (configurable threshold) — only proven skills are injected.
2. Skill injection must be token-budget-aware: the `Budget` constraint limits total skill tokens.
3. Skills are injected read-only — the Composition system does not modify the SkillLibrary.
4. Skill retrieval must be bounded in latency: HDC similarity search must complete within 10ms to not slow prompt assembly.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| SkillLibrary empty (early runs) | No skill injection; prompt is baseline | Graceful: section omitted |
| Skill relevance query slow | Prompt assembly blocked | Timeout with fallback to no-skill injection |
| Skills injected but unhelpful | Token budget wasted; quality unchanged | Track skill-injection vs success-rate correlation in Learning |
| Stale skills (domain shifted) | Outdated patterns injected | Skill decay via Ebbinghaus model; prune low-recency skills |

## Observed Metrics

Expected after implementation:
- Skill injection rate per task type
- Success rate comparison: tasks with vs without skill injection
- Top-5 most-frequently-injected skills (detects reliable patterns)

## Open Questions

1. Should skills have an explicit freshness/decay model (like Engram decay), or just a `last_used` timestamp?
2. HDC-based cross-domain skill retrieval (the enhancement above) requires roko-index integration — is that M4 scope or a separate M item?
3. Should skill injection be shown in the TUI dashboard? Users might want to see what skills are being applied.

## Cross-References

- Knowledge complement: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (both inject into SystemPromptBuilder; they should be ordered: Skills first, then Knowledge)
- Routing impact: [learning-x-routing.md](./learning-x-routing.md) — M6 (cost routing is a sibling Tier 1 item)
- Code context: [code-intel-x-composition.md](./code-intel-x-composition.md) — M8 (code symbols also inject into composition)
- Readiness audit: [RA-05: Learning](../readiness-audit/subsystem-learning.md), [RA-03: Composition](../readiness-audit/subsystem-composition.md)
