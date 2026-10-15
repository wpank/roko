+++
id = "bug-0c7f25"
kind = "bug"
title = "Demo plans run from Hermes in the roko repo are refused by the spec gate or end in a workspace-wide cargo clippy"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["plans/demos"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "622598e5d"
source = "tmp/hermes-roko-v5/findings/roko-demos.md (roko-demos, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: Will's hello-world demo from Hermes"
anchors = ["plans/demos/parallel-plans/demo-hello-world/tasks.toml", "plans/demos/demo-hello/tasks.toml", "plans/demos/demo-release-readiness/tasks.toml", "plans/demos/demo-resume-recovery/tasks.toml", "plans/demos/README.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "out=\"${TMPDIR:-/tmp}/roko-demos-spec-quality.json\" && cargo run -q -p roko-cli --bin roko -- plan validate --spec-quality --dynamic --json plans/demos > \"$out\" && python3 -c 'import json, pathlib, sys, tomllib; report = json.load(open(sys.argv[1])); hf3 = [t[\"plan_path\"] + \"#\" + t[\"task_id\"] for t in report[\"spec_quality\"][\"tasks\"] if \"HF3\" in t[\"hard_fail\"]]; plans = [p for p in pathlib.Path(\"plans/demos\").rglob(\"tasks.toml\") if not any(part.startswith(\"_\") for part in p.parts)]; rust = [str(p) for p in plans for doc in [tomllib.load(open(p, \"rb\"))] if any(f.endswith(\".rs\") for t in doc.get(\"task\", []) for f in t.get(\"files\", [])) and not doc[\"meta\"].get(\"verify\")]; assert not hf3 and not rust, (\"green on base\", hf3, \"rust demos without [meta] verify\", rust)' \"$out\""

[closed]
at = 2026-10-15
at_ts = "2026-10-15T14:54:56Z"
commit = "622598e5d"
size = "M"
claimed_at = "2026-10-15T14:22:08Z"
forced = false
evidence = "Plans fixed in 53c03e8a8 + 622598e5d: [meta] verify on the six Rust demos, dated demo-hello output, stale release-readiness/resume-recovery outputs removed, resume demo under plans/demos/_cli, start-here title, README. 'roko plan validate --spec-quality --dynamic --json plans/demos' on the branch: no HF3 (was 3: DEMO-T01, REL-3, RESUME-1) and no Rust demo without [meta] verify. Live from Hermes' plan route (plugin 0.4.0, roko-main 7b6a5af44, scratch clone): demo-hello-world succeeded 46 s $0.005, demo-hello 42 s $0.012, demo-print-hello 49 s $0.004, demo-fibonacci 59 s $0.019, demo-release-readiness 4m08 $0.064, demo-hello-color 2m $0.017, demo-greeting-module 36 s $0.042; demo-hello-world also on Will's serve binary 216f88c84 (83 s $0.004). Evidence: tmp/hermes-roko-v5/findings/roko-demos.md"
+++

## Problem

Run from Hermes' Plans tab (`POST /api/plugins/roko/plans/{id}/run`, roko serve in the roko repo, 2026-10-15,
`tmp/hermes-roko-v5/findings/roko-demos.md`), the demo plans a newcomer clicks do not end cleanly:

- `demo-hello` is refused before dispatch by the spec gate, `HF3: verify passes on the unchanged base`:
  `demo/roko-plan-smoke-output.md` has been checked in since 2026-08-05 (`e2d512071`, an old run's commit), so
  its only verify step already passes. Its `plan.md` still says the file is "not checked in yet".
  `demo-release-readiness` (REL-3) and `demo-resume-recovery` (RESUME-1) are refused the same way
  (`roko plan validate --spec-quality --dynamic plans/demos` lists all three).
- `demo-hello-world`'s task passes in 13 s for $0.004, then the run spends many minutes in the delivery's
  post-merge regression check, `cargo fmt --all -- --check` and `cargo clippy --workspace --no-deps -- -D
  warnings`, a cold build of the whole workspace. The default whole-plan check (`plan_verify.rs`) covers every
  package because `cargo metadata --offline` fails in this repo (a separate item), so any `.rs` file counts as
  touching every package. Every demo whose tasks write `.rs` files (`demo-hello-world`, `demo-print-hello`,
  `demo-fibonacci`, `demo-hello-color`, `demo-greeting-module`, `demo-full-stack`) ends the same way.
- `demo-resume-recovery` needs `ROKO_RESUME_DEMO_READY=1` set by hand for its second run, which neither Hermes
  nor the portal can do, so from there it can only end failed at 1/2.
- `demo-hello` checks with `rg`, which a newcomer may not have.

## Why it matters

The first thing a person does in Hermes' Plans tab is run a hello-world demo. Today in the roko repo it is
refused (`demo-hello`), or it takes a quarter of an hour of someone else's build and may fail on lints the
demo never touched (`demo-hello-world`). Goal `hermes`.

## Where

- `plans/demos/*/tasks.toml`, `plans/demos/*/*/tasks.toml`: the demos' tasks and `[meta]`.
- `plans/demos/README.md`: the catalogue, which still says every demo passes `--strict` and that checked-in
  outputs only make reruns do less.
- `crates/roko-cli/src/graph_execution/plan_verify.rs::plan_verify_steps`: an authored `[meta] verify`
  replaces the cargo default.
- `crates/roko-cli/src/spec_red_on_base.rs`: the HF3 measurement the spec gate refuses on.

## Plan

1. Give every demo that writes `.rs` files an authored `[meta] verify` that checks its own output, so no demo
   falls back to the workspace-wide cargo check.
2. `demo-hello`: its line 3 carries the run's UTC date, so the checked-in file (or a merged earlier run)
   no longer passes; check with `grep` instead of `rg`.
3. `demo-release-readiness`: remove the stale outputs an Aug 5 run committed, so its checks are red on base.
4. `demo-resume-recovery`: move it to `plans/demos/_cli/`, which plan discovery skips (so Hermes, the portal
   and `roko plan list` do not offer it); `roko plan run plans/demos/_cli/demo-resume-recovery` still runs it.
5. Make the first demo obvious: `demo-hello-world`'s title says to start there.
6. Update the README: which demos run from Hermes, that outputs are not checked in and why, the `_cli` folder.

## Done when

`roko plan validate --spec-quality --dynamic plans/demos` reports no HF3, and every discoverable demo that
writes `.rs` files has a `[meta] verify` (the `[[verify]]` command). `demo-hello-world` and `demo-hello` pass
from Hermes in about a minute for cents (recorded in the findings file).

## Notes

- Keep `demo-hello-world`'s output contract, so it still passes with the fake agent
  (`plans/portal-programme/_harness/fake-claude`) and the README's quick start still works.
- Do not touch `_fixtures/`. The cargo metadata failure and the Hermes-side fixture flag are separate items.
