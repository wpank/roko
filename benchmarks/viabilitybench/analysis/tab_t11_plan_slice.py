#!/usr/bin/env python3
"""T11: the plan-level slice: verified features, cost per verified feature and makespan per arm (§6.6; RQ3,
exploratory). Spec: paper/FIGURES-TABLES.md, T11.

    tab_t11_plan_slice.py BUNDLE/metrics.json --out DIR [--dry-run]

Reads only the `E-PL` MetricRecords that `report.py` emits for the slice (`pl_*`) and the per-feature rows their
`run_ids` name: the `plan_slice.table` of the bundle's `vb.metrics/1` document (gap-1cd676).
- Part a, one row per feature in run order: for each arm VF (1, 0, or unknown with the unknown = 1 bound beside it),
  cost (api_equiv_usd, the vendor's R beside it when the run has one), makespan with the queue wait beside it,
  status and run id. Modules touched and the plan's tasks and width are not in the per-feature table yet and print
  "not recorded".
- Part b, one row per arm: verified features with `pl_vf_rate`'s Clopper-Pearson 95% interval, CPF (`pl_cpf_usd`;
  R's `pl_cpf_vendor_usd` beside it), the planner's share (`pl_planner_share`), the median and range of the
  makespan (`pl_makespan_*_s`), then the roko_plan / fd_claude ratios as point estimates (`pl_cpf_ratio`,
  `pl_makespan_median_ratio`).
- Part c, roko_plan's process measures: `pl_realized_parallelism_median`, `pl_tasks_escalated_share`,
  `pl_integrations_rejected` and the plan-level false greens (the section's list).
- Footer: the discordant features both ways, the features both arms verified, |ℱ|, the seeds, the harness
  commits, the price snapshot, and what the section says is not recorded.
It carries no p-value, bootstrap, confidence sequence or Holm decision (S09 §4.9). Outside a dry run it also checks
the bundle: every E-PL run directory beside `metrics.json` must hold a manifest whose suite hash every record of
the run shares, and the runs must share one suite hash; a missing run directory refuses the table. Without E-PL
records the table is not drawn. figlib has the other refusal rules and the sidecar.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import figlib

SPEC = figlib.Spec("T11", "t11-plan-slice", "Plan-level slice (exploratory, one seed)", "tab_t11_plan_slice.py",
                   kind="table")
READS: dict[str, str] = {}  # every name T11 reads is one report.py emits (metrics.plan_slice)
EXPERIMENT = "E-PL"
RATIO = ("roko_plan", "fd_claude")


def build(inputs: figlib.Inputs) -> figlib.Table:
    table = figlib.Table(SPEC, inputs.dry_run)
    records = [rec for rec in inputs.records if rec.experiment == EXPERIMENT]
    if not records:
        raise figlib.FigureError(f"no {EXPERIMENT} MetricRecords: T11 is not drawn without its bundle")
    sections = inputs.sections.get("plan_slice", [])
    if len(sections) != 1:
        raise figlib.FigureError(f"T11 reads one bundle's plan_slice section; the inputs hold {len(sections)}")
    path, section = sections[0]
    suites = _check_suites(path, records) if not inputs.dry_run else None
    arms = sorted(section["arms"], key=lambda arm: (arm not in RATIO, RATIO.index(arm) if arm in RATIO else 0, arm))

    def one(metric: str, arm: str | None = None, against: tuple = ()) -> figlib.Rec | None:
        return inputs.one(metric, experiment=EXPERIMENT, arm=arm if arm else figlib.ANY, against=against)

    # Part b first: its records are the ones whose run ids name part a's rows.
    summary = []
    for arm in arms:
        vf = one("pl_verified_features", arm)
        rate = one("pl_vf_rate", arm)
        verified = table.cell(vf, part="b", row=arm, column="verified features", kind="count", with_ci=False)
        share = table.cell(rate, part="b", row=arm, column="verified / features (Clopper-Pearson 95%)")
        cpf = table.cell(one("pl_cpf_usd", arm), part="b", row=arm, column="CPF", kind="usd", with_ci=False)
        vendor = one("pl_cpf_vendor_usd", arm)
        if vendor is not None:
            cpf += f" (R: {table.cell(vendor, part='b', row=arm, column='CPF, R', kind='usd', with_ci=False)})"
        span = [table.cell(one(f"pl_makespan_{name}_s", arm), part="b", row=arm, column=f"makespan {name}",
                           kind="minutes", with_ci=False) for name in ("median", "min", "max")]
        summary.append([arm, f"{verified} of {rate.n if rate else '–'}", share, cpf,
                        table.cell(one("pl_planner_share", arm), part="b", row=arm, column="planner share",
                                   kind="pct", with_ci=False),
                        f"{span[0]} ({span[1]} to {span[2]})"])
    ratios = [f"{name.removeprefix('pl_').replace('_', ' ')} (roko_plan / fd_claude): "
              + table.cell(one(name, RATIO[0], (RATIO[1],)), part="b", row="ratios", column=name, with_ci=False)
              for name in ("pl_cpf_ratio", "pl_makespan_median_ratio")]

    # Part a: the per-feature rows the records name.
    columns = ["feature", "seed", "modules touched", "tasks (width)"]
    for arm in arms:
        columns += [f"{arm}: VF", f"{arm}: cost", f"{arm}: makespan (queue wait)", f"{arm}: status", f"{arm}: run"]
    features = []
    for row in section["table"]:
        line = [row["feature"], str(row["seed"]), "not recorded", "not recorded"]
        for arm in arms:
            cell = row["arms"].get(arm)
            if cell is None:
                line += ["NOT RUN"] * 5
                continue
            name = f"{row['feature']} {arm}"
            table.take_row("a", name, "VF", cell["vf"], cell, records)
            vf = "unknown (VF = 0; bound 1)" if cell["unknown"] else str(cell["vf"])
            cost = "unknown" if cell["cost_usd"] is None else figlib.fmt(cell["cost_usd"], "usd")
            if cell["vendor_usd"] is not None:
                cost += f" (R {figlib.fmt(cell['vendor_usd'], 'usd')})"
            span = "not recorded" if cell["makespan_s"] is None else figlib.fmt(cell["makespan_s"], "minutes")
            wait = "not recorded" if cell["queue_wait_s"] is None else figlib.fmt(cell["queue_wait_s"], "minutes")
            for role, value in (("cost", cell["cost_usd"]), ("vendor cost", cell["vendor_usd"]),
                                ("makespan s", cell["makespan_s"]), ("queue wait s", cell["queue_wait_s"])):
                if value is not None:
                    table.take_row("a", name, role, value, cell, records)
            line += [vf, cost, f"{span} ({wait})", cell["status"], cell["run_id"]]
        features.append(line)
    table.part(f"Part a: per feature ({len(section['table'])} features, in run order)", columns).extend(features)
    table.part("Part b: per arm", ("arm", "verified", "verified / features [CP 95%]", "CPF (U′)", "planner share",
                                    "makespan median (range)")).extend(summary)

    # Part c: roko_plan's process measures.
    process = [(label, table.cell(one(metric, "roko_plan"), part="c", row="roko_plan", column=label, kind=kind,
                                  with_ci=False))
               for label, metric, kind in (("realized parallelism (median)", "pl_realized_parallelism_median", "rate"),
                                           ("tasks escalated / tasks run", "pl_tasks_escalated_share", "pct"),
                                           ("integrations the whole-plan gate rejected although every task gate "
                                            "passed", "pl_integrations_rejected", "count"))]
    process.append(("total work / critical path", "not recorded"))
    false_greens = section.get("plan_level_false_greens", [])
    process.append(("plan-level false greens (whole-plan gate passed, hidden suite failed)",
                    f"{len(false_greens)}: {', '.join(false_greens)}" if false_greens else "0"))
    table.part("Part c: roko_plan's process measures", ("measure", "value")).extend([list(item) for item in process])

    seeds = sorted({seed for rec in records for seed in rec.doc["seeds"]})
    commits = sorted({commit for rec in records for commit in rec.doc["commits"]})
    snapshots = sorted({rec.doc["price_snapshot_id"] for rec in records})
    table.footer += ratios + [
        f"Discordant features (verified by one arm only): "
        + "; ".join(f"{arm}: {', '.join(found) or 'none'}" for arm, found in section.get("discordant", {}).items()),
        "Features both arms verified (where \"faster\" is counted): "
        + (", ".join(section.get("both_verified", [])) or "none") + ".",
        f"|ℱ| = {len(section['table'])}; seeds {', '.join(map(str, seeds))}; harness {', '.join(commits)}; prices "
        f"{', '.join(snapshots)}; the parallelism cap is recorded in each run's config_hash.",
        f"Not recorded: {section.get('not_recorded') or 'nothing'}.",
        "Exploratory, one seed: no p-value, bootstrap, confidence sequence or Holm decision (S09 §4.9).",
        "Suite hash: " + (f"one hash, {suites}, in every run's manifest and records." if suites else
                          "not checked in a dry run."),
    ]
    return table


def _check_suites(metrics_path: Path, records: list[figlib.Rec]) -> str:
    """The one suite hash of every E-PL run beside `metrics_path`; FigureError when a run directory is missing or
    a record's suite hash differs from its manifest's or another run's."""
    hashes = set()
    for run_id in sorted({run_id for rec in records for run_id in rec.doc["run_ids"]}):
        run_dir = metrics_path.parent / run_id
        try:
            manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
            rows = [json.loads(line) for line in (run_dir / "records.jsonl").read_text(encoding="utf-8").splitlines()
                    if line.strip()]
            suite = manifest["suite"]["hash"]
        except (OSError, ValueError, KeyError, TypeError) as err:
            raise figlib.FigureError(f"T11 checks each {EXPERIMENT} run's suite hash against its manifest, and run "
                                     f"{run_id} has no readable manifest and records beside {metrics_path.name} "
                                     f"({err})") from None
        differing = [row.get("record_id") for row in rows if (row.get("suite") or {}).get("hash") != suite]
        if differing:
            raise figlib.FigureError(f"run {run_id}: records {', '.join(map(str, differing))} have a suite hash other "
                                     f"than the manifest's {suite}")
        hashes.add(suite)
    if len(hashes) != 1:
        raise figlib.FigureError(f"the {EXPERIMENT} runs use {len(hashes)} suite hashes ({', '.join(sorted(hashes))});"
                                 " a fixture set must be one suite")
    return hashes.pop()


def main(argv: list[str] | None = None) -> int:
    return figlib.run(SPEC, build, argv)


if __name__ == "__main__":
    sys.exit(main())
