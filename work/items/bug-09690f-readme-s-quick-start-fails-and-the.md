+++
id = "bug-09690f"
kind = "bug"
title = "README's quick start fails, and the README claims 100% completion"
status = "open"
triage = "verified"
severity = "p1"
size = "M"
goal = "release"
subsystem = ["docs/readme"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/publication-readiness.md"
anchors = ["README.md:7", "README.md:9", "README.md:12", "README.md:68", "README.md:108", "README.md:111", "README.md:596", "README.md:606", ".github/workflows/docs-lint.yml:77"]
links = { depends_on = [], blocks = [], related = ["bug-f279ea", "gap-ae2f55"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q -- \"--engine runner-v2\" README.md && ! grep -q \"48 epics\" README.md && ! grep -q \"124/124\" README.md && ! grep -q \"tmp/status-quo\" README.md"
+++

## Problem

The public `README.md` gives instructions that fail and makes claims the work graph contradicts:

- The "Full planning pipeline" steps 5 and 6 (`README.md:108`, `:111`) and the CLI quick-reference row (`:606`) say
  `roko plan run plans/ --engine runner-v2`. Runner-v2 was deleted on 2026-09-06. `--engine runner-v2` is still
  parsed (`PlanEngine::RunnerV2` in `crates/roko-cli/src/main.rs:1949`), but `crates/roko-cli/src/commands/plan.rs:640-654`
  logs "the legacy Runner-v2 engine has been removed" and returns `EXIT_FAILURE`. Expected: `roko plan run plans/`
  (Graph is the only engine).
- `README.md:9-60` is an internal programme roll-up: "all 48 epics are accepted", "124/124 tasks complete (100%)",
  and "the exact 109/746/637 status, source-registry, and manifest contracts are checked". The last one is false:
  `tmp/status-quo/` no longer exists. The open items in `work/` contradict the completion claims.
- `README.md:12` links to `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, which is not in the repo. `:11` points at
  the frozen `.roko/GAPS.md` as the "source of truth" (open work now lives in `work/`).
- Stale counts: `:7` and `:644` say "35 workspace members. ~800K lines of Rust. 9,900+ tests". CLAUDE.md says 36
  members, ~1M lines, 10,300+ tests (the assessment measured 36 members and 1,081,075 tracked `.rs` lines).
- `:596-597` link into tracked `tmp/dev-audit/README.md` and `tmp/tui-parity/00-INDEX.md`. These break if `tmp/` is
  untracked (open decision D-03).
- The roll-up leads with arena, DeFi, x402 and marketplace content that neither the companion report nor the Nous
  positioning uses.
- The Quick start (`:68-76`) runs `roko init my-project && cd my-project && roko run "…"`. In that empty directory
  there is no `Cargo.toml` or `go.mod`. Unless `roko init` writes gate rungs, `prompt_verify_steps`
  (`crates/roko-cli/src/run.rs:513`) returns no verify step, and the one-task plan is rejected before dispatch
  (`bug-1410e8`). Not run here; confirm it.

## Why it matters

Goal `release`: the README is the first thing a Nous reviewer or a new user reads. A pipeline that fails at step 5,
and "100% complete" claims, undermine the companion report, whose thesis is that claimed, wired and effective
behaviour differ. Reviewers would find the report's own repo as the counterexample.
Related: `bug-f279ea` (the Docs Lint rule that forces `--engine runner-v2`; it must change in the same PR),
`bug-1410e8` (`roko run` in a workspace with no build manifest), `find-8cc7ac` (red CI), `gap-ae2f55` (licence
files, done).

## Where

- `README.md`: the only file to rewrite. Entry point: GitHub's repository page.
- `.github/workflows/docs-lint.yml:77-80`: the `bare_plan` grep fails any line in README/CLAUDE.md/docs/v2 that ends
  in `roko plan run plans…` without flags. `bug-f279ea` removes it. Its other patterns (`roko neuro`, `--listen`,
  `18 crates`, `F1.?F7`, `~85 routes`, `/healthz`, `/readyz`, an unqualified `executor.json`) still apply, so do
  not use those phrases in the new text.
- `tools/docs_integrity/check_markdown_links.py`: checks local links and anchors in README. Every link in the new
  README must resolve in a clean clone.
- `crates/roko-cli/src/main.rs` (`Command::Init`, `PlanEngine`) and `crates/roko-cli/src/run.rs::prompt_verify_steps`:
  the behaviour the quick start must match.

## Current state

- `README.md` (651 lines) was last changed in `72e0a76b8`. None of the problems above are fixed.
- The audited README is preserved at the local tag `audit/baseline-2026-09-28` (`91b4745f8`), which the companion
  report uses as its specimen. So the rewrite can go ahead now.
- `LICENSE-MIT` and `LICENSE-APACHE` exist (`0e94fa8ef`), so the README's `## License` line ("MIT OR Apache-2.0")
  is now correct.
- `roko init [path]` creates the directory (`commands/util.rs::cmd_init` calls `create_dir_all`), `.roko/` and
  `roko.toml`.

## Plan

1. Rewrite the top of the README: one paragraph on what roko is, then correct counts (36 workspace members,
   ~1M lines of Rust, 10,300+ tests) or no counts. Delete lines 9-60 (the roll-up). Replace them with a short
   "Status" paragraph that points at `work/STATUS.md` and `work/NOW.md`. Make no completion percentages.
2. Make the Quick start something that works in a fresh directory:
   - install: `cargo install --path crates/roko-cli`;
   - provider setup: `roko setup`, or point at `roko config providers list`;
   - `roko init`, then either a `roko run` inside an existing Rust or Go project, or a Rust example
     (`cargo new hello && cd hello && roko init && roko run "…"`), until `bug-1410e8` is fixed;
   - one plan run (`roko plan run plans/<dir>`), and `roko serve`.
   Run each command in a scratch directory before writing it down, and record the output in the PR.
3. Replace `--engine runner-v2` on lines 108, 111 and 606 with `roko plan run plans/` and
   `roko plan run plans/ --resume-plan`. Describe the row as "Execute a plan directory through the Graph engine".
4. Remove the links into `tmp/` (lines 12, 596-597) and the pointer to `.roko/GAPS.md` as the source of truth.
   Link tracked docs instead, or drop the sentences.
5. Cut or shorten the arena/DeFi/x402/marketplace material. If it stays, move it to a short "Optional chain
   primitives" section under Architecture, with no maturity claims (match the CLAUDE.md component table).
6. In the same PR, land `bug-f279ea` or at least drop the `bare_plan` rule in `docs-lint.yml`. Otherwise Docs Lint
   rejects the corrected README.
7. Check that every remaining link resolves: `python3 tools/docs_integrity/check_markdown_links.py` (what Docs Lint
   runs), and the grep rules in `docs-lint.yml`.

## Done when

- No `--engine runner-v2`, "48 epics", "124/124" or `tmp/status-quo` text remains, and no link targets `tmp/`.
- The counts match CLAUDE.md, or are gone.
- Every quick-start command has been run in a clean scratch directory and does what the README says.
- Docs Lint passes on the new README (after `bug-f279ea`).
- Verify: `! grep -q -- "--engine runner-v2" README.md && ! grep -q "48 epics" README.md && ! grep -q "124/124" README.md && ! grep -q "tmp/status-quo" README.md`

## Notes

- Decided (assessment 2026-09-28): drop the runner-v2 lint rule in the same PR as the README fix. Keep the
  audited text at the `audit/baseline-2026-09-28` tag. Do not edit or move that tag.
- Do not touch `.gitignore`, and do not untrack `tmp/` files here: that is decision D-03. Just stop linking to
  them.
- `docs/v2/CLI-REFERENCE.md` and the other `docs/v2` examples are covered by `bug-f279ea`, not by this item.
- Docs only, with no Rust changes. Safe in parallel with code work. It conflicts only with other README edits
  and with `bug-f279ea` (edit `docs-lint.yml` in one place).
- 2026-09-29 (wk-readme), how the new quick start was checked. Binary: `target/debug/roko` built at
  `33e107da1`. Scratch directories under `/private/tmp`, an empty `HOME`, no API keys, no `cargo` on
  `PATH`, and the fake agent `plans/portal-programme/_harness/fake-claude` standing in for `claude`.
  - `roko init` wrote `roko.toml` and `.roko/`, and `roko config providers list` showed
    `claude_cli ... ok (cli found)`.
  - Empty workspace: the one-task prompt path stopped before dispatch with `no gate can verify this change`,
    which confirms `bug-1410e8`. Cargo workspace: `roko run` dispatched the agent and ran
    `cargo check --workspace` as its verify step.
  - `git init && roko init`, plus a copy of `plans/demos/parallel-plans/demo-hello-world`: `roko plan run`
    reached `status: succeeded`, and its `rustc` verify step passed. `plan status`, `--resume-plan`,
    `--dry-run` and `plan validate` also worked.
  - `roko serve --port 16677`: `/health` and `/ready` returned 200, and `/api/plans` returned 401 without a
    token.
  - `cargo install` was checked statically: `[[bin]] roko`, rust-version 1.91.
  - The README's minimal config passes `roko config validate` and the core loader.
- Found while checking; not fixed here (for the coordinator to file):
  - `roko init` without `claude` on `PATH` leaves `[models.claude-sonnet-4-6]` pointing at the commented-out
    `claude_cli` provider. Every config-loading command then fails with `config invariant 3 violated`, even
    with `ANTHROPIC_API_KEY` exported, which is what init itself advises.
  - `roko setup --quick` in an empty directory skips init because `.roko/` already exists (roko's own log
    creates it). It then prints `roko.toml already contains all detected providers` and writes no
    `roko.toml`. After `roko init`, it writes `providers.anthropic.default_model`, a key that
    `roko config providers validate` rejects.
  - The core loader rejects `[budget] max_plan_usd = 10, max_task_usd = 1` without `max_turn_usd`
    (`max_turn_usd (0) must not exceed max_plan_usd (10)`), while `roko config validate` passes the same
    file. The old README's budget example hit this.
  - `roko config set --project agent.default_model X` rewrites `roko.toml` and adds a legacy `agent.model`
    key that `roko config validate` then rejects. It also refuses v2 keys such as `budget.max_plan_usd`.
  - In a fresh `roko init` workspace, `roko run --max-retries 0 "..."` fails budget admission with
    `predicted turn cost $1.5000 exceeds max_turn_usd $1.0000`, although `roko.toml` sets
    `max_turn_usd = 0.0`.
  - `roko prd plan` and the `roko setup` provider advice were not run: both need a live provider.

## Original notes

- The public README tells users to run `roko plan run plans/ --engine runner-v2` (`README.md:108`, `:111`, `:606`). That engine was deleted, and the flag exits with an error.
- Its opening (`README.md:9-60`) is an internal roll-up claiming "all 48 epics are accepted" and "124/124 tasks complete (100%)", which the work graph and the companion audit contradict.
- `README.md:12` links to a `tmp/status-quo` file that is not in the repo.
- The assessment also found that it leads with arena, DeFi and x402 content that neither the paper nor the Nous positioning uses.

Fix: rewrite the README around what works today (install, `roko init`, one plan run, serve), move status claims to `work/`, and fix the docs-lint rule that enforces runner-v2 (bug-f279ea).
