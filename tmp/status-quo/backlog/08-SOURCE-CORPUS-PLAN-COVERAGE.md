# 08 — Source Corpus Plan Coverage

> Coverage ledger for the second executable-planning layer: every source document in
> `tmp/status-quo/*.md`, `docs/v1/**`, `docs/v2/**`, and `docs/v2-depth/**` is assigned to a
> roko-compatible DOC plan task.

## Summary

| Corpus | Source docs | Plan | Tasks | Ledger |
|---|---:|---|---:|---|
| `tmp/status-quo/*.md` | 108 | `plans/DOC-status-quo-corpus` | 12 | `source-coverage/status-quo-corpus.md` |
| `docs/v1` kernel slice | 113 | `plans/DOC-v1-kernel` | 8 | `source-coverage/docs-v1-kernel.md` |
| `docs/v1` cognition slice | 119 | `plans/DOC-v1-cognition` | 7 | `source-coverage/docs-v1-cognition.md` |
| `docs/v1` ecosystem slice | 185 | `plans/DOC-v1-ecosystem` | 10 | `source-coverage/docs-v1-ecosystem.md` |
| `docs/v2/**/*.md` | 34 | `plans/DOC-v2-core` | 10 | `source-coverage/docs-v2-core.md` |
| `docs/v2-depth/**/*.md` | 185 | `plans/DOC-v2-depth` | 24 | `source-coverage/docs-v2-depth.md` |
| **Total** | **744** | **6 DOC plans** | **71** | **6 ledgers** |

The DOC plans are not replacements for E01-E18. They are a source-corpus reconciliation layer:
each task tells a future agent to read a coherent cluster of source docs, map it to existing
E01-E18 work, add/refine downstream DOC follow-up tasks if something remains uncovered, or record
an explicit `mapped`, `doc-follow-up`, `deferred`, or `no-op` result in the source ledger.

## DAG Shape

- `DOC-status-quo-corpus` covers the 108-document audit/status pack and should run before broad
  source-doc reconciliation.
- `DOC-v1-kernel` covers root v1 docs plus architecture, orchestration, agent, composition, and
  verification specs.
- `DOC-v1-cognition` covers learning, neuro/knowledge, conductor, daimon, dreams, heartbeat, and
  lifecycle specs.
- `DOC-v1-ecosystem` covers chain, safety, interfaces, coordination, identity/economy,
  code-intelligence, tools, deployment, technical-analysis, and references.
- `DOC-v2-core` covers canonical v2 specs and public v2 guides.
- `DOC-v2-depth` covers deep v2 design packs and research prompts.

## Validation

Validate the DOC layer:

```sh
for d in tmp/status-quo/backlog/plans/DOC-*; do
  cargo run -q -p roko-cli --bin roko -- plan validate "$d" || exit 1
done
```

Verify that every source document is present in a coverage ledger and in at least one DOC task file:

```sh
python3 - <<'PY'
from pathlib import Path

sources = sorted(
    [str(p) for p in Path("tmp/status-quo").glob("*.md")]
    + [str(p) for root in ["docs/v1", "docs/v2", "docs/v2-depth"] for p in Path(root).glob("**/*.md")]
)
ledger_text = "\n".join(
    p.read_text(errors="replace")
    for p in Path("tmp/status-quo/backlog/source-coverage").glob("*.md")
)
plan_text = "\n".join(
    p.read_text(errors="replace")
    for p in Path("tmp/status-quo/backlog/plans").glob("DOC-*/tasks.toml")
)

missing_ledger = [p for p in sources if p not in ledger_text]
missing_plan = [p for p in sources if p not in plan_text]

print(f"sources={len(sources)}")
print(f"missing_from_ledgers={len(missing_ledger)}")
print(f"missing_from_doc_tasks={len(missing_plan)}")
if missing_ledger:
    print("\n".join(missing_ledger))
if missing_plan:
    print("\n".join(missing_plan))
raise SystemExit(1 if missing_ledger or missing_plan else 0)
PY
```

Current result:

```text
sources=744
missing_from_ledgers=0
missing_from_doc_tasks=0
```

## Execution

Run source-corpus reconciliation after the E01 bootstrap is live:

```sh
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-status-quo-corpus --engine runner-v2 --fresh --max-tasks 2
```

Then run the more speculative doc corpora in parallel slices as capacity allows:

```sh
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-v1-kernel --engine runner-v2 --max-tasks 2
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-v1-cognition --engine runner-v2 --max-tasks 2
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-v1-ecosystem --engine runner-v2 --max-tasks 2
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-v2-core --engine runner-v2 --max-tasks 2
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/DOC-v2-depth --engine runner-v2 --max-tasks 2
```
