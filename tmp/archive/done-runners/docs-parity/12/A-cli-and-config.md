# A — CLI and Config Truth Pass

Refresh target for docs 00-04: keep the large live CLI surface, correct the
two non-shipping command concepts, and avoid turning REF28 into present-tense
feature claims.

Generated: 2026-04-18

---

## Headline

- The CLI is already a shipping primary surface. Treat it as wired, not
  aspirational.
- The two important corrections are:
  - `roko new` does not ship as a top-level command.
  - standalone `roko explain` does not ship as a top-level command.
- Config layering, `roko init`, and the core plan / PRD / research / dashboard
  / serve flow all ship.

## Verified Anchors

| Surface | Status | Notes |
|---|---|---|
| Top-level CLI command tree | Shipping | `crates/roko-cli/src/main.rs:191-344` shows a large live command surface |
| `roko init`, `run`, `plan`, `prd`, `research`, `status`, `replay`, `dashboard`, `serve`, `chat` | Shipping | treat these as the baseline operator workflow |
| Config commands | Shipping | `config init/show/path/edit/set/set-secret` are in the live CLI |
| `roko new` | Not shipping | searches can be confused by `PrdDraftCmd::New` in `main.rs:503-520`; that is not a top-level scaffolder |
| standalone `roko explain` | Not shipping | the nearby real surface is `model route --explain` in `main.rs:658-677` |

## Rewrite Guidance

### Keep

- CLI overview as the main operator entry point.
- Existing command families: plan, PRD, research, config, dashboard, serve,
  chat, daemon, provider, model, subscription, and event-source work.
- Layered configuration as a real behavior, tied to the shipping config
  commands rather than to a future wizard.

### Narrow

- `roko new` should move from "current command" language to a deferred
  scaffolding concept.
- Doc 03 should stop describing `roko explain` as though it ships.
- "Interactive onboarding" should mean the current CLI baseline unless a UI
  wizard is explicitly labeled future work.

### Ship Soon, Not Shipping

- REF28 CLI parity / muscle memory is still worth doing:
  - cleaner `roko` entry behavior,
  - better chat ergonomics,
  - tighter diff-first review and transcript muscle memory.
- Keep those items in "next-step" language, not "already true" language.

## Practical Notes

- If a rewrite needs a live explain surface, cite `model route --explain`.
- If a rewrite needs a "new" command, distinguish `prd draft new` from the
  absent generic scaffolder.
- Do not widen this section into four-layer SDK, domain profiles, or other
  non-interface roadmap work.
