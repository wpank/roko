# CTRL-09 post-merge graph-count correction independent review

- **Verdict:** `ACCEPTED`
- **Candidate:** `02b038a258439fb0aec56c643f2d27215b51aa36`
- **Exact base / parent:** `e84037ef00202ffa9a2932c5c98e7376c742b18b`
- **Integration observed during review:**
  `85d8e3253510f72553f5b3e15ae702b885f0d0b2`
- **Review boundary:** count/evidence correction only. Acceptance authorizes the
  coordinator to integrate the immutable candidate and rerun post-merge proof;
  it does not by itself make CTRL-09 terminal.

## Scope and lineage review

`git rev-parse 02b038a^` returned the exact stated base. The candidate changes
exactly four evidence paths:

```text
A tmp/status-quo/execution-evidence/CTRL-09-POSTMERGE-CORRECTION.md
M tmp/status-quo/execution-evidence/CTRL-09-REVIEW-r3.md
M tmp/status-quo/execution-evidence/CTRL-09-REVIEW.md
M tmp/status-quo/execution-evidence/CTRL-09.md
```

The diff adds the blocking correction record, adds explicit supersession
banners to both historical reviews, and labels the task-level and all-declared
counts precisely in the original evidence. It does not rewrite or delete the
historical rejection or acceptance. The correction remains explicitly
`BLOCKED pending fresh independent review`; neither status nor master control
state is changed by the candidate.

The following history was independently inspected with `git show -s
--format='%H %P %s'` and `git log --first-parent --reverse`:

```text
abfa50fb8... parent bb5048f4c... docs(CTRL-09): deduplicate v2 documentation rollups
9f7bc5616... parent abfa50fb8... review(CTRL-09): reject abfa50fb8 graph count
826c9bdd6... parent bb5048f4c... integrated historical r2 rejection
3ac488dde... parent abfa50fb8... docs(CTRL-09): correct graph edge census
f925843ed... parent 3ac488dde... review(CTRL-09): accept 3ac488dde roll-up proof
a96ffa21d... integrated DOC behavior
560c1bb17... parent a96ffa21d... integrated r3 graph-count evidence
e84037ef0... parent 560c1bb17... integrated historical r3 acceptance
02b038a25... parent e84037ef0... this correction candidate
```

This matches the lineage stated by the correction. The banners preserve both
historical decisions while removing their authority over the now-corrected
full-graph definition.

## Independent graph reproduction

I independently loaded every selected manifest from Git objects for the
historical base, correction base, immutable candidate, and observed integration
revision. The script selected only `plans/*/tasks.toml`, backlog plans, and
self-heal plans; read task `depends_on_plan` and the actual
`meta.depends_on_plan` key separately; resolved same-plan and cross-plan
references; and ran Tarjan SCC detection on both the task-runtime and combined
declared graphs.

The exact revisions passed to the read-only Python census were:

```text
bb5048f4c
e84037ef00202ffa9a2932c5c98e7376c742b18b
02b038a258439fb0aec56c643f2d27215b51aa36
85d8e3253510f72553f5b3e15ae702b885f0d0b2
```

Exact results:

```text
REV bb5048f4c
roots [32, 55, 6] plans 93 tasks 881 statuses {'done': 33, 'ready': 752, 'skipped': 96}
task_refs 296 meta_refs 2 task_unique 133 meta_unique 2 all_unique 135
meta_pairs [('E46-github-workflow-integration', 'E01-execution-engine'), ('E48-rate-limit-budgeting', 'E01-execution-engine')] disjoint True
unresolved_local 0 unresolved_plan 0 runtime_scc 0 declared_scc 0
REV e84037ef00202ffa9a2932c5c98e7376c742b18b
roots [32, 55, 6] plans 93 tasks 881 statuses {'done': 33, 'ready': 752, 'skipped': 96}
task_refs 320 meta_refs 2 task_unique 160 meta_unique 2 all_unique 162
meta_pairs [('E46-github-workflow-integration', 'E01-execution-engine'), ('E48-rate-limit-budgeting', 'E01-execution-engine')] disjoint True
unresolved_local 0 unresolved_plan 0 runtime_scc 0 declared_scc 0
REV 02b038a258439fb0aec56c643f2d27215b51aa36
roots [32, 55, 6] plans 93 tasks 881 statuses {'done': 33, 'ready': 752, 'skipped': 96}
task_refs 320 meta_refs 2 task_unique 160 meta_unique 2 all_unique 162
meta_pairs [('E46-github-workflow-integration', 'E01-execution-engine'), ('E48-rate-limit-budgeting', 'E01-execution-engine')] disjoint True
unresolved_local 0 unresolved_plan 0 runtime_scc 0 declared_scc 0
REV 85d8e3253510f72553f5b3e15ae702b885f0d0b2
roots [32, 55, 6] plans 93 tasks 881 statuses {'done': 33, 'ready': 752, 'skipped': 96}
task_refs 320 meta_refs 2 task_unique 160 meta_unique 2 all_unique 162
meta_pairs [('E46-github-workflow-integration', 'E01-execution-engine'), ('E48-rate-limit-budgeting', 'E01-execution-engine')] disjoint True
unresolved_local 0 unresolved_plan 0 runtime_scc 0 declared_scc 0
```

The two meta pairs are distinct from one another and disjoint from the task
pair set. Therefore the exact corrected definitions are proven: base task
runtime `296 occurrences / 133 unique`, base declared `298 / 135`; corrected
task runtime `320 / 160`, corrected declared `322 / 162`. Both graph variants
are fully resolved and acyclic.

The exact census command was:

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
    docs = []
    for name in selected:
        docs.append((name, tomllib.loads(subprocess.check_output(
            ["git", "show", f"{rev}:{name}"], text=True
        ))))
    return selected, docs

def scc(ids, edges):
    graph = {x: set() for x in ids}
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

for rev in [
    "bb5048f4c",
    "e84037ef00202ffa9a2932c5c98e7376c742b18b",
    "02b038a258439fb0aec56c643f2d27215b51aa36",
    "85d8e3253510f72553f5b3e15ae702b885f0d0b2",
]:
    paths, docs = manifests(rev)
    plans = {doc["meta"]["plan"]: doc for _, doc in docs}
    roots = [sum(path.startswith(prefix) for path in paths) for prefix in
             ("plans/", "tmp/status-quo/backlog/plans/",
              "tmp/status-quo/self-heal/plans/")]
    local_missing = []; plan_missing = []; task_refs = []; meta_refs = []
    statuses = {}
    for plan_id, doc in plans.items():
        task_ids = {task["id"] for task in doc.get("task", [])}
        for task in doc.get("task", []):
            statuses[task["status"]] = statuses.get(task["status"], 0) + 1
            local_missing += [(plan_id, task["id"], dep)
                              for dep in task.get("depends_on", [])
                              if dep not in task_ids]
            task_refs += [(plan_id, dep)
                          for dep in task.get("depends_on_plan", [])]
        meta_refs += [(plan_id, dep)
                      for dep in doc.get("meta", {}).get("depends_on_plan", [])]
    for source, target in task_refs + meta_refs:
        if target not in plans:
            plan_missing.append((source, target))
    task_edges = set(task_refs); meta_edges = set(meta_refs)
    all_edges = task_edges | meta_edges
    print("REV", rev)
    print("roots", roots, "plans", len(plans), "tasks", sum(statuses.values()),
          "statuses", dict(sorted(statuses.items())))
    print("task_refs", len(task_refs), "meta_refs", len(meta_refs),
          "task_unique", len(task_edges), "meta_unique", len(meta_edges),
          "all_unique", len(all_edges))
    print("meta_pairs", sorted(meta_edges), "disjoint",
          task_edges.isdisjoint(meta_edges))
    print("unresolved_local", len(local_missing), "unresolved_plan",
          len(plan_missing), "runtime_scc", len(scc(plans, task_edges)),
          "declared_scc", len(scc(plans, all_edges)))
PY
```

## Immutable behavior and repository controls

For the DOC-v2-core manifest and source-coverage ledger, `git rev-parse
"${rev}:<path>"` returned the same blobs at correction base, candidate, and
observed integration:

```text
tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml
2ade8fe5270df515b63445e7c40bab3dba465204
tmp/status-quo/backlog/source-coverage/docs-v2-core.md
3836f4c7f409a663018e7f6ef72be4f6e2dffbbf
```

Both explicit `git diff --exit-code e84037ef0..<revision> -- <two DOC paths>`
checks returned `0`, for candidate and observed integration. Thus this evidence
correction does not alter the already-integrated DOC manifest or coverage
behavior.

Final repository controls at the immutable candidate:

```text
$ git ls-files '*.toml' | wc -l
193
$ python3 - <<'PY'
from pathlib import Path
import subprocess, tomllib
paths = subprocess.check_output(
    ['git', 'ls-files', '*.toml'], text=True
).splitlines()
for path in paths:
    with Path(path).open('rb') as handle:
        tomllib.load(handle)
print(f'tracked_toml={len(paths)} errors=0')
PY
tracked_toml=193 errors=0
$ shasum -a 256 plans/INDEX.md
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44  plans/INDEX.md
$ git diff --check
# exit 0
```

No validator was run against the source tree. Before adding this review record,
the dedicated review worktree was clean. The candidate's scope, corrected
counts, historical supersession treatment, unchanged DOC blobs, parseability,
and index seal all satisfy the correction contract.
