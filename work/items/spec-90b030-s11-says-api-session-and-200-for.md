+++
id = "spec-90b030"
kind = "spec"
title = "S11 says /api/session and 200 for passphrase login; the shipped route is /api/auth/session and 204"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate-13c follow-up reports 2026-10-04 (PK86 gap-3516d6, task 9337)"
discovered_from = "gap-3516d6 (closed; preflight.sh built correctly against the real route, spec never updated)"
anchors = ["tmp/cybernetic-harness/specs/S11-fly-deploy-security.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

S11's prose disagrees with the shipped code in two ways, throughout the document rather than
only in §7: the session route is `/api/auth/session`, not `/api/session`, and a successful
passphrase login answers `204 No Content`, not `200`.

Code (confirmed at main HEAD `908f7ec40`): `crates/roko-serve/src/routes/auth_session.rs`
mounts `GET`/`POST`/`DELETE` all at `/api/auth/session` (module doc comment, lines 13-15; the
router registration at line 454). `passphrase_login` (lines 262-361, S11 §4.3) returns, on
success, `(StatusCode::NO_CONTENT, [(SET_COOKIE, cookie)]).into_response()` (line 361) — `204`,
with no response body, only the `Set-Cookie` header.

`tmp/cybernetic-harness/specs/S11-fly-deploy-security.md` says `/api/session` (not
`/api/auth/session`) at lines 81, 82, 87, 214, 306, 307, 308 and 407, and says `POST
/api/session` answers `200` with a JSON body (`{"authenticated":true,"scopes":[...],...}`) at
line 306, again at P7 (line 404: "→ 200") and again at A9 (line ~412: "new passphrase 200") —
three separate places claiming 200, all contradicted by the actual 204/empty-body response. The
one place the spec gets the status right is line 308, `DELETE /api/session` → `204` (correct
status, wrong path). So this is a document-wide naming/status drift, not a two-line typo in §7.

## Why it matters

Goal: release, S11 fly-deploy security. Anyone implementing a client against this spec (the
demo app, a test, an operator's curl script) following the documented path or status would get
it wrong: `/api/session` 404s (the real route is `/api/auth/session`), and code that branches on
`200` for a successful login never fires (the real response is `204`). The acceptance-criteria
checks (P7, P13, A9) are exactly the kind of text someone runs by hand during a deploy.

## Where

- `tmp/cybernetic-harness/specs/S11-fly-deploy-security.md` — every `/api/session` occurrence
  (lines 81, 82, 87, 214, 306, 307, 308, 407) to `/api/auth/session`; the `200` at line 306 and
  at P7/A9 (§7) to `204`; line 306's documented success body to "no body" (204 has none).
- `crates/roko-serve/src/routes/auth_session.rs` (read-only reference; not touched by this fix).
- Leave `docs/whitepaper/*` untouched (held for the paper-rewrite session); this is
  `tmp/cybernetic-harness/specs/`, which is in scope.

## Current state

The code and its own tests (`auth_session.rs`'s test module, e.g. the requests built against
`/api/auth/session` at lines 520-665) are self-consistent and correct. Only the spec prose is
stale.

## Plan

1. Replace every `/api/session` in `S11-fly-deploy-security.md` with `/api/auth/session`.
2. Replace the `200` success status for `POST /api/auth/session` with `204` at line 306, P7 and
   A9, and drop or correct the documented JSON success body at line 306 (a 204 has none).
3. Leave `DELETE .../session` → `204` as is (already correct besides the path).

## Done when

- S11's prose matches `auth_session.rs`'s actual route and status codes everywhere it describes
  the session routes.

## Notes

- 2026-10-04 (gate-13c follow-up, PK86 gap-3516d6, task 9337): confirmed at main HEAD
  `908f7ec40`. No `[[verify]]` command: the fix is spec-text only, filed as `kind = "spec"` per
  the instruction to file a spec item for this.
