# CTRL-09 post-merge graph-count correction

> [!IMPORTANT]
> **Status: DONE after fresh independent review and integrated post-merge
> proof.** This correction does not alter the accepted DOC-v2 manifest or
> coverage behavior. It makes the graph evidence exact before canonical
> CTRL-09 completion.

## Scope and lineage

- Correction base: `e84037ef00202ffa9a2932c5c98e7376c742b18b`.
- Original r2 candidate: `abfa50fb8ff50226e6ade3e00e1e13aa3de9c338`.
- Historical r2 rejection: `9f7bc5616` (integrated as `826c9bdd6`).
- Corrected r3 candidate: `3ac488dde191ae9f0dbf32f861cd160153bf263a`.
- Integrated DOC behavior: `a96ffa21d`; integrated r3 count change:
  `560c1bb17`; historical r3 acceptance: `f925843ed` (integrated as
  `e84037ef0`).
- Reserved correction paths: `CTRL-09.md`, `CTRL-09-REVIEW.md`,
  `CTRL-09-REVIEW-r3.md`, and this record only.
- Explicit exclusions: DOC-v2-core manifest, source-coverage ledger, product
  source, tests, master, task statuses, indexes, lockfiles, and release state.

## Root cause and corrected definitions

The r2 rejection claimed that it counted unique plan pairs across both
`meta.depends_on` and task-level `depends_on_plan`. The selected 93-manifest
universe has no relevant `meta.depends_on` graph key. Its two meta-level plan
references use `meta.depends_on_plan`:

```text
E46-github-workflow-integration -> E01-execution-engine
E48-rate-limit-budgeting        -> E01-execution-engine
```

Both pairs are distinct and absent from the task-level pair set. The r2 parser
therefore omitted them, falsely rejected the original 162 full-graph count, and
recast 160 as the full count. The r3 correction and acceptance repeated that
incomplete definition. The numbers become consistent when named precisely:

| Revision / definition | Reference occurrences | Unique plan pairs |
|---|---:|---:|
| base `bb5048f4c`, task-level runtime | 296 | 133 |
| base `bb5048f4c`, meta-level declared | 2 | 2 |
| base `bb5048f4c`, all declared | 298 | 135 |
| integrated `e84037ef0`, task-level runtime | 320 | 160 |
| integrated `e84037ef0`, meta-level declared | 2 | 2 |
| integrated `e84037ef0`, all declared | 322 | 162 |

Thus the DOC change adds 27 unique task-level pairs and removes none:
`133 + 27 - 0 = 160` for scheduler-visible task dependencies, while
`135 + 27 - 0 = 162` for every declared task/meta plan dependency. The DOC
manifest itself introduces no meta edge.

## Independent reproduction

The correction author ran the following read-only census from the immutable
correction base. It loads manifests directly from both Git revisions, selects
only the canonical 32 top-level, 55 backlog, and 6 self-heal plan roots, checks
same-plan and cross-plan resolution, and runs Tarjan SCC detection over the
task-level and all-declared pair sets:

```text
python3 - <<'PY'
import subprocess, tomllib
from pathlib import PurePosixPath

def manifests(rev):
    names = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", rev], text=True
    ).splitlines()
    selected = []
    for name in names:
        p = PurePosixPath(name)
        if p.name != "tasks.toml":
            continue
        if len(p.parts) == 3 and p.parts[0] == "plans":
            selected.append(name)
        elif len(p.parts) == 6 and p.parts[:4] in {
            ("tmp", "status-quo", "backlog", "plans"),
            ("tmp", "status-quo", "self-heal", "plans"),
        }:
            selected.append(name)
    docs = [tomllib.loads(subprocess.check_output(
        ["git", "show", f"{rev}:{name}"], text=True
    )) for name in selected]
    return selected, {doc["meta"]["plan"]: doc for doc in docs}

def cyclic_scc(plan_ids, edges):
    graph = {p: set() for p in plan_ids}
    for source, target in edges:
        graph[source].add(target)
    index = 0; stack = []; on_stack = set(); indices = {}; low = {}; cycles = []
    def visit(node):
        nonlocal index
        indices[node] = low[node] = index; index += 1
        stack.append(node); on_stack.add(node)
        for target in graph[node]:
            if target not in indices:
                visit(target); low[node] = min(low[node], low[target])
            elif target in on_stack:
                low[node] = min(low[node], indices[target])
        if low[node] == indices[node]:
            component = []
            while True:
                member = stack.pop(); on_stack.remove(member); component.append(member)
                if member == node:
                    break
            if len(component) > 1 or component[0] in graph[component[0]]:
                cycles.append(component)
    for node in graph:
        if node not in indices:
            visit(node)
    return cycles

for rev in ("bb5048f4c", "e84037ef0"):
    paths, plans = manifests(rev)
    local_missing = []; plan_missing = []; task_refs = []; meta_refs = []
    for plan_id, doc in plans.items():
        task_ids = {task["id"] for task in doc.get("task", [])}
        for task in doc.get("task", []):
            local_missing += [(plan_id, task["id"], dep) for dep in task.get("depends_on", []) if dep not in task_ids]
            task_refs += [(plan_id, dep) for dep in task.get("depends_on_plan", [])]
        meta_refs += [(plan_id, dep) for dep in doc.get("meta", {}).get("depends_on_plan", [])]
    for source, target in task_refs + meta_refs:
        if target not in plans:
            plan_missing.append((source, target))
    task_edges = set(task_refs); meta_edges = set(meta_refs); all_edges = task_edges | meta_edges
    roots = [sum(name.startswith(prefix) for name in paths) for prefix in
             ("plans/", "tmp/status-quo/backlog/plans/", "tmp/status-quo/self-heal/plans/")]
    print(rev, roots, len(plans), sum(len(d.get("task", [])) for d in plans.values()),
          len(task_refs), len(meta_refs), len(task_edges), len(all_edges),
          len(local_missing), len(plan_missing), len(cyclic_scc(plans, task_edges)),
          len(cyclic_scc(plans, all_edges)))
PY
```

Exact output:

```text
bb5048f4c [32, 55, 6] 93 881 296 2 133 135 0 0 0 0
e84037ef0 [32, 55, 6] 93 881 320 2 160 162 0 0 0 0
```

Column order after tasks is: task-level reference occurrences, meta-level
reference occurrences, task-level unique pairs, all-declared unique pairs,
unresolved local references, unresolved plan references, task-graph cyclic
SCCs, and all-declared-graph cyclic SCCs.

All tracked TOML and sealing controls were rerun independently:

```text
git ls-files '*.toml' | wc -l
# 193

python3 - <<'PY'
from pathlib import Path
import subprocess, tomllib
paths = subprocess.check_output(
    ["git", "ls-files", "*.toml"], text=True
).splitlines()
for name in paths:
    with Path(name).open("rb") as handle:
        tomllib.load(handle)
print(f"tracked_toml={len(paths)} errors=0")
PY
# tracked_toml=193 errors=0

shasum -a 256 plans/INDEX.md
# 7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44

git diff --check
# exit 0
```

The sealed index hash is
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
No validator was run against the source tree, and no generated index or other
artifact was created.

## Integrated disposition

- Fresh independent review accepted the correction in
  `ae41a1f12e26bf74af4472de7ea2625836a0e51f`; integrated correction and review
  commits are `4a4a7b68d` and `7f2105c45`.
- Post-merge proof reproduced 93 plans, 881 tasks, 849 local references, 320
  task-level plan references, two meta-level plan references, 160 unique
  runtime edges, 162 unique all-declared edges, zero unresolved references,
  and zero cyclic SCCs under either graph definition.
- The ten DOC roll-ups remain `ready` with 34 unique docs-only writers, exact
  coverage-ledger representation, and complete E19-E45 dependency coverage.
  All 193 TOMLs parse.
- Disposable strict validation reported zero diagnostics for the 55-plan
  backlog, six-plan self-heal root, and the DOC-v2-core plan. The source index
  remained sealed at the recorded SHA-256 and integration was clean.
- Final status: `DONE`. The two earlier count reviews remain visible with
  prominent supersession notices; their valid manifest/coverage findings are
  retained without allowing the obsolete graph wording to remain canonical.
