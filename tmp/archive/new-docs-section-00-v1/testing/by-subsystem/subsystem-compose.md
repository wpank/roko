# roko-compose — Test Coverage

> 23+ tests for the SystemPromptBuilder: 7-layer construction, token budget, and enrichment pipeline.

**Status**: Shipping
**Crate**: `roko-compose`
**Section**: 03 — Composition
**Last reviewed**: 2026-04-19

---

## Test Count: 23+

Source: implementation status audit, 2026-04-17 ("23 tests" reported; additional tests added since).

| Module | Approx. tests | Focus |
|---|---|---|
| `system_prompt_builder` | ~12 | Layer construction, U-shape placement, token budget |
| `role_templates` | ~6 | 12 role templates produce valid prompts |
| `enrichment_pipeline` | ~5 | 13-step enrichment, knowledge injection |

---

## Key Test Focus Areas

### SystemPromptBuilder (7 layers)

The builder assembles a system prompt from 7 layers:
1. Role and persona layer.
2. Cognitive state layer (Daimon PAD vector injection).
3. Context and memory layer (retrieved Engrams).
4. Task specification layer.
5. Tool availability layer.
6. Output format layer.
7. Safety and constraints layer.

Tests verify:
- All 7 layers appear in the final prompt in the correct order.
- Token budget management: if total prompt exceeds budget, layers are trimmed from lowest priority first.
- U-shape placement (Liu et al. 2023): most important context appears at the beginning and end, not the middle.
- Cache alignment: the prompt structure is stable across similar tasks, enabling KV-cache hits.

### Role Templates (12 templates)

Each role template (`Architect`, `Implementer`, `Reviewer`, `Debugger`, `Researcher`, `Planner`, `Composer`, `Optimizer`, `Validator`, `Documenter`, `Analyst`, `Coordinator`) is tested to:
- Produce a non-empty system prompt.
- Include the role-specific persona and constraints.
- Not include persona sections from other roles.

### 13-Step Enrichment Pipeline

The enrichment pipeline augments prompts with retrieved knowledge:
1. Knowledge retrieval from `roko-neuro`.
2. Episodic memory injection from `roko-learn`.
3. Playbook rule injection.
4. Daimon state injection.
5–13. (Contextual enrichment steps.)

Tests verify: enrichment steps are applied in order; a failed enrichment step does not abort the pipeline.

---

## Known Gaps

- The U-shape placement heuristic is tested only with synthetic prompts, not with real-world token distributions.
- Token budget truncation is tested only with fixed-size layers; dynamic layer sizing is not covered.
- Integration tests for the full `roko-compose → roko-agent → LLM` pipeline are thin.

## See also

- [../by-property/prompt-layer-ordering.md](../by-property/prompt-layer-ordering.md)
- [subsystem-agent.md](subsystem-agent.md) — agent uses the composed prompt
