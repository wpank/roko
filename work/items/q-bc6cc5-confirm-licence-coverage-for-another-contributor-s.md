+++
id = "q-bc6cc5"
kind = "question"
title = "Confirm licence coverage for another contributor's code in apps/mirage-rs"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
goal = "release"
subsystem = ["release/licensing"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "work:gap-ae2f55"
anchors = ["apps/mirage-rs/src/precompiles/hdc.rs", "apps/mirage-rs/src/precompiles/mod.rs", "apps/mirage-rs/src/rpc.rs", "apps/mirage-rs/src/provider.rs", "apps/mirage-rs/src/fork.rs", "LICENSE-MIT"]
links = { depends_on = [], blocks = [], related = ["gap-ae2f55"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '^## Outcome' work/items/q-bc6cc5-confirm-licence-coverage-for-another-contributor-s.md"
+++

## Problem

roko is licensed MIT OR Apache-2.0, with "Copyright (c) 2026 Will Pankiewicz" in `LICENSE-MIT` (added in
`0e94fa8ef`, closing `gap-ae2f55`). Some committed code was written by someone else, and nothing records that
this contributor's code may be released under those terms.

Surviving lines at HEAD attributed to that contributor (git author `JaeLeex`, 15 commits, 2026-04-16 to 04-23), from
`git blame --line-porcelain HEAD -- <file> | grep -c '^author JaeLeex$'` on 2026-09-29:

| File | Their lines / total |
|---|---|
| `apps/mirage-rs/src/precompiles/hdc.rs` | 839 / 842 |
| `apps/mirage-rs/src/rpc.rs` | 336 / 6116 |
| `apps/mirage-rs/src/provider.rs` | 105 / 1063 |
| `apps/mirage-rs/src/fork.rs` | 47 / 3120 |
| `apps/mirage-rs/src/precompiles/mod.rs` | 24 / 24 |
| `apps/mirage-rs/src/persist.rs` | 9 / 536 |
| `crates/roko-demo/src/bindings.rs` | 8 / 142 |
| `apps/mirage-rs/src/lib.rs` | 3 / 535 |
| `crates/roko-serve/src/routes/deployments.rs`, `crates/roko-serve/src/templates.rs` | 1 each |

That is about 1,370 lines, 1,363 of them in `apps/mirage-rs`. The item was filed with 1,357 (320 in `rpc.rs`). The
small difference probably comes from later edits or blame options. Other files that contributor touched
(`Dockerfile.worker`, `DEPLOY-RAILWAY.md`, `publish-worker-image.yml`) no longer exist. None of the 5 lines from a
third author (`simp-son`) survive.

This is a question for Will. Code cannot settle it.

## Why it matters

Goal `release`: the repository is public, and a licence only covers code whose rights the licensor holds or has
permission for. Without a recorded answer, 1.3k published lines have unclear terms, and a careful reviewer or
downstream user could flag it.
Related: `gap-ae2f55` (licence files, done; its closure points here).

## Where

- `LICENSE-MIT`, `LICENSE-APACHE` and `Cargo.toml:104` (`license = "MIT OR Apache-2.0"`, inherited by every member
  through `license.workspace = true`, including `apps/mirage-rs/Cargo.toml:6`).
- `apps/mirage-rs/` is a workspace member (`Cargo.toml:37`): the in-process EVM fork simulator. `precompiles/hdc.rs`
  adds `hdc::HDCPrecompiles`, an HDC precompile wrapped around `revm::EthPrecompiles` and wired into `fork.rs`. The
  `rpc.rs` and `provider.rs` lines are `eth_getBlock*`/`eth_getLogs` fixes for an indexer (PRs #37-#47, 2026-04-22/23).
- Nothing else in the workspace depends on `mirage-rs` as a crate. `crates/roko-chain` only talks to it over RPC.

## Current state

- Facts that may matter for the "already covered" option: the contributor committed with a `nunchi.trade` email
  address, and the work came in through pull requests on the `Nunchi-trade` GitHub organisation (for example
  `Merge pull request #43 from Nunchi-trade/fix/mirage-rs-indexer-unblock`). Whether an employment or contractor
  agreement assigns that work to Nunchi or to Will is unknown.
- Will decided the licence and the copyright holder on 2026-09-28. He has said he owns the bardo/Mori-derived
  code; that statement does not cover this contributor's code.
- No outcome has been recorded.

## Plan

1. Ask Will which of these applies, and record his answer in this item:
   - A. The contributor agrees in writing to MIT OR Apache-2.0 for their contributions. Keep a pointer to the
     written agreement (email, issue comment or signed note).
   - B. The work is already covered: work for hire, or assigned under an agreement with Nunchi or Will. Record
     who holds the copyright. If Nunchi holds it, decide whether `LICENSE-MIT` should name Nunchi too, or say
     "Will Pankiewicz and roko contributors".
   - C. Carve out: exclude `apps/mirage-rs` (or only `precompiles/`) from the licence, with a `LICENSE`/`NOTICE` in
     that directory, or remove it from the public repository.
   - D. Rewrite: delete `precompiles/hdc.rs` and `precompiles/mod.rs` (the HDC precompile is optional chain
     tooling), switch the three `with_precompiles(crate::precompiles::hdc::HDCPrecompiles::new(..))` call sites in
     `fork.rs` (about lines 1646, 2006 and 2071) back to plain `revm` precompiles, and reimplement the
     `rpc.rs`/`provider.rs`/`fork.rs` changes.
   A or B is the cheapest, if either is true. D costs the most, and a rewrite by someone who has read the original
   code may not remove the question. Ask counsel if that matters.
2. Add an `## Outcome` section to this item with the choice, the date and the evidence pointer. Close it with
   `python3 tools/work.py close` and put the same text in `[closed].evidence`.
3. If C or D is chosen, file a new work item for the code or packaging change and link it here. Do not make that
   change under this item.

## Done when

- This item has an `## Outcome` section naming A, B, C or D, with who decided and when, and the evidence is in
  `[closed].evidence`.
- For C or D, a follow-up item exists and is linked from `links.related`.
- Verify: `grep -q '^## Outcome' work/items/q-bc6cc5-confirm-licence-coverage-for-another-contributor-s.md`

## Notes

- Only Will can answer this. Agents must not contact the contributor or change licence files on their own.
- Do not rewrite git history to remove authorship. Never force-push.
- Do not publish the contributor's email address in public docs. The `nunchi.trade` domain is enough context.
- Needs no code change and touches no source, so it is safe in parallel with anything.

## Original notes

roko is now licensed MIT OR Apache-2.0 by Will Pankiewicz. `git blame` at HEAD (2026-09-28) attributes 1,357 surviving lines to another contributor, JaeLeex (commits from 2026-04-16 to 04-23). They are almost all in `apps/mirage-rs`: `precompiles/hdc.rs` (839), `rpc.rs` (320), `provider.rs` (105), `fork.rs` (47) and a few others, plus single lines in roko-serve and roko-demo. None of simp-son's 5 lines survive.

Settle one of:
- the contributor agrees to the licence;
- the code was work for hire or otherwise already covered;
- carve out `apps/mirage-rs`, or rewrite those lines.

Record the outcome here.
