# C — Cognitive Loop (Docs 09-11)

Post-audit parity notes for `docs/00-architecture/09-universal-cognitive-loop.md` through
`11-dual-process-and-active-inference.md`.

The loop story remains useful, but the stronger retelling landed ahead of runtime reality. The
audit correction is to describe `loop_tick()` as shared architecture, not as the single owner of
production control flow across the serve, CLI, and TUI surfaces.

---

## Current Runtime Truth

| Doc | Status | Current truth |
|-----|--------|---------------|
| `09-universal-cognitive-loop.md` | `partial` | `loop_tick()` exists and matters, but major orchestration still lives in open-coded paths |
| `10-three-cognitive-speeds.md` | `keep` | the scheduler model is real and safe current-state language |
| `11-dual-process-and-active-inference.md` | `narrow` | tier types and some active-inference machinery exist, but the clean EFE-driven runtime story is only partially wired |

## REF05 Posture

`REF05` should be described as a **target narrative** or **proposed migration**, not as a landed
runtime cutover.

Keep these corrections explicit:

- `roko-serve` is wired with 200+ routes.
- the TUI is wired at roughly 58K LOC.
- those live interfaces still own large control-flow paths.
- the runtime bus still exposes exactly two live `RokoEvent` variants.

Taken together, those facts mean the seven-step retelling is architecture guidance, not a literal
description of universal production ownership.

## Labeling Rule

When parity notes mention the stronger REF05 framing, use one of:

- `target narrative`
- `documentation target`
- `proposed migration`

Do not say `the universal loop already owns production control flow`.

## Rewrite Bias For Docs 09-11

Prefer:

- `shared loop helper`
- `partial runtime ownership`
- `target narrative`
- `proposed migration`

Avoid:

- `fully unified production loop`
- `complete active-inference runtime`
- `REF05 already landed`

## Batch-00 Boundary

For docs `09-11`, parity work is:

1. keep the shared abstractions visible,
2. narrow claims about runtime convergence,
3. protect current serve/TUI reality from being written out of the architecture story.
