+++
id = "spec-ae5f94"
kind = "spec"
title = "Epic: release blockers"
status = "open"
triage = "unverified"
severity = "p0"
goal = "release"
size = "L"
subsystem = ["roko-gate/shell", "roko-serve/auth", "README", "Cargo.toml", ".github/workflows"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e16"
discovered_from = "tmp/cybernetic-harness/workstreams/PLAN.md (§2 goal order; §3 E16)"
anchors = ["crates/roko-gate/src/shell.rs::ShellGate", "crates/roko-serve/src/routes/auth.rs::AuthRegistry", "README.md", "Cargo.toml", ".github/workflows/ci.yml"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["bug-7d7200", "bug-da5b41", "bug-09690f", "find-8cc7ac", "bug-911361", "gap-452185", "bug-e1327f", "bug-131421", "bug-8465a2", "bug-1c93b4", "bug-5c25e1", "bug-12153c", "bug-39d54c", "bug-8d7d18", "bug-367f33", "bug-647249", "bug-4e7d40", "bug-ab8118", "bug-524a3b", "bug-8f8704", "dec-648cce", "gap-ed511d"], blocks = [], related = ["spec-ba7bea", "spec-9a3131", "bug-7eef96"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'env_clear()' crates/roko-gate/src/shell.rs && grep -rqw 'fn out_of_band_api_key_survives_server_write' crates/roko-serve/ && ! grep -q '124/124' README.md && grep -q '^repository = \"https://github.com/wpank/roko\"' Cargo.toml && test \"$(git grep -l /Users/will -- . ':!work/' | wc -l)\" -eq 0"

[[verify]]
command = "sha=\"$(gh api repos/{owner}/{repo}/commits/main --jq .sha)\" && test \"$(gh run list --commit \"$sha\" --json conclusion --jq length)\" -gt 0 && test \"$(gh run list --commit \"$sha\" --json conclusion --jq '[.[]|select(.conclusion==\"failure\" or .conclusion==\"startup_failure\" or .conclusion==\"timed_out\")]|length')\" = 0 && test \"$(gh api repos/{owner}/{repo}/branches/main/protection/required_status_checks --jq '.contexts|length')\" -gt 0"
+++

## Problem

Six defects block a public release:

- agents and verify commands inherit the provider keys (bug-7d7200);
- a running `roko serve` erases API keys that the CLI created (bug-da5b41);
- the README's quick start fails, and the README claims 100% completion (bug-09690f);
- CI has never passed, and `main` has no required checks (find-8cc7ac);
- `Cargo.toml` points at an unrelated GitHub account (bug-911361);
- 52 tracked files, `CLAUDE.md` among them, contain local paths under `/Users/will` (gap-452185).

## Why it matters

`release` is the first goal in the plan's order (`PLAN.md` §2), and it holds blockers only. The licence is in place
(`0e94fa8ef`). A public repository with key leaks, a broken quick start or a red CI undercuts the whitepaper it
accompanies.

## Where

- `crates/roko-gate/src/shell.rs::ShellGate` and the agent child environment (bug-7d7200).
- `crates/roko-serve/src/routes/auth.rs::AuthRegistry` (bug-da5b41).
- `README.md`, `Cargo.toml`, `.github/workflows/` and every tracked file that holds `/Users/will`.

## Current state

Checked at `41c7ffbd6`:
- **All six are open.** The static parts of their verify commands all still fail; none looks already fixed.
- **bug-7d7200 is in flight.** Its fix is uncommitted in `../roko-wt-env`, on branch `fix/hermetic-child-env`, which
  has no commits yet.
- **gap-452185:** `git grep -l /Users/will` still finds 52 files outside `work/`.
- **Holds:**
  - bug-7eef96 (Privy JWT) is on hold at the author's request since 2026-09-28 and stays out of this epic.
  - `PLAN.md` §2 proposes `hold = "after the golden path"` for every other `release` item; that has not been applied.

## Plan

This is the implementation plan.

1. **bug-7d7200:** the portal session lands it. It is also E3.1.
2. **In parallel, cold:**
   - bug-911361, mechanical;
   - gap-452185, mechanical;
   - bug-09690f, in the docs lane.
3. **bug-da5b41** edits serve's auth routes. Serve routes are hot, so take one writer.
4. **find-8cc7ac last:** fix the failing workflows once the rest has merged. The author then sets the required checks
   on `main`.
5. **Exit check:** the static checks pass, and CI on `main` is green with required checks set.

## Done when

- [x] bug-7d7200: Agents and verify commands inherit roko's whole environment, including provider API keys (existing item)
- [x] bug-da5b41: A running roko serve erases API keys created by the CLI (existing item)
- [x] bug-09690f: README's quick start fails, and the README claims 100% completion (existing item)
- [ ] find-8cc7ac: Some GitHub workflows fail on main and required checks are undefined (existing item)
- [ ] bug-911361: Cargo.toml's repository and homepage point at an unrelated GitHub account (existing item)
- [ ] gap-452185: 52 tracked files contain local absolute paths under /Users/will, including CLAUDE.md (existing item)
- [x] bug-e1327f: roko init without claude on PATH writes a roko.toml that fails every command with config invariant 3
- [x] bug-131421: roko setup --quick writes no roko.toml in a fresh directory, and after init writes a key that validation rejects
- [x] bug-8465a2: roko config validate passes a budget table that the core config loader rejects
- [x] bug-1c93b4: roko config set --project writes a legacy agent.model key that validation rejects, and refuses v2 keys
- [x] bug-5c25e1: roko run in a fresh workspace fails budget admission against a $1 turn cap although roko.toml sets max_turn_usd = 0
- [x] bug-12153c: The config loader silently drops agent.fallback_model, agent.tier_models, serve.port and project.default_domain from roko.toml
- [x] bug-39d54c: roko serve rewrites agent-tokens.json and relay-tokens.json whole from memory, losing other processes' tokens and revocations
- [x] bug-8d7d18: Several roko.toml writers skip the check-before-write: config preset, tune and the TUI config and effects saves
- [x] bug-367f33: roko --config <file> fills missing [budget] keys with the CLI's legacy defaults ($10 per plan, $1 per task) instead of core [budget]'s
- [x] bug-647249: About 100 more roko.toml keys are missing from the loader's schema tree, so loading strips them
- [x] bug-4e7d40: roko config preset --global edits ~/.roko/roko.toml instead of ~/.roko/config.toml, and fails unless that file exists
- [ ] bug-ab8118: A typo inside a [providers.*] or [models.*] entry fails the whole config load, where a typo elsewhere is stripped with a warning
- [ ] bug-524a3b: serve's hints tell users to put serve.auth.api_key in roko.toml; point them, and config set, at ROKO__SERVE__AUTH__API_KEY in .roko/.env
- [ ] bug-8f8704: ${VAR} is expanded only in provider fields, so serve.auth.api_key = "${X}" loads as a literal key
- [ ] dec-648cce: Should roko deploy railway forward ROKO__SERVE__AUTH__API_KEY to the services it deploys?
- [ ] gap-ed511d: docs/v3's auth pages, docker/RAILWAY.md and config set --help still put secrets in roko.toml, which roko now refuses
- [ ] Both of the epic's `[[verify]]` commands pass.

## Notes

- **Shared children:** bug-7d7200 is also in E3 (spec-ba7bea), and find-8cc7ac is also in E15 (spec-9a3131).
- **`CLAUDE.md` is edited twice,** by gap-452185 and by gap-cdf3fc (E15). Merge them one after the other.
- **Needs the author:** branch protection on `main`, and lifting the hold on bug-7eef96.
- **Still open (not accepted on 2026-09-29):** branch protection on `main`. bug-7d7200 closed in 70820a74c.
