# E — Web UI, Onboarding UI, and A2UI

Refresh target for docs 13-15: separate the live CLI bootstrap from the absent
browser/UI runtime work.

Generated: 2026-04-18

---

## Headline

- First-party web UI is deferred. There is zero shipping frontend code for the
  SvelteKit / portal vision.
- A2UI is deferred. There is zero shipping agent-generated UI runtime.
- Onboarding is mixed: the CLI bootstrap exists, but the guided onboarding UI
  does not.

## Rewrite Guidance

### Shipping Baseline

- `roko init`
- `roko run`
- `roko prd idea`
- `roko prd plan`

These are enough to describe a functional CLI onboarding baseline.

### Deferred

- SvelteKit or any other first-party browser app
- web onboarding flow / first-run wizard
- A2UI schema and renderers
- generative-interface safety claims that depend on A2UI actually existing

## Framing To Use

- "`roko-serve` provides the backend surface a future web UI could consume"
- "the current onboarding path is CLI-first"
- "the portal, onboarding UI, and A2UI remain target-state work"

## Framing To Avoid

- "portal ships but is incomplete"
- "A2UI is partially implemented"
- "onboarding UI exists because the CLI has init and PRD commands"
