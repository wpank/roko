+++
id = "bug-48494b"
kind = "bug"
title = "Portal validation badge shows valid for invalid plans: the validate call gets 400 and a failed request renders as valid"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "M"
subsystem = ["apps/portal", "roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "6e01e441f"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/api/queries.ts::useValidation", "apps/portal/src/components/stage/ValidationBadge.tsx::ValidationBadge", "crates/roko-serve/src/routes/plans.rs::ValidatePlanRequest", "crates/roko-serve/src/plan_types.rs::PlanValidationDto", "apps/portal/src/components/shell/Workspace.tsx:153"]
links = { depends_on = [], blocks = [], related = ["gap-655d19"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo build -p roko-cli && bash -c 'source plans/portal-programme/_harness/lib.sh && require_binary && make_workspace && start_server && [ \"$(api POST /api/plans/live-b/validate \"{}\")\" = 200 ] && jcheck \"$WS/last.json\" \"d[\\\"valid\\\"] is True and isinstance(d[\\\"errors\\\"], list) and isinstance(d[\\\"warnings\\\"], list)\"'"

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'validation request fails' apps/portal/src && (cd apps/portal && npx vitest run src/components/stage)"

[closed]
at = 2026-09-29
commit = "6e01e441f"
evidence = "6e01e441f: POST /api/plans/{id}/validate accepts no body, {} and {\"toml\": null} and answers errors/warnings as arrays (roko-serve tests/plan_authoring.rs, 17 passed; routes::plans lib tests, 48 passed). Portal posts no body; a failed request shows 'validation failed', never 'valid ✓' (components/stage/validation.test.tsx, 11 passed, 9 fail on 70820a74c; full suite 63 files / 692 tests). verify[1] passes against the rebuilt binary; AUTHORING-CHECK PASS (36), REVISION-CHECK PASS (15). Headless Chromium on live-b with depends_on = [\"T99\"]: badge '1 ERROR', Run disabled with 'Fix 1 validation error first', alert '1 validation error', no failed validate request."
+++

## Problem

The plan view's validation badge reads `VALID ✓` for every plan, invalid ones included, and Run is
never disabled for a validation error. Reproduced on 2026-09-29 in a harness workspace
(`plans/portal-programme/_harness/lib.sh` `make_workspace`) with `plans/live-b` edited to
`depends_on = ["T99"]`: `POST /api/plans/live-b/validate` with no body answers
`{"valid":false,"errors":1,"warnings":1,"diagnostics":[{"rule_id":"PLAN_005",…}]}`, yet the portal at
`/?plan=live-b` shows `1 task · 1 wave · parallel 1 · VALID ✓` with ▶ Run enabled, while the wave
strip beside it prints "T01 depends on T99, which does not exist" in red.

Three faults combine:

1. `useValidation` posts the body `{}`. The server's `ValidatePlanRequest { toml: String }` requires
   `toml` whenever a body is present, so every validate call the portal makes answers
   `400 {"code":"invalid_json","details":{"reason":"missing field `toml` at line 1 column 2"}}`.
   The contract (`tmp/portal-audit/03-CONTRACT.md`, AS BUILT 04 §C) specifies the body as
   `{"toml"?: "<string>"}`, with `toml` optional.
2. The server returns `errors` and `warnings` as counts (`PlanValidationDto.errors: usize`), while
   the contract (§C) and the portal's `WireValidation` type say arrays of strings. The badge computes
   `data?.errors?.length ?? 0`, which is 0 for a number, so even a successful response would read as
   valid. `PUT /api/plans/{id}/source` returns the same counts in its 200 and 422 bodies.
3. `ValidationBadge` renders `valid ✓` whenever the request fails with any status but 404/405:
   `data` is undefined, so both counts are 0.

Separately, `Workspace.tsx:153` passes `validationErrors: 0` to `pickAlert`, so the design §8
validation alert can never fire.

Every 09 run logged these 400s in the browser console (`tmp/portal-audit/evidence/*/browser-*.json`:
6 in the fake-agent flow, 1 in the parallel flow, 3 in each of the two real-model runs). The checks do not assert
on console errors, and none opens an invalid plan, so both checks passed.

## Why it matters

Goal `visibility`. The badge and "Run disabled while validation has errors" (design §4.1, §4.2) are
the portal's only pre-run check, and contract ask P-3 exists for them. A false `valid ✓` invites
running a plan that the run path will reject. Server-started runs also skip the PLAN_0xx checks
(gap-655d19), so nothing stops the run before it fails.

## Where

- `crates/roko-serve/src/routes/plans.rs::validate_plan` and `ValidatePlanRequest`: body parsing.
- `crates/roko-serve/src/plan_types.rs::PlanValidationDto`: `errors`/`warnings` are `usize`.
- `apps/portal/src/api/queries.ts::useValidation`: posts `{}`.
- `apps/portal/src/components/stage/ValidationBadge.tsx`: label from `errors.length`, no error state.
- `apps/portal/src/components/stage/PlanView.tsx::usePrimaryAction`: disables Run only on
  `validation.errors.length`.
- `apps/portal/src/components/shell/Workspace.tsx:153`: `validationErrors: 0`.

## Current state

Plan 04's AUTHORING-CHECK verified validate with no body and with `{toml}` over HTTP. Nothing
exercised `{}` or the array shape. The portal (plans 05–08g) was built against the contract and never
met this server until plan 09.

## Plan

1. Server: make `toml` optional (`Option<String>` with `#[serde(default)]`) so that `{}` validates the
   file on disk, and return `errors`/`warnings` as the contract's string arrays. Alternatively, amend
   the contract and `WireValidation` together. Either way, pick one shape and test it on both sides.
2. Portal: derive the badge from `valid` and `diagnostics`; show an explicit "validation failed" state
   on a request error instead of `valid ✓`; pass the real error count to `pickAlert`.
3. Add a server test for `{}` and a portal test for the failure state.

## Done when

- `POST /api/plans/<id>/validate` with `{}` answers 200 with `valid` and array-typed `errors` and
  `warnings` (first `[[verify]]`, fake-agent harness).
- A portal test whose name contains "validation request fails" shows the badge does not claim
  validity on a failed request (second `[[verify]]`).
- An invalid plan shows its error count in the badge, and ▶ Run is disabled with the reason.

## Notes

Found by plan 09 T04: a scratch probe logged every HTTP response ≥ 400 during the portal-check flow
(see VERDICT). Do not weaken the 09 checks. Once this is fixed, consider making `browser-flow.cjs`
fail on console errors.
