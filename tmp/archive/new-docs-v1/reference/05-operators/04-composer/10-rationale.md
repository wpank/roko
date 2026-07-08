# Composer Rationale

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Why a Trait?

Different models need different prompt formats (ChatML, Llama-style, raw text). A trait lets you swap `Composer` implementations per model without changing the loop.

## Why 7 Layers?

The layers map to distinct semantic concerns in a system prompt: who you are (Role), what you must not do (Safety), what you should do now (Task), what you know (Context + Memory), how to respond (Format), and where you are (Metadata). Merging any two layers makes them harder to tune independently.

## Why U-Shape Placement?

The lost-in-the-middle effect is well-documented ([Liu et al., 2023](https://arxiv.org/abs/2307.03172)). U-shape placement is a zero-cost mitigation — the same information, different order.

## Open Questions

- Should token estimation be pluggable (model-specific tokeniser)?
