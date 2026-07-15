# CTRL-01 ignored canonical import independent review

Verdict: **ACCEPTED**

Reviewed candidate: `4887c2e6177c7a40749c185b01edbb0666882c81`

Implementation commit: `699df4e0ea34bddabc4516695d28d1bf41328774`

Candidate base: `98a238aed98549a6a0e43077124cc7146a815799`

Review branch: `review/CTRL-01-4887c2e6177c`

## Scope and method

I read the complete current `MASTER-EXECUTION-CHECKLIST.md`, the candidate evidence, both candidate commits, the complete candidate diff, the sealed-root sources, the external recovery archive, the imported files, and the historical plan index. All sealed-root access was read-only. I did not edit the original checkout, implementation, master, manifests, indexes, or shared evidence.

`git diff --name-only 98a238aed98549a6a0e43077124cc7146a815799..4887c2e6177c7a40749c185b01edbb0666882c81` contains exactly 37 paths:

- one `.gitignore` change;
- one `.roko/GAPS.md` import;
- 34 files below `plans/` (32 TOML manifests, `INDEX.md`, and `_meta/IMPLEMENTATION_ORDER.md`);
- one candidate implementation-evidence file.

There are no unexpected cumulative paths. The implementation commit accounts for the first 36 paths; the candidate evidence commit adds only its evidence file. `git diff --check` passes. No master, backlog, status ledger, or review record changed in the candidate.

## Archive classification

The archive is:

```text
/Users/will/.local/state/roko/status-quo-20260714T073140Z/ignored-canonical-control-plane.tar.gz
sha256 01c10b4565c1a897c92ced109c7f351fcb35513816860d094efa446da62c34e0
```

I classified every tar member independently with Python `tarfile`, using member type and basename rather than accepting the implementation evidence's totals:

```text
all members:       144
directory entries: 33
regular files:    111
  intended files:  39
  AppleDouble:      72  (regular-file paths whose basename begins `._`)
other types:         0
```

The 39 intended regular files are exactly 33 top-level `plans/` files, root `.roko/GAPS.md`, and five `.claude/worktrees/*/plans/architecture-core-queue/tasks.toml` source copies. No intended regular file is a `.DS_Store`, log, or runtime artifact. The 72 AppleDouble companion entries were not materialized in the candidate.

## Exact sealed-root, archive, and import identity

For every row below, the sealed-root file, the archive member, and the candidate file have exactly the stated SHA-256 and byte length. The sealed-root set was enumerated independently as every regular file below root `plans/` except `.DS_Store` and `._*`; it contains exactly 33 files.

| SHA-256 | Bytes | Path |
|---|---:|---|
| `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44` | 2464 | `plans/INDEX.md` |
| `e2406f0dbbf1ecc436d7c2de32faabdc0419a0e326b32d3a7efaf6ead2689991` | 15997 | `plans/P08-search-command-fix/tasks.toml` |
| `d3ded0c373224b458920122e924463557e7b0c4f795593808d43b3559fb489a7` | 11461 | `plans/P09-tool-alias-fix/tasks.toml` |
| `dde57f869621112be98272d97ea90a5df49211eb8be2fab009b1ee192b34aeda` | 15545 | `plans/P10-slash-command-flags/tasks.toml` |
| `6ad52d1c4ff1772e34b25cac60023e7ef2ae9ccaa94b2202b90d5b06be61fb2b` | 16000 | `plans/P11-runner-v2-default/tasks.toml` |
| `91cce19e4df3a2f18194f49c6f64a9680158f7f8dcc4c0331e430729181a7cb4` | 24032 | `plans/P12-runner-parallelism/tasks.toml` |
| `a7c48b54700d40e21c62c8aff7f4c25e832392ea14d6b490c2b385c076ebf498` | 13963 | `plans/P13-rate-limit-retry/tasks.toml` |
| `c33120eabd9d607118ea8d0131b47f642cfb813bc5cb497cd2f02c422ca073bd` | 11634 | `plans/P14-gate-rung-fix/tasks.toml` |
| `11b5c2a069824bf2b295622fa57f2d3cefe8cc4aaddc6282d412f530e21488c5` | 13738 | `plans/P15-error-recovery-wiring/tasks.toml` |
| `fc63e6addac2631d909d5f7c371b1f614c93c79f61e12bc48a5390b1974c7ce2` | 16908 | `plans/P16-safety-contracts/tasks.toml` |
| `b9c72fe56f8c1dbfad1c439050ff02c6052f3b086c5bc8db0ed7d8c80760788d` | 18891 | `plans/P17-cli-output-format/tasks.toml` |
| `10abf24b90cd30f015989c07eb5ce364a34ab6c3dfb1540aebe3596ce3f0efe3` | 13391 | `plans/P18-tui-agent-data/tasks.toml` |
| `29f202968a6566fcf55824d5ed7aaf275ac71a18043f01aa8c5183d165fa8892` | 18564 | `plans/P19-cascade-router-acp/tasks.toml` |
| `40220c8c2150f74af0f95130ab925675e67dc4a37d011d9896767ca00ff57353` | 16174 | `plans/P20-zero-config/tasks.toml` |
| `bd4ce0e2cf09775b75ddebd03faa3d0aa8e987f4bb25a5aaab0bb9dd7d9b4499` | 12969 | `plans/P21-acp-streaming/tasks.toml` |
| `9ea7c6b3a1cc77b094ad2e1ca05ae1c18b488619920aa14a49ed4035ebaaea1a` | 18096 | `plans/P22-acp-tool-permission/tasks.toml` |
| `5b465603f8115b1b10a7a28508ae7a227147558b735162cf26dedff16b9886cc` | 17037 | `plans/P23-prd-pipeline-fix/tasks.toml` |
| `7c8e1d3e12412aa627f066b38888d2d4e967d84ab73e8c12f0a3092e13b3e264` | 11314 | `plans/P24-workspace-paths/tasks.toml` |
| `821a0a3dc72405aef894e1c12617a885aea8a3e7d73dced58b4b1af0d946f0a5` | 9356 | `plans/P25-mcp-acp-passthrough/tasks.toml` |
| `e8da1b35475ed046a9708feb641b4f189f632186fdf0a7ec2e6b59582bbbc321` | 10772 | `plans/P26-hdc-similarity-lookup/tasks.toml` |
| `1f1642509f1558b5385ddbb40c5d8321d622d950ff4cf100cdedf670be1c47f9` | 9502 | `plans/P27-provider-error-ux/tasks.toml` |
| `15baf1d1a198cabe00d681ddef950523d5ff718377cc2d0e21bda57fc25b8dc6` | 13536 | `plans/P28-image-support/tasks.toml` |
| `b3bfad2623e5e967ed56076a337656754c75756b1a6cc12c50ed5d12f613fa4d` | 8574 | `plans/P29-develop-command-wire/tasks.toml` |
| `a17f117760c9788c4d846f93aa8b0926329095de8cb813171069a0aeaa5a2b50` | 10244 | `plans/P30-onboarding-doctor/tasks.toml` |
| `1ee8642b997e5f6a9dc883e54e33774d32f374b6920bed3c457585282c3d32c6` | 10786 | `plans/P31-note-and-context/tasks.toml` |
| `4193705d584fb63efe0840e1babe2fe0dbfead88995507e5f70eb15605127ba0` | 6693 | `plans/P32-cli-polish/tasks.toml` |
| `41c821d993933bb22d2c081e7f3fb1c255726926d2d013cdb1eff99b0cce337f` | 5048 | `plans/P33-model-ux/tasks.toml` |
| `b4c4be282fd1ece7edabd190e65fc4e92b0ec60abd89206aecadcca46ec761ee` | 6659 | `plans/P34-verification-sweep/tasks.toml` |
| `2f7d8da201fa8f704de24e4047f0eb2f98f12e36744f79b567ad2e2aaf01d7a8` | 2160 | `plans/_meta/IMPLEMENTATION_ORDER.md` |
| `528e8d88e780ec39f0239cb8977cd0834058a0f4f75a6ba48a7e20e5c857875e` | 6043 | `plans/architecture-defi-critical-path/tasks.toml` |
| `054f834cd0180de520dfe333547be14dd571db100355cf9c48027efbc2451c4d` | 3731 | `plans/e2e-smoke/tasks.toml` |
| `5127fbd57552b49865718f718a0a154656bdfc300ba04f481868c8a272b2acb3` | 19478 | `plans/self-dev-extras/tasks.toml` |
| `280f79acf6a8841099ecd9790e474e7044c4df4ed4fe23e80f1a3cf83b8a56fe` | 81451 | `plans/self-dev-ux/tasks.toml` |
| `6faed7b798c4a3d3f200d5f71a933fb419e4000057e803b7d0b8132197e45f73` | 2308 | `.roko/GAPS.md` |

The independently reproduced aggregate for those 34 sorted `SHA-256 + two spaces + path` records, joined with newline and terminated by newline, is:

```text
be1115dde664b5b46429ba77f97e084cbbae3e55134245ffc01b528ac4545b23
```

## Architecture source identity

All five sealed-root files, their corresponding archive members, and the recovered canonical destination are byte-identical:

| SHA-256 | Bytes | Path |
|---|---:|---|
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `.claude/worktrees/agent-a9a18acb/plans/architecture-core-queue/tasks.toml` |
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `.claude/worktrees/agent-aad01731/plans/architecture-core-queue/tasks.toml` |
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `.claude/worktrees/agent-ab986004/plans/architecture-core-queue/tasks.toml` |
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `.claude/worktrees/agent-adbd1807/plans/architecture-core-queue/tasks.toml` |
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `.claude/worktrees/agent-aefd7c48/plans/architecture-core-queue/tasks.toml` |
| `3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5` | 71760 | `plans/architecture-core-queue/tasks.toml` (recovered) |

The same hash is independently reproduced from `7899494d3^:plans/architecture-core-queue/tasks.toml`, the last tracked pre-removal source. No `.claude` path was imported.

## Manifest and status verification

Python 3 `tomllib` parsed all 32 candidate manifests. Assertions passed for unique `[meta].plan`, `[meta].total == len([[task]])` in every file, and a 210-task corpus. The observed source-preserved state is 30 `ready` and two `superseded` plan statuses; all 210 task records are `ready`.

No canonical task or plan status was changed by the candidate. This follows from exact byte identity for all 31 sealed-root manifests and from exact byte identity between the recovered architecture manifest and all five sealed sources (as well as the historical Git source). The candidate performs imports only; it does not rewrite any status field.

## Historical index caveat and validator

I parsed `plans/INDEX.md` independently and cross-checked each table row against its manifest. It accurately lists 29 executable plans with 120 tasks and two superseded plans with 66 tasks. Its listed manifest set is the complete 31-file sealed-root set and excludes only the separately recovered `architecture-core-queue` manifest, which is `ready` with 24 tasks. Therefore the current corpus is 30 executable plans/144 executable tasks plus two superseded plans/66 excluded tasks, exactly as the candidate evidence explains.

I ran the integrated `target/debug/roko` validator from a disposable repository root containing a copied `plans/` tree and symlinks to the candidate source roots:

```text
roko plan validate --strict plans --color never
0 diagnostics in 32 plans
exit 0
```

The disposable validator side effect regenerated its copied index with `30 plans, 144 tasks` and `2 plans, 66 tasks` excluded. The candidate source index remained unchanged before and after at `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`. The implementation evidence's historical-index warning and side-effect description are accurate.

## Ignore behavior and artifact hygiene

`git check-ignore -v --no-index` independently established:

```text
VISIBLE .roko/GAPS.md
VISIBLE plans/INDEX.md
VISIBLE plans/P34-verification-sweep/tasks.toml
VISIBLE plans/architecture-core-queue/tasks.toml
VISIBLE plans/new-canonical/tasks.toml

IGNORED .roko/INDEX.md
IGNORED .roko/VERSION
IGNORED .roko/runtime/agent-pids.json
IGNORED .roko/state/executor.json
IGNORED .roko/prd/ideas.md
IGNORED nested/workspace/.roko/GAPS.md
IGNORED plans/.DS_Store
```

The root re-inclusion exposes only `.roko/GAPS.md`; all other root `.roko` children remain ignored by `/.roko/*`, while nested workspace `.roko` directories remain ignored by `**/.roko/`. Removing `/plans/` exposes canonical top-level plans. `git ls-files .roko plans` contains only root `.roko/GAPS.md` plus the intended 34 plan files.

The cumulative candidate path set contains no `.DS_Store`, `._*`, `.claude`, log, logs directory, or runtime path. The candidate worktree was clean before this review record was written.

## Decision

**ACCEPTED.** The candidate is a byte-preserving, correctly scoped import of the concealed canonical queue. Archive composition, source authority, hashes, counts, TOML structure, unchanged statuses, historical index arithmetic, validator behavior, ignore precedence, and artifact exclusions all reproduce independently. No release or external action was performed.
