#!/usr/bin/env python3
"""`vb report`: an experiment's metrics file and the summary-bundle check (S08 §4.12, §5.5 and §5.7; T7).

    report.py --experiment PILOT-A [--results DIR] [--out metrics.json] [--bundle DIR] [--k 3[,5]]
    report.py --check BUNDLE [BUNDLE ...] [--budget FILE]

**Report.** It reads every run of one experiment, `<results>/<experiment>/<run_id>/{manifest.json,records.jsonl}`,
where results is `--results`, else `$VB_RESULTS`, else the driver's default `~/.roko-bench/viability`. It refuses to
report when a run record is invalid or simulated, when a run repeats, or when the records use more than one price
snapshot (S08 §4.12). It writes `metrics.json` (`--out`, by default in the experiment directory):

    {"schema_version": "vb.metrics/1", "experiment_id", "price_snapshot_id", "analysis_commit", "computed_at",
     "label", "records": [vb.metric_record/1, ...], "false_greens": [...], "excluded": [...], "plan_slice": {...}}

and prints each cell's table, every false green with its run id, and the excluded runs. `metrics.py` defines the
metrics and the cells: a cell is an arm, or an arm and a model when the arm ran more than one model, and such a
cell's rows name the model and its filters add `model == "<slug>"`. Every MetricRecord lists its run ids, seeds,
commits and config hashes, its exact `record_filter`, `label_source` "vs_census", a `cost_basis` (null for a metric
that is not a cost), and the experiment's one `price_snapshot_id`. The report is descriptive: `preregistered` is
false, `prereg_id` null and `blinded` false. It computes no confidence intervals (`ci_method` "none"): those come
with the pilot page (gap-d9e9fe). The exception is the plan-level slice's Clopper-Pearson interval, which S09 §4.9
asks for.

**Bundle.** `--bundle DIR` also writes the summary bundle that gets committed under `reports/` (D4). It holds
`metrics.json`, and for each run a `<run_id>/` directory with its `manifest.json`, `order-*.json`, `records.jsonl`,
`ledger.jsonl` and `errors.jsonl`. Nothing else is copied: never `private/` (task manifests with canaries, pristine
bundles), `archives/` or `transcripts/`.

**Check.** `--check` exits 0 only when every bundle passes all of these:
- every run record validates and none is simulated; every ledger row and MetricRecord validates;
- every run directory has a manifest, and its records are exactly the (instance, seed) runs the manifest planned,
  with none missing, none extra and none repeated;
- every attempt in the records has a ledger row, so no spend escapes the cap check;
- every MetricRecord has run ids, each naming a run in the bundle, and the records' one price snapshot;
- the false-green list is exactly the false greens in the records;
- the ledgers' spend, summed over all the bundles given, is within every cap of the budget file (`--budget`, default
  `experiments/budget.toml`, `vb.budget/1`): each line's (`[[line]]`, S09 §4.6), each experiment cap's
  (`[[experiment]]`, whose runs may book only to its lines) and the programme stop. A row's spend is its `billed_usd`
  (a subscription bills $0), or its reservation when that is unknown. A line without a cap fails.
Problems print one per line. Exit status: 0 when clean, 1 on a problem, 2 on a usage error.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

import metrics
from metrics import Metric

ANALYSIS_DIR = Path(__file__).resolve().parent
VB_ROOT = ANALYSIS_DIR.parent
sys.path.insert(0, str(VB_ROOT / "schema"))
import validate  # noqa: E402

METRICS_VERSION = "vb.metrics/1"
DEFAULT_RESULTS = Path("~/.roko-bench/viability")
DEFAULT_BUDGET = VB_ROOT / "experiments" / "budget.toml"
BUNDLE_FILES = ("manifest.json", "records.jsonl", "ledger.jsonl", "errors.jsonl")
LABEL = "descriptive, not pre-registered; no confidence intervals except the plan-level slice's"
LISTS = ("records", "false_greens", "excluded")  # written one item per line


class ReportError(RuntimeError):
    """The experiment cannot be reported: the message says why."""


@dataclass
class Run:
    """One run directory: its manifest, run records and ledger rows, each row with its file:line location."""

    path: Path
    manifest: dict | None = None
    records: list[tuple[str, dict]] = field(default_factory=list)
    ledger: list[tuple[str, dict]] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.check:
            problems = check(args.check, args.budget)
            for problem in problems:
                print(problem)
            print(f"report --check: {len(args.check)} bundle(s), {len(problems)} problem(s)", file=sys.stderr)
            return 1 if problems else 0
        results = Path(args.results or os.environ.get("VB_RESULTS") or DEFAULT_RESULTS).expanduser()
        experiment_dir = results / args.experiment
        runs = load_runs(experiment_dir)
        problems = [problem for run in runs for problem in run.problems]
        if problems:
            raise ReportError("invalid runs:\n  " + "\n  ".join(problems))
        for problem in (problem for run in runs for problem in _check_planned(run)):
            print(f"report: warning: {problem}", file=sys.stderr)  # --check refuses such a bundle
        records = [record for run in runs for _, record in run.records]
        report, found = build(records, args.experiment, ks=args.k, analysis_commit=analysis_commit(),
                              computed_at=dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"))
        if args.bundle:  # first: it refuses a directory that is not empty
            write_bundle(runs, Path(args.bundle), report)
        out = Path(args.out) if args.out else experiment_dir / "metrics.json"
        out.write_text(dumps(report), encoding="utf-8")
        print(render(report, found))
        print(f"report: {len(report['records'])} metric records in {out}", file=sys.stderr)
        return 0
    except (ReportError, metrics.MetricsError, OSError) as err:
        print(f"report: {err}", file=sys.stderr)
        return 2 if isinstance(err, OSError) else 1


def load_runs(directory: Path) -> list[Run]:
    """Every run directory in an experiment directory or a bundle: each subdirectory, with every problem found."""
    if not directory.is_dir():
        raise ReportError(f"no such directory: {directory}")
    runs = [load_run(path) for path in sorted(directory.iterdir()) if path.is_dir() and not path.name.startswith(".")]
    if not runs:
        raise ReportError(f"{directory} holds no run directories")
    return runs


def load_run(path: Path) -> Run:
    run = Run(path)
    try:
        run.manifest = json.loads((path / "manifest.json").read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        run.problems.append(f"{path}: no readable manifest.json ({err})")
    for name, kind, rows in (("records.jsonl", "run-record", run.records), ("ledger.jsonl", "ledger", run.ledger)):
        file = path / name
        if not file.exists():
            continue
        for number, line in enumerate(file.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip():
                continue
            where = f"{file}:{number}"
            try:
                doc = json.loads(line)
            except ValueError as err:
                run.problems.append(f"{where}: not JSON ({err})")
                continue
            if kind == "run-record" and isinstance(doc, dict) and doc.get("simulated") is not False:
                run.problems.append(f"{where}: simulated is {json.dumps(doc.get('simulated'))}; only real runs count")
            errors = validate.validate(kind, doc)
            run.problems += [f"{where}: {error}" for error in errors]
            if not errors:
                rows.append((where, doc))
    return run


def build(records: list[dict], experiment_id: str, *, ks: tuple[int, ...], analysis_commit: str,
          computed_at: str) -> tuple[dict, list[Metric]]:
    """The metrics.json document for one experiment's run records, and the metrics behind its MetricRecords."""
    others = sorted({record["experiment_id"] for record in records} - {experiment_id})
    if others:
        raise ReportError(f"records of other experiments ({', '.join(others)}) are mixed into {experiment_id}")
    metrics.check_unique(records)
    snapshot = single_snapshot(records)
    found = [metric for arm, model in metrics.cells(records)
             for metric in metrics.arm_metrics(records, experiment_id, arm, ks, model=model)]
    section, slice_metrics = metrics.plan_slice(records, experiment_id)
    found += slice_metrics
    report = {
        "schema_version": METRICS_VERSION, "experiment_id": experiment_id, "price_snapshot_id": snapshot,
        "analysis_commit": analysis_commit, "computed_at": computed_at, "label": LABEL,
        "records": [metric_record(metric, experiment_id=experiment_id, snapshot=snapshot,
                                  analysis_commit=analysis_commit, computed_at=computed_at) for metric in found],
        "false_greens": metrics.false_greens(records),
        "excluded": metrics.excluded(records),
        "plan_slice": section,
    }
    return report, found


def metric_record(metric: Metric, *, experiment_id: str, snapshot: str, analysis_commit: str,
                  computed_at: str) -> dict:
    """The `vb.metric_record/1` row for `metric`, with the provenance of the runs behind it."""
    rows = metric.cut.rows
    record = {"schema_version": "vb.metric_record/1", "metric": metric.metric, "value": metric.value}
    if metric.ci is not None:
        record["ci"] = list(metric.ci)
    record |= {
        "ci_method": metric.ci_method, "n": metric.n, "estimator": metric.estimator, "experiment_id": experiment_id,
        "arms": _distinct(rows, "arm"), "record_filter": metric.cut.filter, "run_ids": _distinct(rows, "run_id"),
        "seeds": _distinct(rows, "seed"), "commits": _distinct(rows, "harness_sha"),
        "config_hashes": _distinct(rows, "config_hash"), "analysis_commit": analysis_commit,
        "computed_at": computed_at, "preregistered": False, "prereg_id": None, "blinded": False,
        "label_source": "vs_census", "cost_basis": metric.cost_basis, "price_snapshot_id": snapshot,
    }
    if metric.ladder is not None:
        record["ladder"] = metric.ladder
    return record


def single_snapshot(records: list[dict]) -> str:
    """S08 §4.12: every cost of an experiment comes from one price snapshot."""
    snapshots = sorted({record["price_snapshot_id"] for record in records})
    if len(snapshots) != 1:
        raise ReportError(f"the records use {len(snapshots)} price snapshots ({', '.join(snapshots) or 'none'}); an "
                          "experiment is priced from exactly one")
    return snapshots[0]


def write_bundle(runs: list[Run], bundle: Path, report: dict) -> None:
    """Copy each run's summary files and write metrics.json: the directory committed under reports/."""
    if bundle.exists() and any(bundle.iterdir()):
        raise ReportError(f"the bundle directory {bundle} is not empty; remove it first")
    for run in runs:
        target = bundle / run.path.name
        target.mkdir(parents=True)
        for source in [run.path / name for name in BUNDLE_FILES] + sorted(run.path.glob("order-*.json")):
            if source.is_file():
                shutil.copyfile(source, target / source.name)
    (bundle / "metrics.json").write_text(dumps(report), encoding="utf-8")


@dataclass(frozen=True)
class Budget:
    """The caps of a `vb.budget/1` file, in USD billed."""

    lines: dict[str, float]  # line id -> cap
    experiments: list[tuple[str, frozenset[str], frozenset[str], float]]  # (id, experiment ids, lines, cap)
    stop_usd: float | None  # the programme stop


def check(bundles: list[Path], budget: Path) -> list[str]:
    """Every problem with the given bundles; empty when they may be published (the module docstring has the rules)."""
    problems: list[str] = []
    spend: dict[tuple[str, str], tuple[str, str, float]] = {}  # (run_id, attempt_key) -> (line, experiment, spend)
    for bundle in bundles:
        problems += check_bundle(Path(bundle), spend)
    return problems + check_caps(spend, budget)


def check_bundle(bundle: Path, spend: dict[tuple[str, str], tuple[str, str, float]]) -> list[str]:
    try:
        runs = load_runs(bundle)
    except ReportError as err:
        return [f"{bundle}: {err}"]
    problems = [problem for run in runs for problem in run.problems]
    records = [record for run in runs for _, record in run.records]
    for run in runs:
        problems += _check_planned(run)
        attempts = {attempt["attempt_key"] for _, record in run.records for attempt in record["execution"]["attempts"]
                    if "attempt_key" in attempt}
        unledgered = attempts - {row["attempt_key"] for _, row in run.ledger}
        problems += [f"{run.path / 'ledger.jsonl'}: no row for attempt {key}" for key in sorted(unledgered)]
        for where, row in run.ledger:
            if (row["experiment_id"], row["run_id"]) != _run_identity(run):
                problems.append(f"{where}: a ledger row of {row['experiment_id']}/{row['run_id']} in {run.path.name}")
            amount = row["billed_usd"] if row["billed_usd"] is not None else row["reserved_usd"]
            spend[(row["run_id"], row["attempt_key"])] = (row["line"], row["experiment_id"], amount)
    try:
        metrics.check_unique(records)
        snapshot = single_snapshot(records) if records else None
    except (metrics.MetricsError, ReportError) as err:
        problems.append(f"{bundle}: {err}")
        snapshot = None

    path = bundle / "metrics.json"
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        return problems + [f"{path}: no readable metrics file ({err})"]
    if not isinstance(report, dict) or report.get("schema_version") != METRICS_VERSION or not all(
            isinstance(report.get(key), list) for key in LISTS):
        return problems + [f"{path}: not a {METRICS_VERSION} document with {', '.join(LISTS)} lists"]
    run_ids = {record["run_id"] for record in records}
    experiments = {record["experiment_id"] for record in records}
    for index, row in enumerate(report["records"]):
        where = f"{path}: records[{index}]"
        errors = validate.validate("metric-record", row)
        problems += [f"{where}: {error}" for error in errors]
        if errors:
            continue
        unknown = sorted(set(row["run_ids"]) - run_ids)
        if unknown:
            problems.append(f"{where} ({row['metric']}): run ids not in the bundle: {', '.join(unknown)}")
        if row["experiment_id"] not in experiments:
            problems.append(f"{where} ({row['metric']}): experiment {row['experiment_id']} has no runs in the bundle")
        if snapshot and row["price_snapshot_id"] != snapshot:
            problems.append(f"{where} ({row['metric']}): priced from {row['price_snapshot_id']}, not {snapshot}")
    listed = {item.get("record_id") for item in report["false_greens"] if isinstance(item, dict)}
    try:
        actual = {item["record_id"] for item in metrics.false_greens(records)}
    except metrics.MetricsError:  # a run that fits no cell, already reported above
        return problems
    problems += [f"{path}: false green {record_id} is not listed" for record_id in sorted(actual - listed)]
    problems += [f"{path}: listed false green {record_id} is not one" for record_id in sorted(listed - actual, key=str)]
    return problems


def check_caps(spend: dict[tuple[str, str], tuple[str, str, float]], budget: Path) -> list[str]:
    """The spend against every cap in the budget file; `spend` holds one entry per (run_id, attempt_key)."""
    if not spend:
        return []
    try:
        caps = load_budget(budget)
    except (OSError, ValueError) as err:
        return [f"{budget}: cannot read the caps ({err})"]
    problems = []
    lines: dict[str, float] = {}
    for line, _, amount in spend.values():
        lines[line] = lines.get(line, 0.0) + amount
    for line, total in sorted(lines.items()):
        if line not in caps.lines:
            problems.append(f"{budget}: budget line {line} has spend (${total:.4f}) but no cap")
        elif total > caps.lines[line] + 1e-9:
            problems.append(f"budget line {line}: the ledgers spent ${total:.4f}, over its cap of "
                            f"${caps.lines[line]:.2f}")
    for name, experiment_ids, allowed, cap in caps.experiments:
        booked = [(line, amount) for line, experiment, amount in spend.values() if experiment in experiment_ids]
        total = math.fsum(amount for _, amount in booked)
        if total > cap + 1e-9:
            problems.append(f"experiment cap {name}: the ledgers spent ${total:.4f}, over its cap of ${cap:.2f}")
        for line in sorted({line for line, _ in booked} - allowed):
            problems.append(f"experiment cap {name}: its runs booked to line {line}, outside its lines")
    total = math.fsum(amount for _, _, amount in spend.values())
    if caps.stop_usd is not None and total > caps.stop_usd + 1e-9:
        problems.append(f"the ledgers spent ${total:.4f}, past the programme stop of ${caps.stop_usd:.2f}")
    return problems


def load_budget(path: Path) -> Budget:
    """The caps of a `vb.budget/1` file: `[[line]]` id and cap_usd, `[[experiment]]` and `[programme] stop_usd`."""
    with Path(path).open("rb") as handle:
        doc = tomllib.load(handle)
    tables = {key: doc.get(key, []) for key in ("line", "experiment")}
    if not tables["line"] or not all(isinstance(tables[key], list) and all(
            isinstance(table, dict) and isinstance(table.get("id"), str) for table in tables[key]) for key in tables):
        raise ValueError("it needs [[line]] tables and may have [[experiment]] tables, each with an id")
    lines = {table["id"]: _dollars(table.get("cap_usd"), f"[[line]] {table['id']}") for table in tables["line"]}
    experiments = [(table["id"], frozenset(table.get("experiment_ids", [])), frozenset(table.get("lines", [])),
                    _dollars(table.get("cap_usd"), f"[[experiment]] {table['id']}")) for table in tables["experiment"]]
    stop = (doc.get("programme") or {}).get("stop_usd")
    return Budget(lines, experiments, None if stop is None else _dollars(stop, "[programme] stop_usd"))


def analysis_commit() -> str:
    """HEAD of the repository holding this code, with "-dirty" when the analysis directory has changes."""
    try:
        git = ["git", "-C", str(ANALYSIS_DIR)]
        sha = subprocess.run([*git, "rev-parse", "HEAD"], capture_output=True, text=True, timeout=30,
                             check=True).stdout.strip()
        dirty = subprocess.run([*git, "status", "--porcelain", "--", "."], capture_output=True, text=True, timeout=60,
                               check=True).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return "unknown"
    return f"{sha}-dirty" if dirty else sha


def dumps(report: dict) -> str:
    """metrics.json's text: one line per MetricRecord, false green and excluded run, so a diff shows what changed."""
    parts = []
    for key, value in report.items():
        if key in LISTS and value:
            items = ",\n".join(f"    {json.dumps(item, ensure_ascii=False)}" for item in value)
            parts.append(f"  {json.dumps(key)}: [\n{items}\n  ]")
        else:
            text = json.dumps(value, ensure_ascii=False, indent=2).replace("\n", "\n  ")
            parts.append(f"  {json.dumps(key)}: {text}")
    return "{\n" + ",\n".join(parts) + "\n}\n"


def render(report: dict, found: list[Metric]) -> str:
    """The printed report: a table per arm, or per arm and model (overall and per level), the false greens and the
    excluded runs."""
    cells: dict[tuple[str, str], dict[tuple[str, str | None], Metric]] = {}  # (arm or "arm (model)", cell)
    for metric in found:
        if metric.cell == "all" or metric.cell[0] == "l":
            name = metrics.cell_name(metric.cut.rows[0]["arm"], metric.model)
            cells.setdefault((name, metric.cell), {})[(metric.metric, metric.cost_basis)] = metric
    ks = sorted({int(m.metric.split("_")[2]) for m in found if m.metric.startswith("pass_hat_")})
    lines = [f"ViabilityBench report: experiment {report['experiment_id']} ({report['label']}; "
             f"{report['price_snapshot_id']})"]
    for arm in sorted({name for name, _ in cells}):
        runs = cells[(arm, "all")][("infra_error_runs", None)].cut.rows  # every run of the cell, excluded ones too
        lines += ["", f"{arm}: {len(runs)} runs in {', '.join(_distinct(runs, 'run_id'))}",
                  "| cell | runs kept | VS rate | VS, unknown = 1 | " + "".join(f"pass^{k} | " for k in ks)
                  + "$/VS | spend | false greens | FG rate | cap-censored | infra_error | leak_suspected |",
                  "|---" * (11 + len(ks)) + "|"]
        for cell in sorted((c for a, c in cells if a == arm), key=lambda c: (c != "all", c)):
            got = cells[(arm, cell)]

            def show(name: str, basis: str | None = None, money: bool = False) -> str:
                metric = got.get((name, basis))
                if metric is None or metric.value is None:
                    return "-"
                return f"${metric.value:.4f}" if money else f"{metric.value:.3f}" if isinstance(
                    metric.value, float) else str(metric.value)

            def n(name: str) -> int:
                return got[(name, None)].n if (name, None) in got else 0

            lines.append(f"| {cell} | {n('cap_censored_runs')} | {show('vs_rate')} | {show('vs_rate_unknown_as_1')} | "
                         + "".join(f"{show(f'pass_hat_{k}')} | " for k in ks)
                         + f"{show('usd_per_vs', 'api_equiv_usd', True)} | {show('spend_usd', 'api_equiv_usd', True)}"
                         f" | {show('false_greens')} of {n('false_green_rate')} passes | {show('false_green_rate')} | "
                         f"{show('cap_censored_runs')} | {show('infra_error_runs')} | {show('leak_suspected_runs')} |")
    for title, key in (("False greens", "false_greens"), ("Excluded runs", "excluded")):
        lines += ["", f"{title} ({len(report[key])}):"]
        lines += [f"- {metrics.cell_name(item['arm'], item.get('model'))} {item['instance_id']} seed {item['seed']} "
                  f"({item['status']}): run {item['run_id']}, record {item['record_id']}; failed: "
                  f"{', '.join(item['failed']) or '-'}" for item in report[key]]
    section = report["plan_slice"]
    if section:
        lines += ["", f"Plan-level slice, {section['label']}:"]
        for arm, got in section["arms"].items():
            median, wait = got["makespan_median_s"], got["queue_wait_median_s"]
            lines.append(f"- {arm}: {got['verified']} of {got['features']} features verified (95% "
                         f"{got['verified_ci95'][0]:.2f}-{got['verified_ci95'][1]:.2f}); cost per verified feature "
                         + (f"${got['cpf_usd']:.4f}" if got["cpf_usd"] is not None else got["cpf_note"])
                         + "; median makespan " + (f"{median:.0f} s" if median is not None else "not recorded")
                         + "; median queue wait " + (f"{wait:.0f} s ({got['queue_wait_recorded']} of "
                                                     f"{got['features']} features)" if wait is not None
                                                     else "not recorded"))
            if got.get("process"):
                lines.append(f"  process: {_process_text(got['process'])}")
        lines += [f"- ratios (roko_plan / fd_claude, point estimates): {section['ratios']}" if section["ratios"] else
                  "- ratios: need both roko_plan and fd_claude", f"- discordant: {section['discordant']}",
                  "- per feature:"]
        for row in section["table"]:
            for arm, cell in ((arm, cell) for arm, cell in row["arms"].items() if cell is not None):
                cost, span, wait = cell["cost_usd"], cell["makespan_s"], cell["queue_wait_s"]
                lines.append(f"  - {row['feature']} {arm}: VF {cell['vf']}, cost "
                             + (f"${cost:.4f}" if cost is not None else "unknown") + ", makespan "
                             + (f"{span:.0f} s" if span is not None else "not recorded") + ", queue wait "
                             + (f"{wait:.0f} s" if wait is not None else "not recorded") + f", run {cell['run_id']}")
        lines.append(f"- not recorded: {section['not_recorded']}")
    return "\n".join(lines)


def _process_text(process: dict) -> str:
    """S09 §4.9's process measures of a Roko arm, as printed; a measure a feature did not record is "not recorded"."""
    shown = {"planner_share": "planner's share of the cost {:.1%}", "realized_parallelism_median":
             "realized parallelism {:.2f} (median)", "tasks_escalated_share": "tasks escalated {:.0%}",
             "integrations_rejected": "{} integrated features rejected by the whole-plan gate"}
    return "; ".join(text.format(process[name]) if process[name] is not None else
                     f"{name.replace('_', ' ')} not recorded" for name, text in shown.items())


def _check_planned(run: Run) -> list[str]:
    """The run's records against its manifest: the same run, and exactly the planned (instance, seed) runs."""
    manifest = run.manifest
    if not isinstance(manifest, dict):
        return []  # already reported by load_run
    where = run.path / "manifest.json"
    problems = []
    if manifest.get("run_id") != run.path.name:
        problems.append(f"{where}: run_id {manifest.get('run_id')!r} does not name its directory {run.path.name}")
    for key in ("experiment_id", "run_id", "arm", "price_snapshot_id", "config_hash"):
        if key in manifest:
            problems += [f"{loc}: {key} {record[key]!r}, but the manifest says {manifest[key]!r}"
                         for loc, record in run.records if record[key] != manifest[key]]
    done = [(record["task"]["instance_id"], record["seed"]) for _, record in run.records]
    instances = (manifest.get("config") or {}).get("stream", {}).get("instances")
    if isinstance(instances, list) and isinstance(manifest.get("seeds"), list):
        planned = {(instance, seed) for instance in instances for seed in manifest["seeds"]}
        problems += [f"{where}: no record for {instance} seed {seed}" for instance, seed in sorted(planned - set(done))]
        problems += [f"{where}: {instance} seed {seed} was not planned" for instance, seed in sorted(set(done) - planned)]
    elif not isinstance(manifest.get("runs"), int):
        problems.append(f"{where}: the manifest lists neither its instances and seeds nor its number of runs")
    repeats = sorted({key for key in done if done.count(key) > 1})
    problems += [f"{where}: {instance} seed {seed} has more than one record" for instance, seed in repeats]
    if isinstance(manifest.get("runs"), int) and manifest["runs"] != len(done):
        problems.append(f"{where}: the manifest planned {manifest['runs']} runs; the bundle has {len(done)} records")
    return problems


def _run_identity(run: Run) -> tuple[object, object]:
    manifest = run.manifest if isinstance(run.manifest, dict) else {}
    return manifest.get("experiment_id"), manifest.get("run_id", run.path.name)


def _distinct(rows: tuple[dict, ...] | list[dict], key: str) -> list:
    return sorted({row[key] for row in rows})


def _dollars(value: object, where: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
        raise ValueError(f"{where} needs a cap_usd of $0 or more, not {value!r}")
    return float(value)


def _ks(text: str) -> tuple[int, ...]:
    try:
        ks = tuple(sorted({int(part) for part in text.split(",")}))
    except ValueError:
        raise argparse.ArgumentTypeError("use forms like 3 or 3,5") from None
    if not ks or ks[0] < 1:
        raise argparse.ArgumentTypeError("every k must be at least 1")
    return ks


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vb report", description="ViabilityBench metrics and bundle checks (S08 "
                                     "§5.7).", allow_abbrev=False)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--experiment", help="report this experiment's runs")
    mode.add_argument("--check", nargs="+", type=Path, metavar="BUNDLE", help="check summary bundles")
    parser.add_argument("--results", help="default: $VB_RESULTS, then " + str(DEFAULT_RESULTS))
    parser.add_argument("--out", help="where metrics.json goes (default: the experiment directory)")
    parser.add_argument("--bundle", help="also write the summary bundle to this new directory")
    parser.add_argument("--k", type=_ks, default=(3,), help="pass^k for these k, e.g. 3 or 3,5 (default 3)")
    parser.add_argument("--budget", type=Path, default=DEFAULT_BUDGET, help=f"line caps (default: {DEFAULT_BUDGET})")
    return parser


if __name__ == "__main__":
    sys.exit(main())
