# CTRL-14 — Retired authoring-plan supersession proof

## Review state

`DONE` — all 96 retired authoring records remain skipped and resolve
exactly to stronger live task definitions; both defects that made the first proof
nonterminal are now independently reviewed, merged, and cleanly rerun on current
integration base `fb5a47f6bf024a30bce4c6b345896e390b5684b8`.

Terminal candidate `2f3845b6a81a904e899264e392e06273ee3944cd` was independently
accepted by `64273a9c43a167b012822df244192e370d4fd073` and integrated by
`9b3822b727441ba925ccbb0e92427aa4c17ba9e6`. Post-merge reruns report retired
strict `0 diagnostics in 1 plan`, dry-run `0` plans/`0` tasks, backlog strict
`0 diagnostics in 55 plans`, and self-heal strict `0 diagnostics in 6 plans`.
The sealed source index hash remains unchanged. `DONE` is limited to proving this
retired compatibility plan must not execute; it does not complete the 440 ready
implementation tasks or 71 ready DOC tasks mapped by the proof.

## Terminal rerun and current integrated proof

### Assignment and prerequisite ancestry

- Worker base: `fb5a47f6bf024a30bce4c6b345896e390b5684b8`.
- Worker branch/worktree: `agent/CTRL-14-terminal-supersession` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-14-TERMINAL`.
- Reserved write scope: this evidence file only. No master, manifest, index, ledger,
  audit, production file, lockfile, or integration worktree is changed.
- Original accurate nonterminal proof: `ed5ab0fed4a820b814d59b398c5c989b5003cfdf`;
  review `fb632f521d8e59dbb110ea45f947f11aa00210e1` accepted its evidence accuracy while
  explicitly refusing terminal status.
- Source/index repair: corrected candidate
  `d808803069ecb9f74d63cb7baa6e41ddd69368ad`; final review
  `1be9dc64392b6556e2044f18eab922bf3f6f8eb3` (`ACCEPTED`); integration merge
  `c379370672ae3119a5d5aa660be2ef63f5a73adb`.
- Strict-validation repair and terminal ledger proof: candidate
  `9458a6920d72e457553e31cd51b9ac89d70d2483`, review
  `81d1af92b142ce512964b078ccb5bc1a417b8e2d`, prerequisite merge
  `206e9079812b27f738d95f91d1135d0f663c836f`, ledger candidate
  `950fa8bc95a2b92f90dc970d6038547a28feb9e4`, ledger review
  `91da0fea5604e7639928824c3ab8ab07c21832af`, ledger merge
  `f0cf7e769306b3217b30de797e2698b1a673326e`, and terminal integration evidence at
  this base. Git ancestry confirms every named prerequisite is present.

### Immutable manifest and exact-mapping traversal

The exact base commit was exported with `git archive` to a new disposable directory.
Python 3 `tomllib` read the complete 6,349-line retired manifest, all 48 live E manifests,
all six live DOC manifests, all six source-coverage ledgers, both live backlog indexes,
and the generated top-level `plans/INDEX.md`. The traversal asserted every manifest's
metadata and every task ID/status. For all 96 replacement targets it additionally asserted
the complete nested context/verify/acceptance contract, every stable-ID mapping, the declared
target-plan and source-epic paths, and both live index tables.

For every retired task it removed only `GAP-`, required the resulting stable ID to
occur exactly once in the 447-task canonical corpus, required its canonical path to
equal the retired record's declared target plan, required the declared epic to exist
and name that stable ID, and required the replacement to contain nonempty role/files,
surgical read-file/symbol/anti-pattern context, non-placeholder verify commands with
failure messages, and acceptance coverage.

Observed result:

```text
epic_manifests=48 canonical_tasks=447 statuses=done:7,ready:440
epic_subtotals=E01-E18:169,E19-E45:243,E46-E48:35
doc_manifests=6 doc_tasks=71 statuses=ready:71
retired_tasks=96 skipped=96 executable=0 dependency_edges=79
mapped_targets=96 orphan_targets=0 orphan_acceptance=0 epic_groups=17
titles=exact:4,refined_under_07_notes:92
verify=retired:480,canonical_targets:290
acceptance=retired:384,canonical_targets:227
groups=E01:7,E02:8,E03:4,E04:16,E05:5,E06:6,E07:7,E08:4,E09:6,E10:4,E11:2,E12:6,E13:1,E14:4,E15:3,E17:3,E18:10
index_rows=48 index_tasks=447 index_gaps=0
```

The four literal title matches remain `E01-T07`, `E06-T05`, `E10-T04`, and
`E12-T05`. The other 92 use exact stable identity and the exact declared plan/epic
paths; `07-SUBAGENT-TASK-AUTHORING-NOTES.md` preserves why their implementation titles
and task contracts were refined. There is no fuzzy or title-only match.

For additional immutability, the SHA-256 of the 96 sorted records in the form
`"GAP-ID -> TARGET-ID | PLAN-PATH | EPIC-PATH\n"` is:

```text
9bc9bf4e015ce7a71dcb30a866a39a103c89e5b0e7c8e64652b640b9805825f9
```

Current corpus seals are:

```text
retired_manifest_sha256=4705f6f7d00403f8aa33fb14db89bb5a8353a67468fe3a3d3af613245f7a9d03
epic_manifest_files=48
epic_manifest_bytes=1505701
epic_manifest_set_sha256=8f43355496a43b35f84ec56467452d7f2f21fbcc2243fc1ba45c934ceac59f11
```

The retired hash is unchanged from the original proof. The live E-manifest seal and
byte count changed because reviewed CTRL-07 metadata corrections are now integrated;
the fresh traversal proves their task identities, statuses, replacement ownership,
and acceptance completeness remain intact. The original snapshot-specific status
assertion was updated from 6/441 to the integrated 7/440 census because reviewed
CTRL-05 completed `E11-T01`; that task is not one of the retired plan's E11-T04/T05
replacement targets. All supersession invariants were rerun unchanged.

### Source, DOC, and index coverage

The exact source enumeration published in
`tmp/status-quo/backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md` was run inside the immutable
snapshot:

```text
sources=745
missing_from_ledgers=0
missing_from_doc_tasks=0
```

The source set includes 109 direct `tmp/status-quo/*.md` documents. The canonical
master occurs in the status-quo source ledger and is owned by exactly one DOC task,
`DOC-SQ-01`. That task's structural coverage command passes, and focused strict
validation reports `0 diagnostics in 1 plan`.

Structured index comparison proves all 48 E01-E48 rows in
`tmp/status-quo/backlog/plans/00-INDEX.md` have the exact directory and task count from
their manifests, total 447, and report zero definition gaps. The live navigation and
coverage ledgers agree on 169 E01-E18 tasks, 447 implementation tasks, 71 DOC tasks,
and 745 covered sources. The external generated index was also read completely and
still reports its separate 29-plan/120-task executable queue plus two superseded
plans/66 excluded tasks; this proof does not conflate that queue with the backlog.

### Strict validation and nonexecution

The integration-owned validator was used, not a worker artifact:

```text
roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git d4749f9c7)
```

From one disposable full archive:

```text
backlog strict:   exit 0; 0 diagnostics in 55 plans
self-heal strict: exit 0; 0 diagnostics in 6 plans
```

From a separate disposable full archive, the retired plan alone produced:

```text
retired strict: exit 0; 0 diagnostics in 1 plan
retired dry-run: exit 0
{
  "dry_run": true,
  "plans": [],
  "total_plans": 0,
  "total_tasks": 0
}
```

The runtime path still agrees: `TaskDef::is_ready` requires literal `ready`,
`task_status_is_terminal` includes `skipped`, and `TaskTracker::new` pre-collects all
manifest-skipped IDs. Resetting or executing this retired plan would therefore undo
an explicit, proved supersession boundary.

The source `plans/INDEX.md` SHA-256 before and after every isolated command remained:

```text
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

The validator regenerated only the disposable fixture's index and runtime state;
the worker and integration worktrees remained unchanged.

### Disposition of the original nonterminal findings

1. **Missing master source/DOC assignment — resolved.** The accepted coverage repair
   assigns the master to the 109-document status-quo ledger and `DOC-SQ-01`; the
   integrated published check is now 745/0/0.
2. **Stale plan/index/validation inventory — resolved.** The accepted correction
   reconciles both live backlog indexes to 169/447/745 while preserving historical
   149/744 statements as history. CTRL-07 separately replaced the stale warning
   paragraph with reviewed integrated strict results of backlog 0/55 and self-heal
   0/6.

The remainder of this file deliberately preserves the original snapshot, mapping
table, reproduction program, rejection findings, and requested rerun. Its counts and
hashes are historical facts at base `5719b51c0`, not current integrated claims.

## Original nonterminal snapshot (preserved history)

- Worker branch: `agent/CTRL-14-authoring-supersession`
- Reviewed base: `5719b51c03dfde9e2709233d51e422c826ee97a2`
- Reserved write scope: this evidence file only
- Retired manifest:
  `tmp/status-quo/backlog/plans/status-quo-authoring-gaps/tasks.toml`
- Retired-manifest SHA-256:
  `4705f6f7d00403f8aa33fb14db89bb5a8353a67468fe3a3d3af613245f7a9d03`
- Canonical manifest set: all 48 `tmp/status-quo/backlog/plans/E*/tasks.toml`
  files, 1,501,633 bytes
- Canonical manifest-set SHA-256 (SHA-256 of sorted
  `"<file-sha256>  <path>\n"` records):
  `e3cb1eb1a3ec0300aaa07efc652090e1bc492139d90e5cfe892659ba1a1a8607`

No manifest, index, ledger, audit, source document, or product file was changed in
this candidate.

## Supersession result

The retired manifest is structurally and operationally inert:

- `[meta]` is exactly `total = 96`, `done = 96`, `status = "superseded"`,
  `superseded_by = "per-epic-plans-complete"`, and `skip_enrichment = true`.
- It contains exactly 96 unique `GAP-*` tasks. All 96 are `skipped`; zero are
  `pending`, `ready`, or `active`.
- Its internal dependency graph contains 79 edges, all resolving to another one of
  the 96 retired IDs.
- Removing the `GAP-` prefix produces 96 unique stable task IDs. Every ID resolves
  exactly once in the exact canonical plan path declared by the retired record, and
  every declared source epic exists.
- Every resolved canonical task has nonempty role/files, surgical context
  (`read_files`, `symbols`, `anti_patterns`), executable non-placeholder verify
  commands with failure messages, and at least one acceptance record.
- The 480 retired verify records and 384 retired authoring-acceptance records have no
  orphan target. The 96 concrete replacements contain 290 implementation verify
  records and 227 implementation acceptance records.
- The full canonical corpus contains 48 manifests and 447 unique task IDs, with each
  manifest's `meta.total` and `meta.done` agreeing with its parsed tasks.

The exact stable-ID/category mapping is:

| Epic | Retired count | Stable target IDs | Canonical plan | Source epic |
|---|---:|---|---|---|
| `E01` | 7 | `E01-T04`–`E01-T10` | `tmp/status-quo/backlog/plans/E01-execution-engine/tasks.toml` | `tmp/status-quo/backlog/epics/E01-EXECUTION-ENGINE.md` |
| `E02` | 8 | `E02-T04`–`E02-T11` | `tmp/status-quo/backlog/plans/E02-STORAGE-CONVERGENCE/tasks.toml` | `tmp/status-quo/backlog/epics/E02-STORAGE-CONVERGENCE.md` |
| `E03` | 4 | `E03-T04`–`E03-T07` | `tmp/status-quo/backlog/plans/E03-type-consolidation/tasks.toml` | `tmp/status-quo/backlog/epics/E03-TYPE-CONSOLIDATION.md` |
| `E04` | 16 | `E04-T04`–`E04-T19` | `tmp/status-quo/backlog/plans/E04-security-perimeter/tasks.toml` | `tmp/status-quo/backlog/epics/E04-SECURITY-PERIMETER.md` |
| `E05` | 5 | `E05-T04`–`E05-T08` | `tmp/status-quo/backlog/plans/E05-gate-adaptivity-live/tasks.toml` | `tmp/status-quo/backlog/epics/E05-GATE-ADAPTIVITY-LIVE.md` |
| `E06` | 6 | `E06-T04`–`E06-T09` | `tmp/status-quo/backlog/plans/E06-COMPOSE-UNIFY/tasks.toml` | `tmp/status-quo/backlog/epics/E06-COMPOSE-UNIFY.md` |
| `E07` | 7 | `E07-T04`–`E07-T10` | `tmp/status-quo/backlog/plans/E07-learning-knowledge/tasks.toml` | `tmp/status-quo/backlog/epics/E07-LEARNING-KNOWLEDGE.md` |
| `E08` | 4 | `E08-T04`–`E08-T07` | `tmp/status-quo/backlog/plans/E08-conductor-supervision/tasks.toml` | `tmp/status-quo/backlog/epics/E08-CONDUCTOR-SUPERVISION.md` |
| `E09` | 6 | `E09-T04`–`E09-T09` | `tmp/status-quo/backlog/plans/E09-OBSERVABILITY/tasks.toml` | `tmp/status-quo/backlog/epics/E09-OBSERVABILITY.md` |
| `E10` | 4 | `E10-T04`–`E10-T07` | `tmp/status-quo/backlog/plans/E10-FRONTEND-CONTRACT/tasks.toml` | `tmp/status-quo/backlog/epics/E10-FRONTEND-CONTRACT.md` |
| `E11` | 2 | `E11-T04`–`E11-T05` | `tmp/status-quo/backlog/plans/E11-chain-isfr/tasks.toml` | `tmp/status-quo/backlog/epics/E11-CHAIN-ISFR.md` |
| `E12` | 6 | `E12-T04`–`E12-T09` | `tmp/status-quo/backlog/plans/E12-DEAD-CODE-CLEANUP/tasks.toml` | `tmp/status-quo/backlog/epics/E12-DEAD-CODE-CLEANUP.md` |
| `E13` | 1 | `E13-T03` | `tmp/status-quo/backlog/plans/E13-SPEC-DEBT-V2/tasks.toml` | `tmp/status-quo/backlog/epics/E13-SPEC-DEBT-V2.md` |
| `E14` | 4 | `E14-T04`–`E14-T07` | `tmp/status-quo/backlog/plans/E14-providers-tools/tasks.toml` | `tmp/status-quo/backlog/epics/E14-PROVIDERS-TOOLS.md` |
| `E15` | 3 | `E15-T4`–`E15-T6` | `tmp/status-quo/backlog/plans/E15-mcp-config/tasks.toml` | `tmp/status-quo/backlog/epics/E15-MCP-CONFIG.md` |
| `E17` | 3 | `E17-T04`–`E17-T06` | `tmp/status-quo/backlog/plans/E17-acp-completion/tasks.toml` | `tmp/status-quo/backlog/epics/E17-ACP-COMPLETION.md` |
| `E18` | 10 | `E18-T04`–`E18-T13` | `tmp/status-quo/backlog/plans/E18-DOCS-CONFIG-OPS/tasks.toml` | `tmp/status-quo/backlog/epics/E18-DOCS-CONFIG-OPS.md` |
| **Total** | **96** | **96 unique IDs** | **17 named plans** | **17 named epics** |

E16 has no retired gap record and therefore correctly does not appear in this
17-category mapping.

### Stable IDs versus refined titles

Only 4 canonical targets retain the retired checklist title literally; 92 have
refined titles. This is not an inferred fuzzy match. The link is the exact stable task
ID plus the exact target plan path carried by every retired task.
`tmp/status-quo/backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md` is the explicit stronger
supersession record: it says the notes were consumed during per-epic expansion, the
96 authoring tasks are skipped, and their stale paths, dependency shapes, unsafe
scopes, roles, and validation commands were corrected. Thus:

- 4 retired title outcomes are fulfilled literally;
- 92 retired title outcomes are explicitly superseded by the refined canonical
  definitions and the named correction ledger;
- all 96 exact-ID existence/completeness/non-placeholder-check outcomes resolve;
- no acceptance outcome is silently dropped or matched by title similarity.

## Reproducible manifest and acceptance check

Run from the repository root with Python 3.11 or newer:

```python
from pathlib import Path
import collections
import hashlib
import tomllib

root = Path(".")
retired_path = root / "tmp/status-quo/backlog/plans/status-quo-authoring-gaps/tasks.toml"
retired = tomllib.loads(retired_path.read_text())
epic_paths = sorted((root / "tmp/status-quo/backlog/plans").glob("E*/tasks.toml"))

canonical = {}
canonical_path = {}
status_counts = collections.Counter()
for path in epic_paths:
    doc = tomllib.loads(path.read_text())
    tasks = doc["task"]
    assert doc["meta"]["total"] == len(tasks), path
    assert doc["meta"]["done"] == sum(t["status"] == "done" for t in tasks), path
    for task in tasks:
        assert task["id"] not in canonical, task["id"]
        canonical[task["id"]] = task
        canonical_path[task["id"]] = path
        status_counts[task["status"]] += 1

assert len(epic_paths) == 48 and len(canonical) == 447
assert status_counts == collections.Counter(done=6, ready=441), status_counts

meta = retired["meta"]
gaps = retired["task"]
assert (
    meta["plan"], meta["total"], meta["done"], meta["status"],
    meta["superseded_by"], meta["skip_enrichment"],
) == (
    "status-quo-authoring-gaps", 96, 96, "superseded",
    "per-epic-plans-complete", True,
)
assert len(gaps) == len({g["id"] for g in gaps}) == 96
assert {g["status"] for g in gaps} == {"skipped"}
assert not [g for g in gaps if g["status"] in {"pending", "ready", "active"}]
gap_ids = {g["id"] for g in gaps}
assert all(set(g["depends_on"]) <= gap_ids for g in gaps)
assert sum(len(g["depends_on"]) for g in gaps) == 79

def acceptance(task):
    # Both placements occur in the historical TOML corpus.
    return list(task.get("acceptance", [])) + [
        item
        for verify in task["verify"]
        for item in verify.get("acceptance", [])
    ]

groups = collections.Counter()
exact_titles = 0
canonical_acceptance = retired_acceptance = 0
canonical_verify = retired_verify = 0
for gap in gaps:
    target = gap["id"].removeprefix("GAP-")
    target_plan = root / next(p for p in gap["files"] if p.endswith("/tasks.toml"))
    source_epic = root / next(p for p in gap["files"] if "/epics/" in p)
    assert target in canonical, target
    assert canonical_path[target] == target_plan, (
        target, canonical_path[target], target_plan,
    )
    assert source_epic.is_file(), source_epic

    task = canonical[target]
    assert task["role"] and task["files"]
    context = task["context"]
    assert context["read_files"] and context["symbols"] and context["anti_patterns"]
    assert task["verify"]
    assert all(v.get("command") and v.get("fail_msg") for v in task["verify"])
    assert all(
        v["command"].strip() not in {"true", "echo ok", ":", "exit 0"}
        for v in task["verify"]
    )
    assert acceptance(task), target

    original_title = gap["title"].split(f"{target} - ", 1)[1]
    exact_titles += task["title"] == original_title
    canonical_acceptance += len(acceptance(task))
    retired_acceptance += len(acceptance(gap))
    canonical_verify += len(task["verify"])
    retired_verify += len(gap["verify"])
    groups[target.split("-")[0]] += 1

assert len(groups) == 17 and sum(groups.values()) == 96
assert (exact_titles, 96 - exact_titles) == (4, 92)
assert (retired_verify, retired_acceptance) == (480, 384)
assert (canonical_verify, canonical_acceptance) == (290, 227)

print("epic_manifests=48 canonical_tasks=447 statuses=done:6,ready:441")
print("retired_tasks=96 skipped=96 executable=0 dependency_edges=79")
print("mapped_targets=96 unique_epic_groups=17 orphan_targets=0 orphan_acceptance=0")
print("titles=exact:4,refined_under_07_notes:92")
print(f"verify=retired:{retired_verify},canonical_targets:{canonical_verify}")
print(f"acceptance=retired:{retired_acceptance},canonical_targets:{canonical_acceptance}")
print("groups=" + ",".join(f"{k}:{groups[k]}" for k in sorted(groups)))
print("retired_sha256=" + hashlib.sha256(retired_path.read_bytes()).hexdigest())
records = "".join(
    f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.as_posix()}\n"
    for p in epic_paths
)
print("epic_manifest_set_sha256=" + hashlib.sha256(records.encode()).hexdigest())
```

Observed output:

```text
epic_manifests=48 canonical_tasks=447 statuses=done:6,ready:441
retired_tasks=96 skipped=96 executable=0 dependency_edges=79
mapped_targets=96 unique_epic_groups=17 orphan_targets=0 orphan_acceptance=0
titles=exact:4,refined_under_07_notes:92
verify=retired:480,canonical_targets:290
acceptance=retired:384,canonical_targets:227
groups=E01:7,E02:8,E03:4,E04:16,E05:5,E06:6,E07:7,E08:4,E09:6,E10:4,E11:2,E12:6,E13:1,E14:4,E15:3,E17:3,E18:10
retired_sha256=4705f6f7d00403f8aa33fb14db89bb5a8353a67468fe3a3d3af613245f7a9d03
epic_manifest_set_sha256=e3cb1eb1a3ec0300aaa07efc652090e1bc492139d90e5cfe892659ba1a1a8607
```

## Strict validation and nonexecution proof

The corrected integration validator binary was built from integration commit
`a4278ced081c9f42ef186b8c4a93528ef78c05c3`. To avoid adding `.roko` state to the
worker checkout, the retired manifest was copied to an isolated temporary fixture:

```sh
set -eu
repo=$PWD
roko=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target/debug/roko
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/retired"
cp tmp/status-quo/backlog/plans/status-quo-authoring-gaps/tasks.toml "$fixture/retired/tasks.toml"
ln -s "$repo/tmp" "$fixture/tmp"
cd "$fixture"
"$roko" plan validate --strict retired --color never
"$roko" plan run retired --workdir "$fixture" --dry-run --json --no-serve --color never
```

Observed output, exit code 0:

```text
0 diagnostics in 1 plan
{
  "dry_run": true,
  "plans": [],
  "total_plans": 0,
  "total_tasks": 0
}
```

This agrees with the runtime implementation:

- `TaskDef::is_ready` in `crates/roko-cli/src/task_parser.rs` requires status
  `ready`; `skipped` cannot dispatch.
- `task_status_is_terminal` in `crates/roko-cli/src/runner/task_dag.rs` treats
  `skipped` as terminal.
- `TaskTracker::new` in `crates/roko-cli/src/orchestrate.rs` pre-collects all
  manifest-skipped IDs.

The retired plan therefore must remain superseded/skipped and must never be reset,
queued, or executed.

## Control-document cross-check

The authoritative statements about this retired plan agree:

- `backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md` says the 96 records remain only as
  skipped/superseded provenance so root execution cannot re-author canonical blocks.
- `backlog/07-SUBAGENT-TASK-AUTHORING-NOTES.md` names the consumed corrections and
  explicitly retires all 96 authoring tasks.
- `backlog/00-INDEX.md`, `backlog/plans/00-INDEX.md`,
  `audit-2026-07-14/README.md`, and
  `audit-2026-07-14/BACKLOG-ROADMAP-AUDIT.md` all classify the 96 records as
  superseded/skipped provenance excluded from remaining implementation work.
- The audit's exact manifest row is `96 total / 0 done / 0 ready / 96 skipped`.

The supersession statements agree, but their surrounding aggregate inventories do
not yet all agree. Those defects prevent final acceptance.

## Original required reconciliation (resolved above)

### 1. Missing canonical master in source coverage (historical review finding)

The exact coverage command published in
`tmp/status-quo/backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md` currently produces:

```text
sources=745
missing_from_ledgers=1
missing_from_doc_tasks=1
tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md
```

The document instead records `sources=744`, `missing_from_ledgers=0`, and
`missing_from_doc_tasks=0`. Direct search confirms that the canonical master is in
neither `backlog/source-coverage/*.md` nor `backlog/plans/DOC-*/tasks.toml`.

Required repair by the source-coverage owner:

1. assign `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md` in the canonical
   `status-quo-corpus` coverage ledger;
2. add it to the appropriate named `DOC-status-quo-corpus` task without creating a
   duplicate task or queue;
3. reconcile the status-quo corpus count from 108 to 109 and the aggregate source
   count from 744 to 745 everywhere those values claim current coverage;
4. independently review and merge the repair, then rerun the exact published
   coverage command and require both missing counts to be zero.

Three materially different checks ruled out an alternate existing assignment:

1. the published glob-and-membership coverage script found the exact missing path;
2. direct `rg` across every source ledger and DOC manifest found no path alias or
   indirect assignment;
3. Git history plus the `status-quo-corpus` ledger metadata showed that its declared
   108-file snapshot predates inclusion of the current canonical master.

### 2. Stale plan index and validation inventory (historical review finding)

`tmp/status-quo/backlog/plans/00-INDEX.md` claims all `E*` manifests contain 149
implementation tasks, lists only E01–E18, totals those plans as 149, and claims 744
source documents. Parsed reality is 48 epic manifests / 447 tasks; even the current
E01–E18 subtotal is 169; the current source corpus is 745. The canonical audit already
labels this file `materially stale`.

The executable coverage ledger also says root validation has six expected
`PLAN_031` warnings. With the corrected validator, current non-strict root validation
returns exit 0 with **13 diagnostics in 55 plans**; strict root validation returns
exit 1 with the same 13 diagnostics. This does not alter the isolated retired-plan
result (`0 diagnostics`), but the recorded aggregate warning count must not be
presented as current proof.

Required repair by the index/coverage owner: regenerate the plan index from all 48
epic manifests, reconcile 169/447 and 745 source counts, and either update or clearly
baseline/supersede the stale validation-warning statement using reproducible current
output.

## Original requested acceptance rerun (completed above)

After both correction commits are integrated:

1. rerun the Python manifest/acceptance script above unchanged;
2. rerun the exact source-coverage script published in
   `backlog/08-SOURCE-CORPUS-PLAN-COVERAGE.md` and require
   `sources=745`, `missing_from_ledgers=0`, `missing_from_doc_tasks=0`;
3. rerun isolated strict validation and dry-run and require `0 diagnostics` and
   `total_tasks: 0`;
4. compare all plan/index/ledger/audit counts against parsed manifests;
5. amend this evidence with the integrated correction SHA and change
   `REVIEW_NOT_READY` only after independent review accepts the combined proof.
