# B — System Prompt Builder

Coverage for `docs/03-composition/02-system-prompt-builder-7-layer.md`.

---

## Verdict

`rewrite`

The old parity file treated the builder as a partly-built design doc. That is no longer accurate.

---

## Current State

The production entrypoint is already wired:

- `RoleSystemPromptSpec` in `crates/roko-compose/src/role_prompts.rs:154-482`
- `SystemPromptBuilder` in `crates/roko-compose/src/system_prompt_builder.rs:53-493`
- CLI helper path in `crates/roko-cli/src/prompting.rs:25-72`
- orchestration callsites in `crates/roko-cli/src/orchestrate.rs:11709-11727` and `:14263-14295`

From a parity perspective, this should be described as a wired builder path with a six-layer practical contract:

1. role identity,
2. conventions,
3. domain/relevant context,
4. task context,
5. tool instructions,
6. learned/task-local guidance.

The concrete builder struct has more optional sections than that:

- pheromones,
- playbooks/skills,
- anti-patterns,
- affect guidance,
- cache-marker toggles,
- section-effectiveness adjustments,
- token-budget enforcement.

Those are implementation details around an already-live runtime path, not evidence that the whole subsystem is still aspirational.

---

## Keep In Present Tense

- `SystemPromptBuilder::new(...)` is live at `system_prompt_builder.rs:120`.
- `build_sections()` is live at `:336`.
- `with_cache_markers()` is real at `:222`.
- `with_section_effectiveness(...)` is real at `:236-245`.
- `build_with_counter(...)` is real at `:265-327`.
- `RoleSystemPromptSpec` feeds the builder through `builder_with_section_effectiveness(...)` at `role_prompts.rs:280-318`.

---

## Narrow Or Reframe

- The docs should stop centering layer-count novelty. The important fact is that the builder is already in production.
- Cache markers are real but secondary.
- Learned section-effectiveness is wired as a registry input, not a full learned prompt-policy engine.
- Layer reordering and compression-controller ideas belong in a deferred section unless code is shipped.

---

## Deferred

Keep these out of present-tense parity claims:

- learned layer-order policy,
- compression-controller / per-layer compression strategy,
- any claim that the builder is backed by a full evaluation harness.

---

## Follow-On Batch Shape

If code work follows from this section, it should be small:

1. strengthen tests around the live builder path,
2. keep `RoleSystemPromptSpec` as the documented entrypoint,
3. avoid “new builder architecture” work.
