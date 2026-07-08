# A — Composer Core

Coverage for:

- `docs/03-composition/00-composer-trait.md`
- `docs/03-composition/01-prompt-composer.md`
- `docs/03-composition/06-lost-in-the-middle-u-shape.md`

---

## Verdict

`keep`

The composition core is already real. The audit issue here is stale metadata, not missing architecture.

---

## What Exists

- `Composer` is defined in `crates/roko-core/src/traits.rs:143`.
- `PromptComposer` is implemented in `crates/roko-compose/src/prompt.rs`.
- `PromptBuild` is at `crates/roko-compose/src/prompt.rs:828`.
- U-shape ordering is real through `Placement` metadata and the composer ordering logic.
- Budget-aware composition already happens when `RoleSystemPromptSpec::compose_with_budget()` calls `PromptComposer::new().compose(...)` in `crates/roko-compose/src/role_prompts.rs:357-377`.

This part of the stack should be described in present tense.

---

## Narrow Corrections

- The docs still carry stale file/line claims around `Composer`.
- The trait includes `fn name(&self) -> &str`, which some earlier composition docs omit.
- Parameter naming drift (`signals` vs `engrams`) is cosmetic and should not drive work.

None of those justify a new composition subsystem.

---

## What Not To Do

Do not turn this section into a redesign of the composer contract.

Specifically out of scope:

- new composer traits,
- new placement systems,
- new prompt-selection algorithms beyond the existing scorer seam.

---

## Carry Forward

If follow-on code work is needed here, it should be small:

1. refresh tests or comments around the live `PromptComposer` path,
2. keep `PromptBuild` metadata discoverable,
3. leave architectural invention out of batch `03`.
