# C — Role Templates

Coverage for `docs/03-composition/03-role-templates.md`.

---

## Verdict

`rewrite`

The template subsystem is real. The parity issue is scope and ownership, not absence.

---

## Current State

`crates/roko-compose/src/templates/` is a substantial live subsystem:

- module index at `templates/mod.rs:9-28`
- shared role-template trait at `templates/mod.rs:76-88`
- per-role budget table at `templates/common.rs:17-124`

Nine relevant template modules exist under `src/templates/`:

- `assembly.rs`
- `common.rs`
- `implementer.rs`
- `integration.rs`
- `quick.rs`
- `reviewer.rs`
- `scribe.rs`
- `strategist.rs`
- `task_impl.rs`

Within those modules, `templates/mod.rs:20-28` currently exports eight concrete template structs:

- `ImplementerTemplate`
- `IntegrationTemplate`
- `QuickReviewerTemplate`
- `QuickFixTemplate`
- `ReviewerTemplate`
- `ScribeTemplate`
- `StrategistTemplate`
- `TaskImplTemplate`

Those template structs cover multiple named identities. In practice the runtime already has template-backed prompt surfaces for:

- Strategist
- Implementer
- Reviewer
- Quick Reviewer
- Quick Fix
- Scribe
- Critic
- Integration Tester
- Refactorer / single-task implementer

There are more named identities than structs because:

- `ReviewerTemplate` covers Architect, Auditor, and Combined Reviewer variants,
- `ScribeTemplate` covers Scribe, Critic, and revision variants,
- `quick.rs` contains both reviewer and fixer templates.

The important correction is that these templates are not hypothetical.

---

## What The Runtime Actually Maps

`role_identity_for(...)` in `crates/roko-compose/src/role_prompts.rs:498-530` resolves:

- template-backed identities for Strategist, Implementer, Architect, Auditor, QuickReviewer, Scribe, Critic, AutoFixer, IntegrationTester, and Refactorer,
- inline fallback strings for `Researcher` and `Conductor`.

That means the honest doc state is:

- core role-template coverage is real,
- some small fallback seams remain,
- there is no need to describe role templates as missing infrastructure.

---

## Budget Reality

The template layer already owns static budget data:

- `PromptBudget` at `templates/common.rs:17`
- `budget_for()` at `templates/common.rs:44`

What is still partial is runtime authority:

- templates still hardcode section caps in many places,
- `adjusted_budget_for()` exists in `crates/roko-compose/src/budget.rs:66`,
- the default runtime path does not yet make that function the universal source of truth.

This is a small integration gap, not a reason to design a new budgeting architecture.

---

## Keep / Narrow / Defer

Keep:

- template subsystem exists,
- shared template trait exists,
- role identities are mostly template-backed.

Narrow:

- describe inline `Researcher` and `Conductor` identities as explicit fallbacks,
- describe budget helpers as partial plumbing.

Defer:

- any broader “template engine” rewrite,
- new role systems,
- adaptive multi-policy template selection.

---

## Follow-On Batch Shape

Reasonable follow-on work from this section:

1. move more role identity text into `src/templates/`,
2. close one or two inline fallback seams,
3. reduce duplicated cap literals where practical.
