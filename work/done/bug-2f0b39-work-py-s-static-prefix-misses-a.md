+++
id = "bug-2f0b39"
kind = "bug"
title = "work.py's static_prefix misses a cargo step behind env -i and treats docker build as static, so verify --static-only ran a cargo test and a Docker image build"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "0332ba894"
source = "gap-6ee3c5 worker, 2026-10-16"
discovered_from = "gap-6ee3c5: verify --closed-since 2026-09-26 --static-only started a roko-cli build and a Docker image build"
anchors = ["tools/work.py::is_heavy", "tools/work.py::static_prefix", "tools/test_work.py"]
lane = "tracker"
links = { depends_on = [], blocks = [], related = ["dec-01be49", "gap-fcb44c", "gap-de8fdc", "gap-6ee3c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import sys; sys.path.insert(0, 'tools'); import work; sys.exit(work.static_prefix('env -i HOME=x cargo test -p c t') != '' or work.static_prefix('grep -q x f && docker build -t t .') != 'grep -q x f')\""

[closed]
at = 2026-10-16
at_ts = "2026-10-16T19:36:50Z"
commit = "0332ba894"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:35Z"
forced = false
evidence = "0332ba894: tools/work.py HEAVY_ANYWHERE lets env/nice/xargs/time/exec take options with an optional argument (and timeout options before its duration), HEAVY_CMD (shared by HEAVY and HEAVY_ANYWHERE) adds docker, podman, make and deploy/*.sh, and the new HEAVY_ENV makes a command run with *REAL_ROKO*= or *ROKO_BIN*= in its environment heavy. Over the tracker's 1,830 verify commands exactly 7 static prefixes change, all real heavy steps: dec-01be49 (env -i ... cargo test), gap-fcb44c#8 (docker build), gap-31c0e8 and gap-127263#2 (VB_REQUIRE_REAL_ROKO=1 pytest), gap-3516d6 (preflight.sh --local) and the open gap-00e882 and gap-31e1b0 (preflight.sh against fly.dev). verify --static-only dec-01be49 gap-fcb44c gap-31c0e8 now runs only greps, test -f and a tomllib parse, finishes at once and creates no target/. The verify passes; test_static_prefix_stops_at_a_heavy_command_anywhere_in_a_part covers every form and fails on the old regexes; test_work.py 45 OK; check 0 problems. Left static on purpose: git-only python checks, and viabilitybench verify_verifiers.py --families f7, which runs a seconds-long offline cargo test of a one-file crate inside the script."
+++

## Problem

`python3 tools/work.py verify --closed-since 2026-09-26 --static-only` runs heavy commands that `static_prefix` takes
for static ones:

- dec-01be49's verify is `env -i HOME="$(mktemp -d)" CARGO_HOME=… PATH=… cargo test -p roko-cli --lib doctor::tests::`.
  `HEAVY_ANYWHERE` accepts `env` followed only by `VAR=value` assignments; an option before them (`-i`, `-u NAME`)
  breaks the match, so the whole command counts as static.
- gap-fcb44c's last verify runs `docker build --target showcase-replay -t roko-showcase-replay:verify .`. `docker` is
  not one of the heavy commands (`cargo`, `npx`, `npm`, `pnpm`, `node`, `roko`, `target/`, `curl`, `gh`).

Seen on 2026-10-16 while running the static regression sweep for gap-6ee3c5: a worker told not to run cargo got a cold
roko-cli build (7 GB of `target/` in its worktree) and a Docker image build from `--static-only` runs; both were
stopped by hand.

## Why it matters

Goal `tooling`: `--static-only` promises never to run a cargo, npm or network step, and `close`, `sync` and
`retire-verify --static-only` judge an item by the same static prefix. A verify written as `env -i … cargo …` (the
pattern for running a test without the caller's environment) or one that builds an image silently starts a long build
there, and `close`/`sync` give it only 60 seconds.

## Where

`tools/work.py`: `HEAVY`, `HEAVY_ANYWHERE`, `is_heavy` and `static_prefix`; the test
`test_static_prefix_stops_at_a_heavy_command_anywhere_in_a_part` in `tools/test_work.py`.

## Current state

`work.static_prefix('env -i HOME=x cargo test -p c t')` returns the whole command (it should return `''`), and
`work.static_prefix('grep -q x f && docker build -t t .')` returns the whole command (it should return `grep -q x f`).

## Plan

1. Let the `env` alternative in `HEAVY_ANYWHERE` take options before the assignments, for example
   `env(?:\s+-[A-Za-z-]+(?:\s+[A-Za-z_]\w*)?)*` (`-i`, `--ignore-environment`, `-u NAME`).
2. Add `docker` (and `podman`, `make`, `pytest`-style test runners if a done item's verify uses them) to the heavy
   commands in both regexes. Grep the done items' static prefixes for other long-running commands while there.
3. Add both forms to the test above.

## Done when

The `[[verify]]` command passes, and `test_work.py` stays green.

## Notes

Tracker lane: `tools/work.py` is one file. Related: gap-de8fdc (verifies that can never pass or never fail).
gap-31c0e8's static part also runs a viabilitybench pytest when `target/debug/roko` exists.
