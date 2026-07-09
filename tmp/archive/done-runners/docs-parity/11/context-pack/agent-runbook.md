# Agent Runbook — 11 Safety

Batch `11` is a constrained parity refresh for `tmp/docs-parity/11`.

## Current execution split

- `M1`: core shipped-safety docs (`00-INDEX.md`, `SOURCE-INDEX.md`, `A`, `B`, `C`)
- `M2`: threat/risk plus coverage-status refresh (`D`, `F`)
- `M3`: chain-safety defer pass (`E`)
- `M4`: context-pack source-of-truth refresh
- `M5`: final verify sweep and runner consistency

## Default posture

- Treat the shipped safety system as the starting point: **two crates, 7,183 LOC, already in repo**.
- Fix status drift before describing any new gap.
- Keep the work inside `tmp/docs-parity/11` unless a quick code-anchor check is required.
- Defer compliance, chain, kernel, and other frontier work instead of turning it into pseudo-requirements.

## What good work looks like

- core parity docs acknowledge `SafetyLayer` plus the orchestrator safety primitives as shipping
- threat/risk and Doc 16 notes read as honest status refreshes
- chain-safety notes are explicitly deferred rather than treated as same-batch implementation
- the context pack and runner agree on the current `M1`-`M5` plan

## What to avoid

- broad `docs/11-safety` rewrites from this pack
- re-opening old 7-batch planning when the runner and context pack have moved to 5 batches
- recasting frontier design material as required implementation
- conflating "shipped primitive" with "full coverage everywhere"

## Tie-breaker

Keep `BATCHES.md`, the context pack, and `run-docs-parity.sh` on the same audit-constrained `M1`-`M5` framing; if one drifts, prefer the narrowed doc/status scope over broader rewrite plans.
