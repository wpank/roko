# Composer Overview

> `Composer` takes the cognitive loop's context — recalled memories, the selected action,
> agent configuration, and the current input — and assembles them into a system prompt for
> the language model.

**Status**: Shipping
**Crate**: `roko-compose`
**Depends on**: [Router](../03-router/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

`Composer::compose(ctx, action) -> PromptOutput` builds the LLM prompt. The shipping
implementation is `SystemPromptBuilder` — a 7-layer builder that constructs prompts by
stacking role, context, memory, task, format, safety, and metadata layers in a configurable
order.

---

## What Composition Does

After the Router selects an action, the agent must tell the LLM what to do. The Composer
converts the machine-readable loop state (engrams, scores, action kind) into the natural-
language prompt the LLM will process.

The Composer is responsible for:
1. **Selecting which recalled memories to include** — not all 16 recalled engrams may fit
   in the context window.
2. **Ordering the prompt sections** — research shows that LLMs attend to content at the
   beginning and end of a prompt better than the middle (the "lost-in-the-middle" effect).
   The U-shape placement strategy puts the most important content at both ends.
3. **Formatting** — adapting to the model's preferred prompt format (ChatML, raw string,
   structured JSON).

---

## The 7-Layer Prompt Structure

`SystemPromptBuilder` constructs prompts as a stack of named layers:

| Layer | Contents | Default position |
|---|---|---|
| 1. Role | Agent persona, capabilities declaration | Top |
| 2. Safety | Hard constraints, prohibited outputs | Top |
| 3. Task | Current action description | Upper middle |
| 4. Context | Retrieved memories (most salient) | Middle (U-shape start) |
| 5. Memory | Additional recalled engrams | Middle |
| 6. Format | Output format instructions | Lower middle |
| 7. Metadata | Agent version, session ID, timestamp | Bottom |

---

## See Also

- [Implementation](./03-implementation.md) — 7-layer SystemPromptBuilder
- [Placement Strategies](./09-placement-strategies.md) — U-shape placement
- [Trait Surface](./01-trait-surface.md)
