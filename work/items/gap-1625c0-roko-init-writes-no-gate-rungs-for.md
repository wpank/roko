+++
id = "gap-1625c0"
kind = "gap"
title = "roko init writes no gate rungs for a Python project (and none without --profile anywhere), so its prompt runs end unverified"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/init"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "7b6a5af44"
source = "hermes-plugin-roko dogfood, 2026-10-13 (roko serve :6716 built from 6ab6fde47)"
anchors = ["crates/roko-cli/src/commands/init.rs::ProjectGates", "crates/roko-cli/src/commands/init.rs::InitProfile", "crates/roko-core/src/project.rs::detect_from_files", "crates/roko-cli/src/run.rs::prompt_verify_steps"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn python_profile_writes_a_test_rung' crates/roko-cli/ && cargo test -p roko-cli --lib python_profile_writes_a_test_rung"
+++

## Problem

`roko init` gives a Python project no gate rungs, so every prompt run there ends `unverified`:

- In a stdlib-only Python project (a package directory plus `tests/` using `unittest`; no `pyproject.toml`,
  `requirements.txt` or `setup.py`), `roko init` prints "detected project domain: general" and writes
  `rungs = []`.
- `roko init --profile python` prints "suggested gates: test (pytest), lint (ruff), typecheck (mypy)", but still
  writes `rungs = []`: `profile_gate_rungs` knows only `rust` and `typescript`. The suggested tools may not even be
  installed, while `python3 -m unittest discover` needs nothing.
- Even in a detected Rust or TypeScript project, plain `roko init` writes no rungs: `detect_init_profile` reads only
  the `--profile` flag and never the detected domain. The domain is printed but not used.
- With no rungs, `prompt_verify_steps` gives a project that is not Cargo or Go no verify step. Every prompt run
  started from a chat host then ends `unverified`.

## Why it matters

The Hermes guide tells people to run `roko init` in their project and then ask for changes. Outside Rust that
yields runs nobody checked, which defeats the point of gated runs (goal `hermes`; first-run experience).

## Where

- `crates/roko-cli/src/commands/init.rs::profile_gate_rungs`, `detect_init_profile`, `append_no_profile_gate_hint`
- `crates/roko-cli/src/commands/util.rs::detect_project_domain`, `domain_gate_hint`
- `crates/roko-cli/src/run.rs::prompt_verify_steps`

## Current state

The dogfood set its rung by hand: `{ name = "test", command = "python3 -m unittest discover -s tests -q" }`.

Re-checked 2026-10-15 at `7b6a5af44` (roko-init, `tmp/hermes-roko-v5/roko-init/evidence/before-*`): fresh
`roko init` in a stdlib Python project, a Node project with a `test` script, an empty directory and a Cargo crate
writes `rungs = []` in all four. Four project detectors exist: `roko_core::project::detect_from_files` (I/O-free,
used nowhere outside roko-core), `roko_gate::BuildSystem::detect`, `repo_context::ProjectKind::detect` (plan
generation) and init's own `detect_project_domain`, which only printed a hint. None recognises a Python project
with no packaging file.

## Plan

1. Add a `python` profile. Its test rung is `python3 -m pytest -q` when pytest is importable, else
   `python3 -m unittest discover -s tests -q` (or `-s .` when there is no `tests/`). Add lint and typecheck rungs
   only for tools that are installed.
2. Detect Python from `*.py` files plus a `tests/` directory as well as from the packaging files.
3. When no `--profile` is given, write the detected domain's rungs, or ask on a terminal.
4. Make the hint list the profiles that exist.

## Done when

`[[verify]]` passes: `roko init` in such a project writes a test rung, and that rung passes on the unchanged
project.

## Notes

Keep `roko init` idempotent. Don't overwrite rungs a person has set.
