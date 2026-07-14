# CTRL-02 final precursor census review

## Verdict and immutable scope

- **Verdict:** `READY_FOR_FINALIZATION`
- **Review base:** `35b1eb45520f095846f96cfc5f3c9f79b02bbd57`
- **Historical precursor:**
  `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- **Exact precursor parent:**
  `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- **Historical range:** one commit, 31 paths, 3,177 insertions and 424
  deletions
- **Review branch/worktree:** `review/CTRL-02-final-census` /
  `reviews/CTRL-02-final-census`
- **Authorized write scope:** this evidence file only

The 31-path implementation/control-document census is complete and has no
unattributed precursor hunk. Eight bounded precursor clusters and canonical
`SH01-T06C4` have accepted integrated proof. The only two unintegrated
implementation clusters are config-dispatch and worktree reattachment, exactly
as the coordinator expected. During review, the sealed original checkout was
found to contain a programme-created, byte-divergent untracked review draft.
The coordinator preserved and attributed that breach, committed its recovery
record, removed only the superseded duplicate, and restored byte-exact equality
with the saved status. An independent recheck confirmed that resolution before
this verdict. The exact pending implementation list is therefore limited to
config-dispatch and worktree reattachment.

I read the complete master, CTRL-01 preservation/import evidence and reviews,
the saved recovery inventories, every current implementation/review record
named below, both rejected candidate records, the complete historical diff,
the current source/Cargo call paths, the current Git graph, and all ten control
documents in the historical commit. No production, manifest, master, status,
or source file was edited by this review.

## Population and classification

The path set independently derives as:

```text
31 total
19 Rust paths = 18 substantive + 1 formatting-only
2 Cargo metadata paths
10 historical control-document paths
```

This reconciles the apparently conflicting descriptions. The July audit's
“18 modified source files” excludes the formatting-only `agents_view.rs` and
does not count `Cargo.lock` or the orchestrator manifest. The Git commit has 31
paths because it also contains those two metadata files and ten historical
control documents.

Cluster abbreviations used below:

- `PROC`: process registration/reader termination;
- `OWN`: claim mutation ordering;
- `DL`: deadline/activity reconstruction;
- `GATE`: gate dispatch attribution;
- `C4`: canonical lost-effect/timeout task `SH01-T06C4`;
- `TUI`: authoritative bounded output and route capacity;
- `ATOM`: atomic write and JSONL recovery;
- `LOCK`: workspace advisory-lock diagnostic ownership;
- `HUB`: StateHub/SSE ordering and recovery;
- `CFG`: config-dispatch, pending;
- `WT`: worktree reattachment, pending;
- `FMT`: formatting-only; and
- `HIST`: dated baseline/control history, not current task completion.

## Exact substantive and Cargo path census

`Relation` compares the review base with the `3041d095d` blob. A changed blob is
accounted for by the accepted correction named in the cluster proof table.

| # | Path | Original `3041` hunk/outcome | Owner | Relation at review base |
|---:|---|---|---|---|
| 1 | `Cargo.lock` | `+1/-0`: add orchestrator `tracing` dependency resolution | `WT` | exact `3041` blob `c9fb112b713e`; pending |
| 2 | `crates/roko-cli/src/runner/agent_stream.rs` | `+11/-4`: add non-consuming child-finished probe, register PID immediately after spawn, and return when stdout delivery closes | `PROC` + `C4` | reviewed corrections/tests; blob `bacfbb42d72c` |
| 3 | `crates/roko-cli/src/runner/attempt_ownership.rs` | `+28/-13`: mutable resource probe for lost effects; reject missing resource before claim/nonce/cancellation mutation; share nonce transition | `OWN` + `C4` | reviewed correction; blob `af47742489fd` |
| 4 | `crates/roko-cli/src/runner/deadlines.rs` | `+104/-56`: saturating duration conversion, documented monotonic types, allocation-free stable earliest-owner expiry, free owner helper | `DL` | reviewed boundary regression; blob `1c04368da1c9` |
| 5 | `crates/roko-cli/src/runner/event_loop.rs` | `+336/-28`: runner config validation, worktree rediscovery, exact eligible activity refresh, post-activation gate logging, canonical plan-verify rung, lost-producer expiry, bounded sibling drain, timeout-ledger ordering/tests | `DL` + `GATE` + `C4` + `CFG` + `WT` | integrated owners preserved; blob `397533f94045`; only `CFG`/`WT` boundary work remains pending |
| 6 | `crates/roko-cli/src/runner/persist.rs` | `+54/-22`: use `roko-fs` atomic writer, add serialization context, atomically clear wholly invalid/partial JSONL and prove later append | `ATOM` | exact `3041` blob `f3446b89560d`; inherited behavior independently accepted |
| 7 | `crates/roko-cli/src/tui/app.rs` | `+9/-11`: replace three lossy snapshot read/replace sequences with selected-field `update_snapshot` mutations | `HUB` | reviewed topology regression; blob `612725d37722` |
| 8 | `crates/roko-cli/src/tui/state.rs` | `+90/-13`: treat connected output as an authoritative ring, cap at 50, rebuild stale task tails, add bounds/nonduplication tests | `TUI` | reviewed empty-ring correction; blob `6020f296e259` |
| 9 | `crates/roko-cli/src/tui/views/agents_view.rs` | `+1/-4`: rustfmt collapses one tuple expression; no semantic or test change | `FMT` | exact `3041` blob `63ab294cee7b` |
| 10 | `crates/roko-cli/src/tui/views/dashboard_view.rs` | `+47/-1`: subtract only table header from already-inner height; add capacity and rendered-route tests | `TUI` | reviewed minimum-buffer regression; blob `36b2ea4a8728` |
| 11 | `crates/roko-cli/src/workspace_lock.rs` | `+62/-10`: lock before truncate/write, sync owner diagnostic, clear before unlock, `must_use`, focused tests | `LOCK` | reviewed cross-process tests; blob `ecedf559dcc7` |
| 12 | `crates/roko-core/src/config/loader.rs` | `+178/-7`: normalize exact keys/unique slugs; reject duplicate/unresolved dispatch references; add initial tests | `CFG` | exact `3041` blob `09e329b32959`; r2 remains unreviewed |
| 13 | `crates/roko-core/src/config/mod.rs` | `+16/-0`: add typed ambiguous-slug and unresolved-model load errors | `CFG` | exact `3041` blob `eb1409d04ee7`; pending |
| 14 | `crates/roko-core/src/config/timeouts.rs` | `+5/-0`: assert hard-run/task/gate/silence/progress duration defaults | `DL` + `CFG` | exact reviewed blob `b6f66284330c`; `DL` owns the default proof, `CFG` consumes it unchanged |
| 15 | `crates/roko-fs/src/atomic.rs` | `+141/-35`: unique exclusive same-directory staging, staged-file sync, rename, Unix parent sync, collision/concurrency tests | `ATOM` | reviewed bare-relative parent correction; blob `50e2169a34c6` |
| 16 | `crates/roko-orchestrator/Cargo.toml` | `+1/-0`: declare `tracing` used by reattachment diagnostics | `WT` | exact `3041` blob `fb04eeaaf271`; pending |
| 17 | `crates/roko-orchestrator/src/worktree.rs` | `+239/-7`: rediscover/reattach plan worktrees, validate id/common-dir/branch/tip, derive timestamps, add real-Git tests/debug logging | `WT` | exact `3041` blob `e8402c5cd270`; rejected correction unmerged |
| 18 | `crates/roko-runtime/src/state_hub.rs` | `+87/-13`: serialize snapshot/log/sequence/ring/broadcast, add atomic selected-field mutation and ordering/concurrency tests | `HUB` | reviewed atomic replay/live cursor correction; blob `cf382f32e195` |
| 19 | `crates/roko-serve/src/lib.rs` | `+3/-3`: migrate cascade-router bootstrap to selected-field StateHub mutation | `HUB` + `CFG` | exact `3041` blob `b943fcad36aa`; `HUB` hunk accepted, separate startup validation remains in pending `CFG` r2 |
| 20 | `crates/roko-serve/src/routes/config.rs` | `+3/-1`: map new dispatch validation errors to HTTP 400 | `CFG` | exact `3041` blob `613db1eb7a9d`; pending transaction/boundary correction |
| 21 | `crates/roko-serve/src/routes/sse.rs` | `+50/-14`: exclusive Last-Event-ID resume, explicit lag gap snapshot, remove never-loop, add reconnect regression | `HUB` | reviewed handoff/oversize/stale-floor correction; blob `6e1d0aadf077` |

Direct current-source inspection confirmed every named symbol/call path: early PID
registration and reader return; resource prechecks and mutable producer probes;
the deadline helper and monotonic activity refresh; all five event-loop ownership
blocks; atomic JSONL recovery; TUI selected-field/bounded paths; lock ordering;
config types/normalizer/default assertions; exclusive atomic staging and parent
sync; worktree Git identity probes; StateHub publication/cursor locking; and SSE
gap construction. The `agents_view.rs` diff is exactly formatting-only.

## Candidate, review, integration, and status proof

| Cluster | Candidate and independent review | Integrated proof | Current status boundary |
|---|---|---|---|
| `PROC` | `fa828276a535`; ACCEPTED `feb5753f1515` | merge `05927deda1c6`; status proof `cad46b1b7952` | bounded precursor `DONE`; no broader SH01 closure |
| `OWN` | `c71eb14f1aaa`; ACCEPTED `69a22c723548` | merge `56b242dcfc2c`; status proof `871b3bbb0020` | bounded precursor `DONE` |
| `DL` | integration-native `51bb0a0e5d0f`; ACCEPTED `58ee07f2b97e` | merge `df76374841ea`; status proof `128dc950c165` | bounded precursor `DONE`; historical conflicting candidate retained only as evidence |
| `GATE` | `bfe7b281abda`; ACCEPTED `81b92cd20d4f` | merge `915d3c246c93`; status proof `fc831c554295` | bounded precursor `DONE` |
| `C4` | rejected `f42df7d7ab10`, corrected cumulative `b8bfd506d316`; ACCEPTED r2 `e07ce8d5c82f` | implementation replay `1967a06879c0` + `dd593bfac850`; integrated review `4ef4e0d84e21`; status `ebcc3add020a` | canonical `SH01-T06C4 = done`; SH01 is truthfully 27/28 with only T07 ready |
| `TUI` | `2b18ae814211`; ACCEPTED `14dc40953d83` | reviewed replay `0a307ab08659` + `1eb2eabb604f`; status `9d89e81608da` | bounded precursor `DONE`; `SH04-T06`/issues remain open |
| `ATOM` | `06ebf26dd4b9`; ACCEPTED `5626cd136908` | reviewed replay `20fae27138ad` + `9c015378a37f`; status `9d89e81608da` | bounded precursor `DONE`; SH03 debris/quarantine remains open |
| `LOCK` | `2e4296e4e4f5`; ACCEPTED `acd2675e474f` | reviewed replay `4454af33548d` + `cd185001ba44`; status `7f5221e9da76` | bounded precursor `DONE`; full dirty-work recovery remains open |
| `HUB` | `0d9b43781966`; ACCEPTED `8b33f6e7751b` | reviewed replay `27bd7df55d50` + `8ccac6d73ec2`; status `cd933788789b` | bounded precursor `DONE`; `SH03-T06` and other consumers remain open |
| `CFG` | r1 `91992857f160` rejected in integrated review `acac82c6e8c2`; r2 source `af66e1446b9f`, evidence-bearing tip `83bec5197842` | no accepted review or integration | **pending** fresh review of r2, integration, and post-merge proof |
| `WT` | `4c5abf86067d` rejected in integrated review `eed6bc78672f` | no corrected candidate or integration | **pending** cancellation-safe Git/state ownership correction, regression, fresh review, integration |

The eight bounded integrated precursor clusters are exactly `PROC`, `OWN`, `DL`,
`GATE`, `TUI`, `ATOM`, `LOCK`, and `HUB`. `C4` is separately accounted because it
advanced a full canonical SH01 task to done after rejection/correction; counting it
as a ninth “bounded precursor” would be a false cluster count.

Several accepted worker/review branch SHAs are not ancestors of the integration
head because the accepted content was replayed onto the current integration base.
The integration SHAs above, the evidence records, blob comparisons, and post-merge
gates make those dispositions explicit; they are not orphan pending work.

## Exact historical control-document census

These ten paths document the July audit or its earlier status/backlog baselines.
They are lower-authority evidence under the master, not proof that a current task
is complete. In particular, dated statements such as `SH01 = 26/28`, `SH02-SH06 =
0/N`, and `SH01-T06C4 remains ready` accurately describe audit start but are now
superseded for current status by the manifest and integrated evidence above.

| # | Path | Original `3041` outcome | Current relation/disposition |
|---:|---|---|---|
| 22 | `tmp/status-quo/00-INDEX.md` | `+368/-141`: expand the July 7-10 status-pack/backlog navigation and historical counts | exact `3041` blob `32c0d2042a74`; historical navigation, not current completion |
| 23 | `tmp/status-quo/backlog/00-INDEX.md` | `+111/-14`: expand E01-E48/DOC backlog navigation and historical execution instructions | later count/source correction `d808803069ec`, independently accepted `1be9dc64392b`, merged `c379370672ae`; current blob `f4d71b556b34`; still a baseline/control index |
| 24 | `tmp/status-quo/backlog/05-MASTER-CHECKLIST.md` | `+458/-27`: historical 447-task E01-E48 flat seed checklist | exact `3041` blob `6ab25580f43a`; not the live programme master |
| 25 | `tmp/status-quo/self-heal/README.md` | `+4/-0`: link the dated changelog and distinguish tasks from precursors | exact `3041` blob `09d3a38ad8c3`; navigation only |
| 26 | `tmp/status-quo/self-heal/changelog/AUDIT-2026-07-14-PASS2.md` | new 201-line second-pass audit of 18 substantive source paths and then-current gates | exact `3041` blob `43024041d925`; immutable dated baseline |
| 27 | `tmp/status-quo/self-heal/changelog/AUDIT-2026-07-14.md` | new 188-line first audit, precursor fixes, open gaps, and then-current verification | exact `3041` blob `6c40baaacd34`; immutable dated baseline |
| 28 | `tmp/status-quo/self-heal/changelog/COMMIT-INVENTORY.md` | new 95-line inventory of the 88 audited pre-precursor commits | exact `3041` blob `e8bf733fdc83`; historical inventory |
| 29 | `tmp/status-quo/self-heal/changelog/README.md` | new 47-line authority/status boundary for the dated changelog | exact `3041` blob `5ea939d6b018`; explicitly audit-start status |
| 30 | `tmp/status-quo/self-heal/changelog/SH01-runner-lifecycle.md` | new 162-line SH01 commit/task history and then-pending C4/T07 boundary | exact `3041` blob `f45b1050cec4`; historical task evidence, not current manifest status |
| 31 | `tmp/status-quo/self-heal/changelog/SH02-SH06-planned-batches.md` | new 77-line planned-batch/non-overclaim ledger | exact `3041` blob `770fbcc90218`; historical planned boundary, not task completion |

No one should infer current completion from these files. Current source, current
manifests, accepted evidence, and the live master outrank them. Their preservation
is correct; future documentation reconciliation may add explicit baseline banners
or update current navigation, but CTRL-02 must not rewrite history or mark any DOC,
SH02-SH06, or enclosing issue complete from these bytes.

## Recovery inventory and resolved seal breach

The saved CTRL-01 recovery state still proves the initial partition:

```text
original HEAD/branch: 3041d095d / main
visible intended control-plane paths: 23
ignored canonical backlog paths: 56
preserved unrelated artifacts: 15
```

During this review, the original root reported the exact saved HEAD and branch,
and every saved status row still matched, but it had one additional row:

```text
? tmp/status-quo/execution-evidence/
```

The only member is:

```text
tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH-REVIEW.md
created: 2026-07-14 16:25:49 +0200
lines: 182
SHA-256: 7cb77fcac159e166ff592384fe082e6d47074b8f3cd29a4cb626d511a0cf0106
```

It is a dirty draft of the canonical committed rejection record, whose current
and `eed6bc786` bytes have SHA-256
`cf8bff45db9748e245ff0d2830400eb0725e7c798510562851abec70181aba6d`.
The files differ at exactly one line: the dirty draft says Tokio `1.52.3`; the
canonical reviewed record says the lockfile-correct `1.51.1`. Thus the draft is
neither original user work nor unique accepted evidence. It was a programme-created,
superseded dirty-only artifact and a breach of the sealed-checkout invariant.

The coordinator then reconciled the breach through the authorized cleanup path:

- preserved the discovery and checksum in
  `$HOME/.local/state/roko/status-quo-20260714T073140Z/seal-breach-CTRL-02-WORKTREE-REATTACH-REVIEW.txt`;
- committed the recovery chronology as `0ce6af71c55c5f28d5d9d0974b8b87ad8f57edd6`
  (`CTRL-02-SEAL-BREACH.md`); and
- used a named patch to remove only the superseded untracked duplicate.

I independently rechecked the original root after that reconciliation. Its HEAD
is still exactly `3041d095d`, its branch is still `main`, its current status has
the same 32 porcelain-v2-with-branch lines as `original-status.txt`, byte
comparison succeeds, and the duplicate path is absent. The seal breach is
therefore preserved as evidence and resolved; it is not a third pending
implementation disposition.

No other new root status row, source-tree artifact, unaccounted 3041 path/hunk, or
unexplained cluster was found.

## Remaining actions after this census

1. Obtain fresh independent acceptance of config r2 tip `83bec5197842`, integrate
   it in dependency order, and run post-merge focused proof. Do not inherit r1's
   rejected verdict as acceptance.
2. Correct worktree cancellation ownership from rejected `4c5abf86067d`, add the
   required controlled cancellation regressions, obtain fresh independent review,
   integrate, and run post-merge proof.
3. Rerun this exact 31-path census/status proof at the resulting integration head.

`READY_FOR_FINALIZATION` means this precursor census is ready for coordinator
finalization with the exact two pending clusters above. It does not mark CTRL-02,
config-dispatch, worktree reattachment, or any enclosing programme outcome done.

No broad Cargo suite was run, per assignment. This review is immutable
history/evidence reconciliation; the focused behavioral gates are bound to the
accepted records above and must be rerun only by the pending cluster owners and
integration owner.
