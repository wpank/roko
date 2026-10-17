+++
id = "gap-ac2f31"
kind = "gap"
title = "Serve's M1 route reports on while M1 runs in shadow on roko's shipped viability policy, and stamps a person's rows with policy_version 0"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-serve/learning"]
created = 2026-10-16
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "d57ab941f"
source = "gap-cad57a"
discovered_from = "gap-cad57a"
anchors = ["crates/roko-serve/src/routes/learning/homeostasis.rs::view", "crates/roko-serve/src/routes/learning/homeostasis.rs::record"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = ["gap-cad57a"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn homeostasis_route_reports_shadow_on_the_shipped_policy' crates/roko-serve/ && cargo test -p roko-serve homeostasis_route_reports_shadow_on_the_shipped_policy"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T03:02:39Z"
commit = "d57ab941f"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T02:10:41Z"
forced = false
evidence = "d57ab941f: crates/roko-serve/src/routes/learning/homeostasis.rs `view` now computes mode via ViabilityPolicy::load_or_shipped + mode_on_policy (the same check roko learn homeostasis status and HomeostasisSink::for_workdir already use) and returns a new policy: \"person\"|\"shipped\" field; `record` takes policy_version from load_or_shipped instead of load_optional, so a fresh workspace's controller.mode/hold rows carry 1, not 0. Proven by the new test homeostasis_route_reports_shadow_on_the_shipped_policy (fresh workspace: admin sets on, view reads mode=shadow/policy=shipped, ledger row policy_version=1; after a person's policy file is written, view reads mode=on/policy=person) and a new policy=\"person\" assertion added to the pre-existing homeostasis_route_reports_mode_state_theta_lkg; both pass under cargo test -p roko-serve."
+++

## Problem

Since gap-cad57a, M1 runs on roko's shipped viability policy when a workspace has none, or has the unedited text
`roko init` writes, and `on` then runs in shadow (`roko_learn::homeostasis::policy::mode_on_policy`, used by
`HomeostasisSink::for_workdir`). Serve's M1 route does not know this:

- `GET /api/learn/homeostasis` (`crates/roko-serve/src/routes/learning/homeostasis.rs::view`) reports `mode` as the
  person's mode from the admin route, else the config's. After `POST /api/learn/homeostasis/mode {"mode":"on"}` in a
  workspace without a policy of its own it says `on`, while every run opens in shadow.
- `record` stamps the `controller.mode` and `controller.hold` rows a person writes with the `policy_version` of
  `ViabilityPolicy::load_optional`: 0 without a policy file, where the controller's own rows carry the shipped
  policy's 1.

## Why it matters

Goal `cybernetic` (spec-7a71d6, stage 2). S10's Homeostat panel renders this route: a person who turns M1 on through it
is told M1 is on when it is not, and nothing says a policy of their own is missing. `roko learn homeostasis status`
already reports the mode the next run opens in (gap-cad57a).

## Where

- `crates/roko-serve/src/routes/learning/homeostasis.rs::view`: the reported mode.
- `crates/roko-serve/src/routes/learning/homeostasis.rs::record`: the `policy_version` of a person's rows.
- `crates/roko-learn/src/homeostasis/policy.rs::mode_on_policy` and `ViabilityPolicy::load_or_shipped`: what M1 does.

## Current state

gap-cad57a added `ViabilityPolicy::load_or_shipped`, `PolicyOrigin` and `mode_on_policy`, and used them in
`HomeostasisSink::for_workdir` and in `roko learn homeostasis status`. Serve was left as it was.

## Plan

1. `view`: report `mode_on_policy(person's mode, else the config's, origin)` with the origin from
   `ViabilityPolicy::load_or_shipped`, and add `"policy": "person" | "shipped"` so the panel can say why `on` is shadow.
2. `record`: take `policy_version` from `load_or_shipped`, so a person's row names the policy M1 runs on.
3. Test `homeostasis_route_reports_shadow_on_the_shipped_policy`: in a fresh workspace an admin sets `on`; the view
   says `shadow` and `shipped`, and the mode row carries `policy_version` 1; once a person's policy file is there it
   says `on` and `person`.

## Done when

The `[[verify]]` command passes.

## Notes

Keep `set_mode` accepting `on`: the person's choice stands for when a policy of their own is written.
