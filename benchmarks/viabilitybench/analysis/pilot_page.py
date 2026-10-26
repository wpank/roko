"""`vb report --pilot`: the pilot's one page, every estimate with its 95% interval and run ids (gap-d9e9fe).

    report.py --pilot BUNDLE [BUNDLE ...] --bundle OUT [--superseded BUNDLE ...] [--note TEXT ...]
              [--experiments DIR] [--verifier-ci FILE] [--b 10000] [--seed 0] [--k 3] [--budget FILE]

It turns the pilots' committed summary bundles (`reports/pilot_a_recal/`, `reports/pilot_b/`; S09 §0's first slice,
E1a and E1b) into one bundle, `OUT` (D4: `reports/pilot/`): the merged runs, `metrics.json` with a MetricRecord for
every number on the page, the verifier-CI record, and `REPORT.md`, the page itself. `report.py --check OUT` accepts it.

**Merge.** The bundles given to `--pilot` must pass `--check` together, and their records must share one price
snapshot and one suite hash: the runs are paired by (task, seed), which is only meaningful when every arm saw the same
task versions. A bundle on another suite (Pilot A before the level re-tune) goes to `--superseded`: the page lists its
runs and why they are not merged, and no number uses them. No (arm, task, seed) may repeat across bundles.

**The design.** For each merged experiment, its manifest (`vb.experiment/1`, the file in `--experiments` whose `id`
matches) says what was planned: a block's arm file, model, seeds and instances (the block's own, else its stream's). A
run belongs to the block with its experiment, arm id, harness marker (`metrics.harness_of`: "" for the arm's own
harness), pinned model and disturbance (a run with `config.disturbances` belongs only to a block with a
`disturbance`). The page lists every planned (instance, seed) that has no record as pending, by block and seed, and
the arms of pending blocks stay on the page as pending rows, never as numbers. Without a manifest an experiment's
pending runs are unknown, and the page says so.

**Numbers.** Per arm cell (`metrics.cells`, per experiment), overall and at each level: the VS rate with the
unknown = 1 bound, pass^k (`--k`, default 3), $/VS from U' (`costs.api_equiv_usd`), and for an arm with a vendor
figure $/VS from R (`costs.vendor_usd`) and |U' - R|/R beside it; the false-green rate (false greens / reported
passes); and the tallies: false greens, cap censoring, `infra_error` and `leak_suspected` runs, API-equivalent and
billed spend. The estimates are `metrics.py`'s own values (the page refuses to render if a recomputed estimate
differs). Then, for each pair in `PAIRS` whose arms both have runs, on the (task, seed) pairs both arms kept: the
$/VS ratio a / b and the differences of VS rate and pass^k, a - b. A pair with a pending arm is shown as pending.

**Intervals.** Every estimate gets a 95% interval from `bootstrap.paired_bootstrap`, S09 §4.1's method as written:
strata family x level, tasks resampled within strata, then seeds within tasks, B = 10,000 (`--b`), seed `--seed`;
percentile intervals for rates and differences, BCa for $/VS, the cost ratios and |U' - R|/R. BCa falls back to the
percentile interval where it is undefined (a leave-one-task-out estimate with no VS, or a jackknife that cannot tell
the tasks apart), and the page marks those numbers. The decision dec-3d5714 is open: in simulation this two-stage
bootstrap over-covers (VS-rate coverage 0.975), so these intervals are likely conservative. The page says so; the
method is not changed here.

The bootstrap hands a statistic one flat bag of rows. Here a row is a unit, one (task, seed) with every arm's runs at
it, so each drawn seed is one unit and each drawn task is `n_t` consecutive units (its seeds). The statistics read
the bag back as task draws (`_draws`), so a task drawn twice counts twice, as two tasks: what a per-task, non-linear
statistic such as pass^k needs, and what grouping rows by task id would lose (econ.py's `_cc_ci` describes the
same trap). A replicate's pass^k scores the drawn seeds as runs, so a task that passed 2 of 3 seeds can score 1 in a
replicate. Undefined replicates follow envelope.py's convention, the value least favourable to the arm: $/VS and a
cost ratio with no VS count as unbounded (+inf), a false-green rate with no reported pass as 1; the page counts them.
An interval that reaches an unbounded replicate is shown open, `[low, inf)`, and its MetricRecord has no `ci` (JSON
has no infinity) but says so in its `estimator`.

**Not intervals.** Counts (false greens, censored and excluded runs) and spend totals are tallies of the runs listed,
not estimates. They carry run ids and no interval.

**Output.** `OUT` must not exist or be empty. It gets each merged run's summary files (`report.write_bundle`),
`metrics.json` (`vb.metrics/1`; its `experiment_id` is the merged pilot's, `PILOT_ID`, and `experiments` lists the
merged experiments; a per-arm record keeps its own experiment, a contrast record across experiments takes
`PILOT_ID`, which `--check` accepts), the verifier-CI record the sources hold (`verifier-ci.json`, or
`--verifier-ci`), and `REPORT.md`, labelled "pilot, descriptive: ..., not pre-registered", with D3's bar shown for
context only: the pilot does not test H1.

API:
    PilotError(report.ReportError)
    Cell(experiment_id, arm, model, harness); Block(...); Interval(...)
    run(args, argv) -> int                                     # report.py --pilot
    build(sources, *, superseded=(), notes=(), experiments_dir, verifier_ci=None, k=3, b=10_000, seed=0,
          budget, analysis_commit, computed_at, argv=()) -> Page    # the merged document and the page, unwritten
    write(page, out) -> None
    interval(units, statistic, *, ratio=False, undefined=None, b, seed, alpha=0.05) -> Interval
"""

from __future__ import annotations

import datetime as dt
import json
import math
import os
import shlex
import shutil
import sys
import tomllib
from collections import Counter
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass, field, replace
from pathlib import Path

import bootstrap
import envelope
import metrics
import report

for _extra in (report.VB_ROOT / "driver",):
    if str(_extra) not in sys.path:
        sys.path.insert(0, str(_extra))

import campaign  # noqa: E402 (driver/campaign.py: experiment manifests)

PILOT_ID = "PILOT"
REPO_ROOT = report.VB_ROOT.parents[1]
ARMS_DIR = report.VB_ROOT / "arms"
VERIFIER_CI = "verifier-ci.json"
G0_FILE = "g0.json"
SIMULATION = report.VB_ROOT / "reports" / "simulation" / "simulate-planted-0.15.json"  # task 3339's coverage run
PAGE = "REPORT.md"
ALPHA = 0.05
# The contrasts (a, b): the cost ratios a / fd_claude the item asks for, and the harness effect, Roko against the
# direct loop on the same model (W10: a null harness effect is possible, and is reported as such).
PAIRS = (("cheap_direct", "fd_claude"), ("roko_fixed", "fd_claude"), ("roko_fixed", "cheap_direct"))
# The families' language, for the label ("two Python families": W10 §4; the item). A family not listed here gets no
# language word.
FAMILY_LANGUAGE = {"F1": "Python", "F4": "Python"}
NUMBERS = {1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six", 7: "seven", 8: "eight"}
DECISION = "dec-3d5714"
UNBOUNDED = math.inf  # a replicate's $/VS or cost ratio with no VS: the least favourable value
NO_PASS = 1.0  # a replicate's false-green rate with no reported pass: the least favourable value


class PilotError(report.ReportError):
    """The bundles cannot make an honest pilot page: the message says why."""


@dataclass(frozen=True)
class Cell:
    """An arm cell of one experiment, as `metrics.cells` defines it."""

    experiment_id: str
    arm: str
    model: str | None
    harness: str

    @property
    def name(self) -> str:
        return metrics.cell_name(self.arm, self.model, self.harness)


@dataclass(frozen=True)
class Block:
    """A planned block of an experiment manifest, with the arm id and harness marker its runs record."""

    experiment_id: str
    id: str
    arm: str
    harness: str
    model: str
    seeds: tuple[int, ...]
    instances: tuple[str, ...]
    optional: bool
    disturbed: bool

    @property
    def planned(self) -> set[tuple[str, int]]:
        return {(instance, seed) for instance in self.instances for seed in self.seeds}


@dataclass(frozen=True)
class Interval:
    """An estimate and its 95% interval. `low`/`high` are None without an estimate; `high` may be +inf."""

    estimate: float | None
    low: float | None = None
    high: float | None = None
    method: str = "none"  # "percentile" or "bca"; "none" without an estimate
    fallback: bool = False  # BCa was asked for and the percentile interval stands in
    undefined: int = 0  # replicates whose statistic was undefined and took the convention's value
    convention: str = ""  # what an undefined replicate counts as
    why: str = ""  # why there is no estimate
    b: int = 0


@dataclass
class Coverage:
    """A block's planned (instance, seed) runs against the records in the bundles."""

    block: Block | None
    cells: list[Cell] = field(default_factory=list)
    run_ids: list[str] = field(default_factory=list)
    done: dict[tuple[str, int], str] = field(default_factory=dict)  # (instance, seed) -> status

    @property
    def pending(self) -> list[tuple[str, int]]:
        return sorted(self.block.planned - set(self.done)) if self.block else []


@dataclass
class Page:
    doc: dict  # metrics.json
    markdown: str  # REPORT.md
    runs: list[report.Run]
    verifier_ci: Path | None


# --------------------------------------------------------------------------- the command


def run(args, argv: Sequence[str]) -> int:
    """`report.py --pilot`: build the page from the bundles and write the bundle (module docstring)."""
    if not args.bundle:
        raise PilotError("--pilot needs --bundle OUT, the directory the merged bundle and its page go to")
    out = Path(args.bundle)
    if out.exists() and any(out.iterdir()):
        raise PilotError(f"the bundle directory {out} is not empty; remove it first")
    if args.b < 1:
        raise PilotError(f"--b must be at least 1, not {args.b}")
    page = build(args.pilot, superseded=args.superseded or (), notes=args.note or (),
                 experiments_dir=args.experiments or campaign.EXPERIMENTS_DIR, verifier_ci=args.verifier_ci,
                 k=args.k[0], b=args.b,
                 seed=args.seed, budget=args.budget, analysis_commit=report.analysis_commit(),
                 computed_at=dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ"), argv=argv)
    write(page, out)
    print(page.markdown)
    print(f"report --pilot: {len(page.doc['records'])} metric records and {PAGE} in {out}", file=sys.stderr)
    return 0


def write(page: Page, out: Path) -> None:
    report.write_bundle(page.runs, out, page.doc)
    if page.verifier_ci is not None:
        shutil.copyfile(page.verifier_ci, out / VERIFIER_CI)
    (out / PAGE).write_text(page.markdown, encoding="utf-8")


def build(sources: Sequence[Path], *, superseded: Sequence[Path] = (), notes: Sequence[str] = (),
          experiments_dir: Path = campaign.EXPERIMENTS_DIR, verifier_ci: Path | None = None, k: int = 3,
          b: int = bootstrap.DEFAULT_B, seed: int = 0, budget: Path = report.DEFAULT_BUDGET,
          analysis_commit: str = "unknown", computed_at: str = "", argv: Sequence[str] = ()) -> Page:
    """The merged metrics.json and the page, from the source bundles (module docstring); nothing is written."""
    sources = [Path(source) for source in sources]
    runs, records = _merge(sources, budget)
    experiments = list(dict.fromkeys(record["experiment_id"] for record in records))
    snapshot = report.single_snapshot(records)
    rows = metrics.with_models(records)
    cells = [Cell(experiment, arm, model, harness) for experiment in experiments
             for arm, model, harness in metrics.cells([row for row in records if row["experiment_id"] == experiment])]
    coverage, unplanned, no_manifest = _coverage(runs, records, experiments, Path(experiments_dir), cells)
    kept = {cell: _kept(rows, cell) for cell in cells}
    found: list[metrics.Metric] = []
    arm_rows = []
    for cell in cells:
        exp_records = [row for row in records if row["experiment_id"] == cell.experiment_id]
        by_metric = {(m.metric, m.cell, m.cost_basis): m for m in metrics.arm_metrics(
            exp_records, cell.experiment_id, cell.arm, (k,), model=cell.model, harness=cell.harness)}
        levels = sorted({row["task"]["ladder"] for row in kept[cell]})
        arm_rows.append({"cell": cell, "levels": {}})
        for level in [None, *levels]:
            got = _cell_numbers(cell, kept[cell], by_metric, level, k, b, seed)
            arm_rows[-1]["levels"][level] = got
            found += [metric for metric, _ in got["metrics"]]
    contrasts = []
    for a, b_arm in PAIRS:
        contrast = _contrast(a, b_arm, cells, kept, coverage, k, b, seed)
        if contrast is not None:
            contrasts.append(contrast)
            found += [metric for metric, _ in contrast["metrics"]]
    false_greens = metrics.false_greens(records)
    ci_path, ci_doc = _verifier_ci(sources, verifier_ci)
    g0 = _g0(sources, experiments)
    gone = [_superseded(Path(path), records) for path in superseded]
    label = _label(records)
    pilot = {
        "label": label, "experiments": experiments, "sources": [_show(path) for path in sources],
        "suite": records[0]["suite"]["hash"], "generators": _generators(records),
        "bootstrap": {"method": "paired, strata family x level; tasks within strata, then seeds within tasks "
                                "(S09 §4.1 as written)", "b": b, "seed": seed, "alpha": ALPHA,
                      "percentile": "rates and differences", "bca": "$/VS, cost ratios, |U'-R|/R",
                      "decision": f"{DECISION} (open): the two-stage bootstrap over-covers in simulation"},
        "design": [_coverage_json(item) for item in coverage],
        "unplanned_runs": unplanned, "no_manifest": no_manifest, "superseded": gone, "notes": list(notes),
        "verifier_ci": _ci_summary(ci_doc, records) if ci_doc else None, "g0": g0,
    }
    doc = {
        "schema_version": report.METRICS_VERSION, "experiment_id": PILOT_ID, "experiments": experiments,
        "price_snapshot_id": snapshot, "analysis_commit": analysis_commit, "computed_at": computed_at,
        "label": label,
        "records": [report.metric_record(metric, experiment_id=_experiment_of(metric), snapshot=snapshot,
                                         analysis_commit=analysis_commit, computed_at=computed_at) for metric in found],
        "false_greens": false_greens, "excluded": metrics.excluded(records), "plan_slice": None, "pilot": pilot,
    }
    _bound_estimators(doc["records"], found)
    markdown = _render(doc, arm_rows, contrasts, coverage, records, k, b, seed, argv)
    return Page(doc=doc, markdown=markdown, runs=runs, verifier_ci=ci_path)


# --------------------------------------------------------------------------- merging and the design


def _merge(sources: Sequence[Path], budget: Path) -> tuple[list[report.Run], list[dict]]:
    """The runs and records of the source bundles, refused unless they pass --check together, share one suite and
    one price snapshot, and repeat no run (module docstring)."""
    if not sources:
        raise PilotError("--pilot needs at least one bundle")
    problems = report.check(list(sources), budget)
    if problems:
        raise PilotError("the bundles do not pass report.py --check together:\n  " + "\n  ".join(problems))
    runs = [run for source in sources for run in report.load_runs(source)]
    names = Counter(run.path.name for run in runs)
    repeated = sorted(name for name, count in names.items() if count > 1)
    if repeated:
        raise PilotError(f"run directories repeat across the bundles: {', '.join(repeated)}")
    records = [record for run in runs for _, record in run.records
               if record["task"]["family"] != metrics.PLAN_SLICE]
    if not records:
        raise PilotError("the bundles hold no run records outside the plan-level slice")
    suites: dict[str, set[str]] = {}
    for record in records:
        suites.setdefault(record["suite"]["hash"], set()).add(record["experiment_id"])
    if len(suites) > 1:
        listed = "; ".join(f"{suite}: {', '.join(sorted(experiments))}" for suite, experiments in sorted(suites.items()))
        raise PilotError(f"the bundles ran {len(suites)} suites ({listed}); runs pair by (task, seed) only on one "
                         "suite, so name the other bundles with --superseded")
    metrics.check_unique(records)
    return runs, records


def _coverage(runs: Sequence[report.Run], records: Sequence[dict], experiments: Sequence[str], directory: Path,
              cells: Sequence[Cell]) -> tuple[list[Coverage], list[str], list[str]]:
    """Each planned block's runs (module docstring), the runs no block plans, and the experiments with no manifest."""
    by_run: dict[str, list[dict]] = {}
    for record in records:
        by_run.setdefault(record["run_id"], []).append(record)
    rows = {row["record_id"]: row for row in metrics.with_models(records)}
    coverage: list[Coverage] = []
    claimed: set[str] = set()
    no_manifest = []
    for experiment in experiments:
        manifest = _manifest(directory, experiment)
        if manifest is None:
            no_manifest.append(experiment)
            continue
        for block in _blocks(manifest, runs):
            item = Coverage(block)
            for run in runs:
                mine = by_run.get(run.path.name, [])
                if mine and run.path.name not in claimed and _run_key(run, mine) == _block_key(block):
                    claimed.add(run.path.name)
                    item.run_ids.append(run.path.name)
                    for record in mine:
                        item.done[(record["task"]["instance_id"], record["seed"])] = record["execution"]["status"]
                        cell = _cell_of(rows[record["record_id"]])
                        if cell not in item.cells:
                            item.cells.append(cell)
            coverage.append(item)
    unplanned = sorted(name for name in by_run if name not in claimed)
    for cell in cells:  # a cell no block planned still gets its row
        if not any(cell in item.cells for item in coverage):
            coverage.append(Coverage(None, cells=[cell], run_ids=sorted(
                {row["run_id"] for row in rows.values() if _cell_of(row) == cell})))
    return coverage, unplanned, no_manifest


def _manifest(directory: Path, experiment: str) -> campaign.Manifest | None:
    """The experiment manifest in `directory` whose id is `experiment`, or None."""
    for path in sorted(directory.glob("*.toml")):
        try:
            with path.open("rb") as handle:
                doc = tomllib.load(handle)
        except (OSError, tomllib.TOMLDecodeError):
            continue
        if doc.get("schema_version") == campaign.SCHEMA and doc.get("id") == experiment:
            try:
                return campaign.load(path)
            except campaign.CampaignError as err:
                raise PilotError(f"the manifest of {experiment} does not load: {err}") from None
    return None


def _blocks(manifest: campaign.Manifest, runs: Sequence[report.Run]) -> list[Block]:
    out = []
    for block in manifest.blocks:
        arm, harness = _arm_file(block.arm)
        instances = block.instances or _stream_instances(block.stream, runs)
        out.append(Block(experiment_id=manifest.id, id=block.id, arm=arm, harness=harness, model=block.model,
                         seeds=block.seeds, instances=tuple(instances), optional=block.optional,
                         disturbed=block.disturbance is not None))
    return out


def _arm_file(name: str) -> tuple[str, str]:
    """An arm file's arm id, and the harness marker its runs record: "" when the file is the arm's own (its name is
    the id), else its `[arm] harness` (S08 decision 3: cheap_direct_msa's runs say "mini-swe-agent")."""
    path = ARMS_DIR / f"{name}.toml"
    try:
        with path.open("rb") as handle:
            table = tomllib.load(handle)["arm"]
        arm = table["id"]
    except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as err:
        raise PilotError(f"cannot read the arm file {path}: {err}") from None
    return arm, "" if arm == name else str(table.get("harness") or "")


def _stream_instances(stream: str, runs: Sequence[report.Run]) -> list[str]:
    """A stream's instances: from a merged run's manifest that ran it, else from the stream file."""
    for run in runs:
        config = (run.manifest or {}).get("config") or {}
        if (config.get("stream") or {}).get("id") == stream and isinstance(config["stream"].get("instances"), list):
            return list(config["stream"]["instances"])
    import vb  # noqa: PLC0415 (driver/vb.py, only for a stream no merged run ran)

    try:
        return list(vb.load_stream(stream).instances)
    except Exception as err:  # noqa: BLE001 - vb.DriverError and OSError alike: the page cannot list the plan
        raise PilotError(f"cannot read stream {stream!r}: {err}") from None


def _run_key(run: report.Run, records: Sequence[dict]) -> tuple:
    manifest = run.manifest or {}
    config = manifest.get("config") or {}
    return (records[0]["experiment_id"], records[0]["arm"], metrics.harness_of(records[0]), manifest.get("model"),
            bool(config.get("disturbances")))


def _block_key(block: Block) -> tuple:
    return block.experiment_id, block.arm, block.harness, block.model, block.disturbed


def _cell_of(row: Mapping) -> Cell:
    return Cell(row["experiment_id"], row["arm"], row["model"], row["harness"])


def _kept(rows: Iterable[dict], cell: Cell) -> list[dict]:
    """The cell's runs that count: outside the plan-level slice, neither `infra_error` nor `leak_suspected`."""
    return [row for row in rows if _cell_of(row) == cell and row["task"]["family"] != metrics.PLAN_SLICE
            and row["execution"]["status"] not in metrics.EXCLUDED]


def _coverage_json(item: Coverage) -> dict:
    block = item.block
    out = {"cells": [cell.name for cell in item.cells], "experiments": sorted({c.experiment_id for c in item.cells}),
           "run_ids": item.run_ids, "done": len(item.done)}
    if block is None:
        return {"block": None, **out, "planned": None, "pending": None}
    pending = item.pending
    return {"block": block.id, "experiment_id": block.experiment_id, "arm": block.arm, "harness": block.harness,
            "model": block.model, "seeds": list(block.seeds), "optional": block.optional,
            "disturbed": block.disturbed, **out, "planned": len(block.planned), "pending": len(pending),
            "pending_seeds": sorted({seed for _, seed in pending})}


# --------------------------------------------------------------------------- units, draws and intervals


def _units(rows_by_cell: Mapping[Cell, Sequence[dict]], *, level: int | None = None, complete: bool = False,
           min_seeds: int = 0) -> list[dict]:
    """One unit per (task, seed), each holding every cell's runs there, sorted by stratum, task and seed. `complete`
    keeps the units where every cell has a run (a pair); `min_seeds` the tasks with at least that many units."""
    grid: dict[tuple, dict] = {}
    for cell, rows in rows_by_cell.items():
        for row in rows:
            task = row["task"]
            if level is not None and task["ladder"] != level:
                continue
            key = (task["family"], task["ladder"], task["instance_id"], task["spec_variant"], row["seed"])
            unit = grid.setdefault(key, {"task": {name: task[name] for name in (
                "family", "ladder", "instance_id", "spec_variant")}, "seed": row["seed"], "runs": {}})
            unit["runs"].setdefault(cell, []).append(row)
    units = [grid[key] for key in sorted(grid)]
    if complete:
        units = [unit for unit in units if all(cell in unit["runs"] for cell in rows_by_cell)]
    if min_seeds:
        counts = Counter(bootstrap.task_key(unit) for unit in units)
        units = [unit for unit in units if counts[bootstrap.task_key(unit)] >= min_seeds]
    return units


def _sizes(units: Sequence[dict]) -> dict[tuple, int]:
    """Each task's number of units: how many seed draws one draw of the task makes."""
    return dict(Counter(bootstrap.task_key(unit) for unit in units))


def _draws(units: Sequence[dict], sizes: Mapping[tuple, int]) -> list[list[dict]]:
    """A bag of units as its task draws: consecutive units of one task, `sizes[task]` at a time (module docstring).
    Raises PilotError when the bag is not whole task draws, which would make every per-task statistic wrong."""
    draws: list[list[dict]] = []
    for unit in units:
        key = bootstrap.task_key(unit)
        if draws and bootstrap.task_key(draws[-1][0]) == key and len(draws[-1]) < sizes[key]:
            draws[-1].append(unit)
            continue
        if draws and len(draws[-1]) != sizes[bootstrap.task_key(draws[-1][0])]:
            raise PilotError("a bootstrap replicate is not whole task draws; bootstrap.py's resampling changed")
        draws.append([unit])
    if draws and len(draws[-1]) != sizes[bootstrap.task_key(draws[-1][0])]:
        raise PilotError("a bootstrap replicate is not whole task draws; bootstrap.py's resampling changed")
    return draws


Statistic = Callable[[list[list[dict]]], float | None]  # task draws -> value, None when undefined


def interval(units: Sequence[dict], statistic: Statistic, *, ratio: bool = False, undefined: float | None = None,
             convention: str = "", b: int = bootstrap.DEFAULT_B, seed: int = 0, alpha: float = ALPHA) -> Interval:
    """`statistic`'s estimate on `units` and its paired bootstrap interval (module docstring): BCa when `ratio`,
    else percentile. A replicate where `statistic` is undefined takes `undefined`; with `undefined` None such a
    replicate is an error, for statistics the units always define."""
    units = list(units)
    if not units:
        return Interval(None, why="no runs")
    sizes = _sizes(units)
    estimate = statistic(_draws(units, sizes))
    if estimate is None:
        return Interval(None, why="undefined on the data")
    method = "bca" if ratio else "percentile"
    if ratio:
        tasks = sorted(sizes, key=repr)
        jackknife = [statistic(_draws([u for u in units if bootstrap.task_key(u) != task], sizes)) for task in tasks]
        if len(tasks) < 2 or any(value is None or not math.isfinite(value) for value in jackknife):
            method = "percentile"
    count = [0]

    def wrapped(bag: Sequence[dict]) -> float:
        value = statistic(_draws(bag, sizes))
        if value is not None:
            return value
        if undefined is None:
            raise PilotError("a replicate left a statistic undefined that its units always define")
        count[0] += 1
        return undefined

    try:
        result = bootstrap.paired_bootstrap(units, wrapped, b=b, seed=seed, alpha=alpha, method=method)
    except bootstrap.BootstrapError:
        if method != "bca":
            raise
        method, count[0] = "percentile", 0
        result = bootstrap.paired_bootstrap(units, wrapped, b=b, seed=seed, alpha=alpha, method=method)
    if not math.isclose(result.estimate, estimate, rel_tol=1e-12, abs_tol=1e-15):
        raise PilotError(f"the bootstrap's estimate {result.estimate} is not the statistic's {estimate}")
    low, high = (math.inf if math.isnan(bound) else bound for bound in (result.low, result.high))
    return Interval(estimate, low, high, method=method, fallback=ratio and method != "bca", undefined=count[0],
                    convention=convention if count[0] else "", b=b)


def _rows(draw: Sequence[dict], cell: Cell, *, resolve: bool = False) -> list[dict]:
    """The cell's runs in one task draw; `resolve` leaves F8 honeypots out, as `metrics`' resolve rates do."""
    return [row for unit in draw for row in unit["runs"].get(cell, ()) if not (resolve and row["task"]["is_honeypot"])]


def vs_rate_of(cell: Cell, label: Callable[[dict], int] = metrics.vs_minus) -> Statistic:
    """`metrics.vs_rate`: the mean over task draws of each draw's share of verified successes."""

    def statistic(draws: list[list[dict]]) -> float | None:
        shares = [sum(map(label, rows)) / len(rows) for rows in (_rows(d, cell, resolve=True) for d in draws) if rows]
        return math.fsum(shares) / len(shares) if shares else None

    return statistic


def pass_k_of(cell: Cell, k: int, label: Callable[[dict], int] = metrics.vs_minus) -> Statistic:
    """`passk.pass_k`: the mean over task draws with at least k runs of C(c, k) / C(n, k)."""

    def statistic(draws: list[list[dict]]) -> float | None:
        terms = []
        for draw in draws:
            rows = _rows(draw, cell, resolve=True)
            if len(rows) >= k:
                terms.append(math.comb(sum(map(label, rows)), k) / math.comb(len(rows), k))
        return math.fsum(terms) / len(terms) if terms else None

    return statistic


def usd_per_vs_of(cell: Cell, field_name: str = "api_equiv_usd") -> Statistic:
    """`metrics`' $/VS: the cell's summed `costs.<field_name>` over its summed VS- (None with no VS)."""

    def statistic(draws: list[list[dict]]) -> float | None:
        rows = [row for draw in draws for row in _rows(draw, cell)]
        vs = sum(map(metrics.vs_minus, rows))
        return math.fsum(row["costs"][field_name] for row in rows) / vs if vs else None

    return statistic


def cost_gap_of(cell: Cell) -> Statistic:
    """`metrics`' cost_gap_ur: |sum U' - sum R| / sum R (None when sum R is 0)."""

    def statistic(draws: list[list[dict]]) -> float | None:
        rows = [row for draw in draws for row in _rows(draw, cell)]
        r = math.fsum(row["costs"]["vendor_usd"] for row in rows)
        return abs(math.fsum(row["costs"]["api_equiv_usd"] for row in rows) - r) / r if r else None

    return statistic


def false_green_rate_of(cell: Cell, label: Callable[[dict], int] = metrics.vs_minus) -> Statistic:
    """`metrics`' false_green_rate: false greens over reported passes (None with no reported pass)."""

    def statistic(draws: list[list[dict]]) -> float | None:
        passes = [row for draw in draws for row in _rows(draw, cell) if metrics.reported_pass(row)]
        return sum(1 - label(row) for row in passes) / len(passes) if passes else None

    return statistic


def difference(first: Statistic, second: Statistic) -> Statistic:
    def statistic(draws: list[list[dict]]) -> float | None:
        x, y = first(draws), second(draws)
        return None if x is None or y is None else x - y

    return statistic


def quotient(first: Statistic, second: Statistic) -> Statistic:
    def statistic(draws: list[list[dict]]) -> float | None:
        x, y = first(draws), second(draws)
        return None if x is None or y is None or not y else x / y

    return statistic


# --------------------------------------------------------------------------- the numbers


def _cell_numbers(cell: Cell, kept: Sequence[dict], by_metric: Mapping[tuple, metrics.Metric], level: int | None,
                  k: int, b: int, seed: int) -> dict:
    """One arm cell's numbers overall (`level` None) or at one level: each metric of `metrics.arm_metrics` the page
    shows, with its interval where it is an estimate. An estimate that differs from the report's is an error."""
    name = "all" if level is None else f"l{level}"
    rows = [row for row in kept if level is None or row["task"]["ladder"] == level]
    units, pass_units = _units({cell: rows}), _units({cell: rows}, min_seeds=k)
    unknown = any(row["vs"]["unknown"] for row in rows)
    out: dict = {"metrics": [], "intervals": {}, "tallies": {}, "run_ids": sorted({row["run_id"] for row in rows}),
                 "unknown": unknown, "kept": len(rows), "passes": sum(1 for row in rows if metrics.reported_pass(row)),
                 "censored_runs": [row for row in rows if row["execution"]["status"] in metrics.CENSORED],
                 "excluded_runs": [row for row in _all_rows(by_metric, name)
                                   if row["execution"]["status"] in metrics.EXCLUDED]}

    def take(metric: str, cost_basis: str | None = None) -> metrics.Metric | None:
        return by_metric.get((metric, name, cost_basis))

    def add(metric: str, statistic: Statistic, chosen: Sequence[dict], cost_basis: str | None = None, **kw) -> None:
        found = take(metric, cost_basis)
        if found is None:
            return
        if found.value is None:
            got = Interval(None, why=found.estimator.rsplit("null: ", 1)[-1] if "null: " in found.estimator else "")
        else:
            got = interval(chosen, statistic, b=b, seed=seed, **kw)
            if got.estimate is None or not math.isclose(got.estimate, found.value, rel_tol=1e-9, abs_tol=1e-12):
                raise PilotError(f"{cell.name} {metric} ({name}): the page's estimate {got.estimate} is not the "
                                 f"report's {found.value}")
        out["intervals"][metric] = got
        out["metrics"].append((_with_interval(found, got), metric))

    def bound(metric: str, statistic: Statistic, chosen: Sequence[dict], headline: str, **kw) -> None:
        """The unknown = 1 bound: its own interval when the cell has an unknown label, else the headline's."""
        if unknown:
            add(metric, statistic, chosen, **kw)
        elif take(metric) is not None and headline in out["intervals"]:
            out["intervals"][metric] = out["intervals"][headline]
            out["metrics"].append((_with_interval(take(metric), out["intervals"][headline]), metric))

    add("vs_rate", vs_rate_of(cell), units)
    bound("vs_rate_unknown_as_1", vs_rate_of(cell, metrics.vs_plus), units, "vs_rate")
    add(f"pass_hat_{k}", pass_k_of(cell, k), pass_units)
    bound(f"pass_hat_{k}_unknown_as_1", pass_k_of(cell, k, metrics.vs_plus), pass_units, f"pass_hat_{k}")
    for metric, field_name in (("usd_per_vs", "api_equiv_usd"), ("usd_per_vs_vendor", "vendor_usd")):
        amounts = [row["costs"][field_name] for row in rows]
        if metric == "usd_per_vs_vendor" and all(amount is None for amount in amounts):
            continue  # the arm reports no vendor figure (R): nothing to show beside U'
        if all(amount is not None for amount in amounts):
            add(metric, usd_per_vs_of(cell, field_name), units, "api_equiv_usd", ratio=True, undefined=UNBOUNDED,
                convention="no VS, counted as unbounded")
        else:
            _tally(out, take(metric, "api_equiv_usd"), metric)  # unmeasurable: a run's figure is unknown
    if "usd_per_vs_vendor" in out["intervals"]:
        add("cost_gap_ur", cost_gap_of(cell), units, "api_equiv_usd", ratio=True, undefined=UNBOUNDED,
            convention="R of $0, counted as unbounded")
    add("false_green_rate", false_green_rate_of(cell), units, undefined=NO_PASS,
        convention="no reported pass, counted as 1")
    bound("false_green_rate_unknown_as_1", false_green_rate_of(cell, metrics.vs_plus), units, "false_green_rate",
          undefined=NO_PASS, convention="no reported pass, counted as 1")
    for metric, basis in (("false_greens", None), ("cap_censored_runs", None), ("infra_error_runs", None),
                          ("leak_suspected_runs", None), ("unverified_runs", None), ("already_satisfied_runs", None),
                          ("spend_usd", "api_equiv_usd"), ("spend_usd", "billed_usd")):
        _tally(out, take(metric, basis), metric if basis is None else f"{metric}:{basis}")
    return out


def _all_rows(by_metric: Mapping[tuple, metrics.Metric], name: str) -> tuple[dict, ...]:
    found = by_metric.get(("infra_error_runs", name, None))
    return found.cut.rows if found is not None else ()


def _tally(out: dict, found: metrics.Metric | None, key: str) -> None:
    if found is not None:
        out["tallies"][key] = found
        out["metrics"].append((found, key))


def _with_interval(metric: metrics.Metric, got: Interval) -> metrics.Metric:
    if got.estimate is None or got.low is None:
        return metric
    if not math.isfinite(got.high):
        return replace(metric, ci=None, ci_method=f"paired_bootstrap_{got.method}")
    return replace(metric, ci=(got.low, got.high), ci_method=f"paired_bootstrap_{got.method}")


def _contrast(a: str, b_arm: str, cells: Sequence[Cell], kept: Mapping[Cell, list[dict]],
              coverage: Sequence[Coverage], k: int, b: int, seed: int) -> dict | None:
    """The pair's contrasts overall and at each level both arms ran, or its pending/unavailable status; None when
    neither arm is planned or run."""
    mine = {arm: [cell for cell in cells if cell.arm == arm and not cell.harness] for arm in (a, b_arm)}
    planned = {arm: [item for item in coverage if item.block is not None and item.block.arm == arm
                     and not item.block.harness and not item.block.optional] for arm in (a, b_arm)}
    status = {}
    for arm in (a, b_arm):
        if len(mine[arm]) > 1:
            status[arm] = f"{arm} has {len(mine[arm])} cells ({', '.join(c.name for c in mine[arm])})"
        elif not mine[arm] or not kept[mine[arm][0]]:
            status[arm] = (f"{arm} pending" if planned[arm] else f"{arm} not in the bundles")
    if not mine[a] and not mine[b_arm] and not planned[a] and not planned[b_arm]:
        return None
    out = {"a": a, "b": b_arm, "status": "; ".join(status.values()), "levels": {}, "metrics": [], "run_ids": []}
    if status:
        return out
    first, second = mine[a][0], mine[b_arm][0]
    rows = {first: kept[first], second: kept[second]}
    both = _units(rows, complete=True)
    out["run_ids"] = sorted({row["run_id"] for unit in both for cell_rows in unit["runs"].values()
                             for row in cell_rows})
    levels = sorted({unit["task"]["ladder"] for unit in both})
    costs_known = all(row["costs"]["api_equiv_usd"] is not None for unit in both
                      for cell_rows in unit["runs"].values() for row in cell_rows)
    for level in [None, *levels]:
        units = [unit for unit in both if level is None or unit["task"]["ladder"] == level]
        pass_units = _units(rows, level=level, complete=True, min_seeds=k)
        got = {
            "usd_per_vs_ratio": interval(units, quotient(usd_per_vs_of(first), usd_per_vs_of(second)), ratio=True,
                                         undefined=UNBOUNDED, convention="an arm with no VS: unbounded", b=b,
                                         seed=seed) if costs_known else Interval(None, why="a cost is unknown"),
            "vs_rate_diff": interval(units, difference(vs_rate_of(first), vs_rate_of(second)), b=b, seed=seed),
            f"pass_hat_{k}_diff": interval(pass_units, difference(pass_k_of(first, k), pass_k_of(second, k)), b=b,
                                           seed=seed) if pass_units else Interval(None, why=f"no task with {k} pairs"),
        }
        if got["usd_per_vs_ratio"].estimate is None and costs_known:
            got["usd_per_vs_ratio"] = replace(got["usd_per_vs_ratio"], why=_no_vs(units, first, second))
        out["levels"][level] = got
        clauses = _pair_clauses(first, second, level)
        # metric -> (what it is, its n, the units behind it, its cost basis)
        shapes = {
            "usd_per_vs_ratio": (f"$/VS of {first.name} / $/VS of {second.name}, each the summed costs.api_equiv_usd "
                                 "over the summed VS (App. D.2)", len(units), units, "api_equiv_usd"),
            "vs_rate_diff": (f"VS rate of {first.name} - VS rate of {second.name} (App. D.1)", len(units), units,
                             None),
            f"pass_hat_{k}_diff": (f"pass^{k} of {first.name} - pass^{k} of {second.name} (App. D.3), on the tasks "
                                   f"with {k} or more pairs", len(_sizes(pass_units)), pass_units, None),
        }
        for metric, value in got.items():
            if value.estimate is None:
                continue
            what, n, behind, basis = shapes[metric]
            rows_behind = tuple(row for unit in behind for cell_rows in unit["runs"].values() for row in cell_rows)
            found = metrics.Metric(metric, value.estimate, n, f"{what}; on the (task, seed) pairs both arms kept; "
                                   "unknown labels count as 0", basis, metrics.Cut(clauses, rows_behind),
                                   "all" if level is None else f"l{level}", level)
            out["metrics"].append((_with_interval(found, value), metric))
    return out


def _no_vs(units: Sequence[dict], first: Cell, second: Cell) -> str:
    names = [cell.name for cell in (first, second)
             if not sum(metrics.vs_minus(row) for unit in units for row in unit["runs"].get(cell, ()))]
    return f"{' and '.join(names)} verified no run here" if names else "undefined on the data"


def _pair_clauses(first: Cell, second: Cell, level: int | None) -> tuple[metrics.Clause, ...]:
    clauses = [metrics.Clause("experiment_id", "in", sorted({first.experiment_id, second.experiment_id})),
               metrics.Clause("arm", "in", [first.arm, second.arm]), metrics.Clause("harness", "==", ""),
               metrics.Clause("task.family", "!=", metrics.PLAN_SLICE),
               metrics.Clause("execution.status", "not in", list(metrics.EXCLUDED))]
    if level is not None:
        clauses.append(metrics.Clause("task.ladder", "==", level))
    return tuple(clauses)


def _experiment_of(metric: metrics.Metric) -> str:
    experiments = sorted({row["experiment_id"] for row in metric.cut.rows})
    return experiments[0] if len(experiments) == 1 else PILOT_ID


def _bound_estimators(rows: list[dict], found: Sequence[metrics.Metric]) -> None:
    """A MetricRecord whose interval is open above has no `ci` (JSON has no infinity): its estimator says so."""
    for row, metric in zip(rows, found, strict=True):
        if metric.ci is None and metric.ci_method.startswith("paired_bootstrap"):
            row["estimator"] += "; 95% interval open above (it reaches replicates with no VS), so no ci is recorded"


# --------------------------------------------------------------------------- evidence beside the numbers


def _verifier_ci(sources: Sequence[Path], given: Path | None) -> tuple[Path | None, dict | None]:
    """The verifier-CI record: `--verifier-ci`, else the one the source bundles hold (refused when they differ)."""
    paths = [Path(given)] if given else [source / VERIFIER_CI for source in sources if (source / VERIFIER_CI).is_file()]
    if not paths:
        return None, None
    contents = {path.read_bytes() for path in paths}
    if len(contents) > 1:
        raise PilotError(f"the bundles hold {len(contents)} different verifier-CI records; name one with "
                         "--verifier-ci")
    try:
        doc = json.loads(paths[0].read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        raise PilotError(f"{paths[0]}: not a readable verifier-CI record ({err})") from None
    if not isinstance(doc, dict) or doc.get("schema_version") != "vb.verifier_ci/1":
        raise PilotError(f"{paths[0]}: not a vb.verifier_ci/1 record")
    return paths[0], doc


def _ci_summary(doc: dict, records: Sequence[dict]) -> dict:
    """Green cells per family, and the cells of the merged tasks (by instance id and latent)."""
    cells = [cell for cell in doc.get("cells", []) if isinstance(cell, dict)]
    families = {}
    for family in sorted({str(cell.get("family")) for cell in cells}):
        mine = [cell for cell in cells if cell.get("family") == family]
        families[family] = {"green": sum(1 for cell in mine if not cell.get("problems")), "cells": len(mine)}
    by_name = {(cell.get("cell"), cell.get("latent")): cell for cell in cells}
    tasks = sorted({(row["task"]["instance_id"], row["task"].get("latent_version")) for row in records})
    covered = [(instance, latent) for instance, latent in tasks if (instance, latent) in by_name]
    red = [instance for instance, latent in covered if by_name[(instance, latent)].get("problems")]
    return {"secret": doc.get("secret"), "families": families, "seeds": doc.get("seeds"),
            "levels": doc.get("levels"), "latents": doc.get("latents"), "seconds": doc.get("seconds"),
            "tasks": len(tasks), "tasks_with_cells": len(covered), "tasks_red": red,
            "missing": [instance for instance, latent in tasks if (instance, latent) not in by_name],
            "trimmed_from_sha256": (doc.get("trimmed") or {}).get("from_sha256")}


def _g0(sources: Sequence[Path], experiments: Sequence[str]) -> dict | None:
    """The G0 page (`gates.py`) of the sources computed over exactly the merged experiments, else over the most."""
    found = []
    for source in sources:
        path = source / G0_FILE
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if isinstance(doc, dict) and isinstance(doc.get("checks"), list):
            found.append((path, doc))
    if not found:
        return None
    path, doc = max(found, key=lambda item: (set(item[1].get("experiments") or []) == set(experiments),
                                             len(set(item[1].get("experiments") or []) & set(experiments))))
    verdicts = Counter(check.get("verdict") for check in doc["checks"])
    return {"path": _show(path), "experiments": doc.get("experiments"), "verdict": doc.get("verdict"),
            "computed_at": doc.get("computed_at"), "counts": dict(sorted(verdicts.items())),
            "failed": [{"title": check.get("title"), "value": check.get("value"), "threshold": check.get("threshold")}
                       for check in doc["checks"] if check.get("verdict") == "fail"]}


def _superseded(path: Path, merged: Sequence[dict]) -> dict:
    """A bundle that is not merged: its experiments, arms, seeds, suite and run ids, and why it is left out."""
    runs = report.load_runs(path)
    records = [record for run in runs for _, record in run.records]
    suites = sorted({record["suite"]["hash"] for record in records})
    arms: dict[str, dict] = {}
    for record in records:
        arm = arms.setdefault(record["arm"], {"runs": 0, "seeds": set(), "run_ids": set(), "experiments": set()})
        arm["runs"] += 1
        arm["seeds"].add(record["seed"])
        arm["run_ids"].add(record["run_id"])
        arm["experiments"].add(record["experiment_id"])
    merged_suite = merged[0]["suite"]["hash"]
    why = "it ran another suite" if merged_suite not in suites else "it was named with --superseded"
    return {"bundle": _show(path), "suites": suites, "generators": _generators(records), "why": why,
            "arms": {arm: {"runs": got["runs"], "seeds": sorted(got["seeds"]), "run_ids": sorted(got["run_ids"]),
                           "experiments": sorted(got["experiments"])} for arm, got in sorted(arms.items())}}


def _generators(records: Iterable[dict]) -> dict[str, list[str]]:
    found: dict[str, set[str]] = {}
    for record in records:
        for family, version in (record["suite"].get("generator_versions") or {}).items():
            found.setdefault(family, set()).add(version)
    return {family: sorted(versions) for family, versions in sorted(found.items())}


def _label(records: Sequence[dict]) -> str:
    """S09/W10's label for the page: "pilot, descriptive: <families>, <tasks> x <seeds>, not pre-registered"."""
    families = sorted({record["task"]["family"] for record in records})
    languages = {FAMILY_LANGUAGE.get(family) for family in families}
    language = f"{languages.pop()} " if len(languages) == 1 and None not in languages else ""
    count = NUMBERS.get(len(families), str(len(families)))
    tasks = len({metrics.task_key(record) for record in records})
    seeds = len({record["seed"] for record in records})
    return (f"pilot, descriptive: {count} {language}famil{'y' if len(families) == 1 else 'ies'} "
            f"({', '.join(families)}), {tasks} tasks × {seeds} seed{'s' * (seeds != 1)}, not pre-registered")


def _show(path: Path | str) -> str:
    """A path as the page prints it: relative to the repository when inside it, never with the home directory."""
    path = Path(path)
    try:
        return str(path.resolve().relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


# --------------------------------------------------------------------------- the page


def _render(doc: dict, arm_rows: Sequence[dict], contrasts: Sequence[dict], coverage: Sequence[Coverage],
            records: Sequence[dict], k: int, b: int, seed: int, argv: Sequence[str]) -> str:
    pilot = doc["pilot"]
    notes: list[str] = []
    lines = [f"# ViabilityBench pilot: {', '.join(_arm_names(arm_rows, coverage))}", "",
             f"**{doc['label']}.**", "",
             "Every estimate below has a 95% paired bootstrap interval (S09 §4.1: tasks resampled within family × "
             f"level strata, then seeds within tasks; B = {b:,}; method and limits at the end). Every row names the "
             "runs behind it; each run id links to its directory in this bundle, and `metrics.json` holds every "
             "number as a `vb.metric_record/1` with its run ids and interval. Counts and spend totals are tallies of "
             "the runs listed, not estimates, so they have no interval.", ""]
    lines += ["## Status", ""] + _status(pilot, coverage) + [""]
    lines += ["## Reading", ""] + _reading(arm_rows, contrasts, coverage, k) + [""]
    lines += ["## Arms, all levels", ""] + _arm_table(arm_rows, coverage, k, notes) + _notes(notes) + [""]
    lines += ["## By level ℓ", ""] + _level_table(arm_rows, k, notes) + _notes(notes) + [""]
    lines += ["## Paired contrasts", ""] + _contrast_table(contrasts, k, notes) + _notes(notes)
    lines += ["", f"D3's bar is shown for context only; the pilot does not test H1. H1 (S09 §4.2, §4.4) claims a level "
              f"for `roko_full` against `fd_claude` when the 95% lower bound of the VS-rate ratio is at least "
              f"X = {envelope.X} and the upper bound of the $/VS ratio is at most Y = {envelope.Y}, on 120 tasks × 3 "
              "seeds at G5, pre-registered and Holm-adjusted. No arm here is `roko_full`, the page is not "
              "pre-registered, and nothing on it is tested against the bar.", ""]
    lines += ["## False greens: visible pass, VS = 0", ""] + _false_green_section(doc["false_greens"], records) + [""]
    lines += ["## Verifier CI", ""] + _ci_section(pilot.get("verifier_ci")) + [""]
    lines += ["## Method and limits", ""] + _method(pilot, records, k, b, seed) + [""]
    lines += ["## Provenance", ""] + _provenance(doc, argv) + [""]
    return "\n".join(lines)


def _arm_names(arm_rows: Sequence[dict], coverage: Sequence[Coverage]) -> list[str]:
    names = [row["cell"].name for row in arm_rows]
    pending = [f"{item.block.arm} (pending)" for item in coverage if item.block is not None
               and not item.block.optional and not item.done and item.block.arm not in names]
    return names + list(dict.fromkeys(pending))


def _status(pilot: dict, coverage: Sequence[Coverage]) -> list[str]:
    generators = "; ".join(f"{family} {', '.join(versions)}" for family, versions in pilot["generators"].items())
    lines = [f"- **Merged:** {', '.join(pilot['experiments'])} from {', '.join(f'`{s}`' for s in pilot['sources'])}, "
             f"on one suite, `{pilot['suite']}` ({generators}). Runs are paired by (task, seed)."]
    for item in coverage:
        block = item.block
        if block is None:
            lines.append(f"- **{', '.join(c.name for c in item.cells)}:** runs no manifest block plans: "
                         f"{_run_links(item.run_ids)}.")
            continue
        what = (f"`{block.experiment_id}` block `{block.id}` ({block.arm}{f' [{block.harness}]' if block.harness else ''}"
                f", {block.model}{', under a disturbance' if block.disturbed else ''}"
                f"{', optional' if block.optional else ''})")
        done, planned = len(item.done), len(block.planned)
        pending = item.pending
        if not pending:
            lines.append(f"- {what}: all {planned} planned runs recorded ({_run_links(item.run_ids)}).")
        elif not done:
            seeds = ", ".join(map(str, sorted({seed for _, seed in pending})))
            state = "not run" if block.optional else "**pending**"
            lines.append(f"- {what}: {state}, 0 of {planned} planned runs; seeds {seeds}.")
        else:
            by_seed = Counter(seed for _, seed in pending)
            seeds = ", ".join(f"seed {seed} ({count} runs)" for seed, count in sorted(by_seed.items()))
            lines.append(f"- {what}: **partly pending**, {done} of {planned} planned runs ({_run_links(item.run_ids)}); "
                         f"missing: {seeds}.")
    if pilot["no_manifest"]:
        lines.append(f"- No experiment manifest for {', '.join(pilot['no_manifest'])}, so their pending runs are "
                     "unknown.")
    listed = {run_id for item in coverage if item.block is None for run_id in item.run_ids}
    unplanned = [run_id for run_id in pilot["unplanned_runs"] if run_id not in listed]
    if unplanned:
        lines.append(f"- Runs no manifest block plans (shown, but outside the design): {_run_links(unplanned)}.")
    for gone in pilot["superseded"]:
        arms = "; ".join(f"{arm} × seeds {', '.join(map(str, got['seeds']))} ({got['runs']} runs: "
                         f"{', '.join(got['run_ids'])})" for arm, got in gone["arms"].items())
        generators = "; ".join(f"{family} {', '.join(v)}" for family, v in gone["generators"].items())
        lines.append(f"- **Not merged:** `{gone['bundle']}` ({arms}), suite {', '.join(f'`{s}`' for s in gone['suites'])}"
                     f" ({generators}): {gone['why']}. The page pairs runs by (task, seed) on one suite only, so none "
                     "of these runs is merged and no number here uses them.")
    lines += [f"- {note}" for note in pilot["notes"]]
    g0 = pilot.get("g0")
    if g0:
        order = {"pass": 0, "fail": 1, "not evaluated": 2}
        counts = ", ".join(f"{count} {verdict}" for verdict, count in sorted(
            g0["counts"].items(), key=lambda item: (order.get(item[0], 3), str(item[0]))))
        failed = "; ".join(f"{check['title']}: {_value(check['value'])} against {check['threshold']}"
                           for check in g0["failed"])
        lines.append(f"- **G0** (`{g0['path']}`, over {', '.join(g0['experiments'] or [])}): **{g0['verdict']}**, "
                     f"{counts}{f'. Failed: {failed}' if failed else ''}.")
    return lines


def _reading(arm_rows: Sequence[dict], contrasts: Sequence[dict], coverage: Sequence[Coverage], k: int) -> list[str]:
    """What the numbers say, in sentences computed from them."""
    lines = []
    for contrast in contrasts:
        a, b = contrast["a"], contrast["b"]
        if contrast["status"]:
            lines.append(f"- **{a} against {b}:** not measured yet ({contrast['status']})."
                         + (" This is the harness effect: a null effect is possible at pilot scale (W10), and will be "
                            "reported as one." if {a, b} == {"roko_fixed", "cheap_direct"} else ""))
            continue
        got = contrast["levels"][None]
        ratio, diff, passes = got["usd_per_vs_ratio"], got["vs_rate_diff"], got[f"pass_hat_{k}_diff"]
        parts = []
        if ratio.estimate is not None:
            side = ("lies below 1: cheaper per verified success" if ratio.high < 1 else
                    "lies above 1: it costs more per verified success" if ratio.low > 1 else
                    "includes 1: no resolved difference in cost per verified success")
            parts.append(f"$/VS ratio {_ratio(ratio)}; the interval {side}")
        else:
            parts.append(f"$/VS ratio undefined ({ratio.why})")
        if diff.estimate is not None:
            side = ("excludes 0: the VS rates differ" if diff.high < 0 or diff.low > 0 else
                    "includes 0: no resolved difference in VS rate at this n")
            parts.append(f"VS-rate difference {_diff(diff)}; the interval {side}")
        if passes.estimate is not None:
            parts.append(f"pass^{k} difference {_diff(passes)}")
        verdict = ""
        if ratio.estimate is not None and diff.estimate is not None and ratio.high < 1 and diff.high < 0:
            verdict = (f" So {a} is cheaper per verified success but verifies less: this is not \"cheaper at equal "
                       "quality\".")
        elif ratio.estimate is not None and diff.estimate is not None and ratio.high < 1 and diff.low <= 0 <= diff.high:
            verdict = f" So {a} is cheaper per verified success, with no resolved difference in VS rate at this n."
        lines.append(f"- **{a} against {b}:** {'; '.join(parts)}.{verdict}")
    for row in arm_rows:
        cell, levels = row["cell"], row["levels"]
        none = [level for level in levels if level is not None and levels[level]["intervals"].get("vs_rate")
                and levels[level]["intervals"]["vs_rate"].estimate == 0]
        if none:
            lines.append(f"- {cell.name} verified no run at {', '.join(f'ℓ{level}' for level in none)}, so its $/VS "
                         "is undefined there.")
    lines.append("- At this n, W10 expects only the cost ratio to resolve (token prices differ 10–25×) and VS "
                 "differences under about 0.3 not to.")
    return lines


def _arm_table(arm_rows: Sequence[dict], coverage: Sequence[Coverage], k: int, notes: list[str]) -> list[str]:
    head = (f"| arm | experiment | runs kept / planned | VS rate | VS, unknown = 1 | pass^{k} | $/VS, U′ (BCa) | "
            "$/VS, R (BCa) | \\|U′ − R\\| / R (BCa) | spend, U′ / billed | false greens / reported passes | "
            "false-green rate | cap-censored · infra_error · leak_suspected | runs |")
    lines = [head, "|---" * 14 + "|"]
    for row in arm_rows:
        cell, got = row["cell"], row["levels"][None]
        planned = sum(len(item.block.planned) for item in coverage if item.block is not None and cell in item.cells)
        lines.append("| " + " | ".join([
            f"**{cell.name}**", cell.experiment_id,
            f"{got['kept']} / {planned if planned else '?'}",
            _show_interval(got, "vs_rate", cell, "all", notes),
            _show_interval(got, "vs_rate_unknown_as_1", cell, "all", notes, same=not got["unknown"]),
            _show_interval(got, f"pass_hat_{k}", cell, "all", notes),
            _show_interval(got, "usd_per_vs", cell, "all", notes, money=True),
            _show_interval(got, "usd_per_vs_vendor", cell, "all", notes, money=True),
            _show_interval(got, "cost_gap_ur", cell, "all", notes, digits=3),
            _spend(got), _passes(got), _show_interval(got, "false_green_rate", cell, "all", notes),
            _counts(got), _run_links(got["run_ids"])]) + " |")
    for item in coverage:
        block = item.block
        if block is None or block.optional or item.done:
            continue
        seeds = ", ".join(map(str, sorted({seed for _, seed in item.pending})))
        lines.append("| " + " | ".join([f"**{block.arm}**", block.experiment_id, f"0 / {len(block.planned)}"]
                                       + [f"pending (seeds {seeds})"] + ["–"] * 9 + ["–"]) + " |")
    return lines


def _level_table(arm_rows: Sequence[dict], k: int, notes: list[str]) -> list[str]:
    lines = [f"| ℓ | arm | runs | VS rate | pass^{k} | $/VS, U′ (BCa) | $/VS, R (BCa) | spend, U′ / billed | "
             "false greens / reported passes | false-green rate | cap-censored · infra_error · leak_suspected | "
             "runs |", "|---" * 12 + "|"]
    levels = sorted({level for row in arm_rows for level in row["levels"] if level is not None})
    for level in levels:
        for row in arm_rows:
            got = row["levels"].get(level)
            if got is None:
                continue
            cell, where = row["cell"], f"l{level}"
            lines.append("| " + " | ".join([
                f"ℓ{level}", cell.name, str(got["kept"]), _show_interval(got, "vs_rate", cell, where, notes),
                _show_interval(got, f"pass_hat_{k}", cell, where, notes),
                _show_interval(got, "usd_per_vs", cell, where, notes, money=True),
                _show_interval(got, "usd_per_vs_vendor", cell, where, notes, money=True),
                _spend(got), _passes(got), _show_interval(got, "false_green_rate", cell, where, notes),
                _counts(got), _run_links(got["run_ids"])]) + " |")
    return lines


def _contrast_table(contrasts: Sequence[dict], k: int, notes: list[str]) -> list[str]:
    levels = sorted({level for contrast in contrasts for level in contrast["levels"] if level is not None})
    lines = ["| contrast | all levels | " + " | ".join(f"ℓ{level}" for level in levels) + " | runs |",
             "|---" * (3 + len(levels)) + "|"]
    for contrast in contrasts:
        a, b = contrast["a"], contrast["b"]
        names = (("usd_per_vs_ratio", f"$/VS ratio, {a} ÷ {b} (BCa)"), ("vs_rate_diff", f"VS rate, {a} − {b}"),
                 (f"pass_hat_{k}_diff", f"pass^{k}, {a} − {b}"))
        for metric, title in names:
            if contrast["status"]:
                lines.append(f"| {title} | pending: {contrast['status']} | " + " | ".join("–" for _ in levels)
                             + " | – |")
                continue
            cells = []
            for level in [None, *levels]:
                got = contrast["levels"].get(level, {}).get(metric)
                where = "all" if level is None else f"l{level}"
                cells.append("–" if got is None else _format(got, f"{title}, {_where(where)}", notes,
                                                             money=False, digits=3 if "ratio" in metric else 2,
                                                             signed="ratio" not in metric))
            lines.append(f"| {title} | " + " | ".join(cells) + f" | {_run_links(contrast['run_ids'])} |")
    return lines


def _false_green_section(false_greens: Sequence[dict], records: Sequence[dict]) -> list[str]:
    """Every false green, F4 first (the family built to produce them, S08 §4.3), each with its run and record ids;
    and per arm and family how many visible passes were VS = 0."""
    lines = ["A false green is a reported pass whose VS-census label is 0 (App. D.1): for direct and CLI arms the "
             "visible checks passed on the final tree; for Roko arms the final gate verdict was `passed`."]
    tally: dict[tuple[str, str], list[int]] = {}
    for row in metrics.with_models(records):
        if row["execution"]["status"] in metrics.EXCLUDED:
            continue
        key = (metrics.cell_name(row["arm"], row["model"], row["harness"]), row["task"]["family"])
        counts = tally.setdefault(key, [0, 0, 0])  # reported passes, false greens, runs
        counts[2] += 1
        if metrics.reported_pass(row):
            counts[0] += 1
            counts[1] += 1 - metrics.vs_minus(row)
    lines += ["", "| arm | family | runs | reported passes | false greens |", "|---|---|---|---|---|"]
    for (cell, family), (passes, greens, runs) in sorted(tally.items(), key=lambda item: (item[0][1] != "F4",
                                                                                          item[0])):
        lines.append(f"| {cell} | {family} | {runs} | {passes} | {greens} |")
    families = sorted({item["family"] for item in false_greens}, key=lambda family: (family != "F4", family))
    for family in families:
        mine = [item for item in false_greens if item["family"] == family]
        lines += ["", f"**{family}: {len(mine)} false green{'s' * (len(mine) != 1)}**", "",
                  "| arm | instance | ℓ | seed | status | run | record | failed checks |", "|---|---|---|---|---|---|---|---|"]
        for item in sorted(mine, key=lambda item: (item["arm"], item["ladder"], item["instance_id"], item["seed"])):
            lines.append(f"| {metrics.cell_name(item['arm'], item.get('model'))} | {item['instance_id']} | "
                         f"{item['ladder']} | {item['seed']} | {item['status']} | {_run_links([item['run_id']])} | "
                         f"`{_short(item['record_id'])}` | {', '.join(item['failed']) or '–'} |")
    if not false_greens:
        lines += ["", "No false greens in the merged runs."]
    return lines


def _ci_section(summary: dict | None) -> list[str]:
    if not summary:
        return ["No verifier-CI record in the bundles, so this page cannot show that the verifiers were green "
                "(G0's first check)."]
    families = ", ".join(f"{family.upper()} {got['green']}/{got['cells']}" for family, got in
                         summary["families"].items())
    lines = [f"`{VERIFIER_CI}` (`vb.verifier_ci/1`, `ci/verify_verifiers.py --json`), copied here from the source "
             f"bundle: green cells {families}; levels {_span(summary['levels'])}, seeds {_span(summary['seeds'])}, "
             f"latents {_span(summary['latents'])}; secret `{summary['secret']}`; {summary['seconds']} s; trimmed "
             "from sha256 "
             f"`{summary['trimmed_from_sha256']}`. A cell is green when the reference passes the visible checks and "
             "the truth suite, the stub fails both, and planted gaming passes the visible checks but not the truth "
             "suite.",
             f"Of the {summary['tasks']} merged tasks, {summary['tasks_with_cells']} have their own CI cell (same "
             f"instance id and latent), and {len(summary['tasks_red'])} of those are red"
             + (f": {', '.join(summary['tasks_red'])}" if summary["tasks_red"] else "") + "."]
    if summary["missing"]:
        lines.append(f"Tasks with no CI cell: {', '.join(summary['missing'])}.")
    return lines


def _method(pilot: dict, records: Sequence[dict], k: int, b: int, seed: int) -> list[str]:
    seeds = Counter((row["arm"], metrics.task_key(row)) for row in records
                    if row["execution"]["status"] not in metrics.EXCLUDED)
    lean = (f" With {k} seeds a task, as here, a replicate never scores a task below its estimate, so pass^{k}'s "
            "replicates lean above it, and its interval can start at the estimate." if set(seeds.values()) == {k}
            else "")
    return [
        f"- **Bootstrap** (`analysis/bootstrap.py`, S09 §4.1 as written): paired, stratified by family × level; "
        f"tasks are resampled within their stratum, then seeds within each drawn task; B = {b:,}, seed {seed}. "
        "Percentile intervals for rates and differences; BCa for $/VS, the cost ratios and |U′ − R| / R, with the "
        "percentile interval where BCa is undefined (marked †). Arm rows resample the arm's own kept runs; contrasts "
        "resample the (task, seed) pairs both arms kept, so both arms see the same draws.",
        f"- **Open decision {DECISION}:** {_coverage_note()} The decision is whether to resample tasks only (n_h − 1 "
        "draws a stratum) or to accept the conservative intervals; until the author decides, the method stays as "
        "written, so these intervals are likely conservative (too wide).",
        f"- **pass^{k} in a replicate** scores the seeds it drew as runs, so a task that passed 2 of 3 seeds scores 1 "
        f"in a replicate that draws its passing seeds three times, while its estimate counts 0.{lean}",
        "- **Undefined replicates** take the value least favourable to the arm (envelope.py's convention): $/VS or "
        "a cost ratio with no VS counts as unbounded, a false-green rate with no reported pass as 1. Such numbers "
        "are marked ‡ with the count.",
        "- **Labels and costs:** VS is S05's VS-census label; an unknown label counts as 0, with the unknown = 1 "
        "bound beside it. $/VS is the API-equivalent spend over ΣVS, priced from one snapshot; R is the vendor's "
        "own figure (Claude Code's `total_cost_usd`). Subscription runs bill $0; billed dollars are shown apart.",
        "- **Exclusions:** `infra_error` and `leak_suspected` runs are left out and counted; `aborted_cap` and "
        "`timeout` runs stay in with VS = 0 and are counted as cap censoring (App. D.1).",
        "- **Limits:** descriptive and exploratory: the families, tasks and seeds in the label above, one suite of "
        "generated tasks, and dated model snapshots. Not pre-registered, so no p-value or Holm decision applies, "
        "and no claim about H1–H7 follows from this page.",
    ]


def _coverage_note() -> str:
    """What S09's synthetic simulation (task 3339) measured of this bootstrap's coverage, from its report."""
    try:
        doc = json.loads(SIMULATION.read_text(encoding="utf-8"))
        got = {name: doc["criteria"][f"bootstrap_{name}_coverage"] for name in ("rate", "diff", "ratio")}
        sims, b = int(doc["sims"]), int(doc["resamples"]["b_per_interval"])
        low, high = got["rate"]["band"]
        text = (f"In S09's synthetic simulation (`{_show(SIMULATION)}`, {sims:,} campaigns, B = {b:,}) this two-stage "
                f"bootstrap over-covers: the VS-rate interval covered {got['rate']['value']:.3f} (Monte Carlo SE "
                f"{got['rate']['mc_se']:.3f}), above the {low:.2f}–{high:.2f} band S09 sets for the toolkit (§6 E3), "
                f"and the difference and ratio intervals covered {got['diff']['value']:.3f} and "
                f"{got['ratio']['value']:.3f}, at the band's top.")
    except (OSError, ValueError, KeyError, TypeError):
        return ("S09's synthetic simulation found this two-stage bootstrap over-covers (VS-rate coverage 0.975 "
                "against the toolkit's 0.93–0.97 band).")
    return text + " The decision records the simulation worker's explanation: the seed stage counts seed noise twice."


def _provenance(doc: dict, argv: Sequence[str]) -> list[str]:
    pilot = doc["pilot"]
    run_ids = sorted({run_id for row in doc["records"] for run_id in row["run_ids"]})
    lines = [f"- **This bundle:** {_run_links(run_ids)} (each run's summary files: manifest, records, ledger, and "
             f"the scrubbed proxy log where it has one, copied from {', '.join(f'`{s}`' for s in pilot['sources'])}); "
             f"`metrics.json` ({len(doc['records'])} MetricRecords, `experiment_id` `{doc['experiment_id']}`, merging "
             f"{', '.join(doc['experiments'])}); {f'`{VERIFIER_CI}`; ' if pilot.get('verifier_ci') else ''}this "
             "page.",
             f"- **Computed** {doc['computed_at']} at analysis commit `{doc['analysis_commit']}`; prices "
             f"`{doc['price_snapshot_id']}`; suite `{pilot['suite']}`.",
             "- **Check:** `benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py "
             f"--check {_show_out(argv)}`."]
    if argv:
        lines.append(f"- **Regenerate:** `vb report {' '.join(_shell_arg(arg) for arg in argv)}` (into an empty "
                     "directory).")
    return lines


# --------------------------------------------------------------------------- formatting


def _show_interval(got: dict, metric: str, cell: Cell, where: str, notes: list[str], *, money: bool = False,
                   digits: int = 2, same: bool = False) -> str:
    value = got["intervals"].get(metric)
    if value is None:
        tally = got["tallies"].get(metric)
        return "–" if tally is None else f"unmeasurable ({tally.estimator.rsplit('null: ', 1)[-1]})"
    text = _format(value, f"{cell.name}, {_display(metric)}, {_where(where)}", notes, money=money, digits=digits)
    return f"{text} (no unknown label)" if same else text


def _format(value: Interval, where: str, notes: list[str], *, money: bool = False, digits: int = 2,
            signed: bool = False) -> str:
    if value.estimate is None:
        return f"– ({value.why})" if value.why else "–"

    def number(x: float) -> str:
        if math.isinf(x):
            return "∞"
        if money:
            return f"${x:.4f}"
        return f"{x:+.{digits}f}" if signed else f"{x:.{digits}f}"

    marks = ""
    if value.fallback:
        marks += "†"
    if value.undefined:
        marks += "‡"
        notes.append(f"‡ {where}: {value.undefined:,} of {value.b:,} replicates undefined ({value.convention}).")
    closing = ")" if math.isinf(value.high) else "]"
    return f"{number(value.estimate)} [{number(value.low)}, {number(value.high)}{closing}{marks}"


def _ratio(value: Interval) -> str:
    return _format(value, "", [], digits=3)


def _diff(value: Interval) -> str:
    return _format(value, "", [], signed=True)


def _notes(notes: list[str]) -> list[str]:
    out = ([""] + notes) if notes else []
    notes.clear()
    return out


def _spend(got: dict) -> str:
    u, billed = got["tallies"].get("spend_usd:api_equiv_usd"), got["tallies"].get("spend_usd:billed_usd")
    show = [("unknown" if metric is None or metric.value is None else f"${metric.value:.4f}") for metric in (u, billed)]
    return f"{show[0]} / {show[1]}"


def _passes(got: dict) -> str:
    greens = got["tallies"].get("false_greens")
    return f"{'?' if greens is None else greens.value} / {got['passes']}"


def _counts(got: dict) -> str:
    def count(key: str) -> str:
        metric = got["tallies"].get(key)
        return "?" if metric is None else str(metric.value)

    text = f"{count('cap_censored_runs')} · {count('infra_error_runs')} · {count('leak_suspected_runs')}"
    listed = [*got["censored_runs"], *got["excluded_runs"]]
    if listed:
        text += ": " + "; ".join(f"{row['task']['instance_id']} seed {row['seed']} {row['execution']['status']} "
                                 f"in {_run_links([row['run_id']])}" for row in listed)
    return text


def _run_links(run_ids: Iterable[str]) -> str:
    return ", ".join(f"[{run_id}]({run_id}/)" for run_id in run_ids) or "–"


def _short(record_id: str) -> str:
    prefix, _, digest = record_id.partition(":")
    return f"{prefix}:{digest[:12]}" if digest else record_id[:20]


DISPLAY = {"vs_rate": "VS rate", "usd_per_vs": "$/VS, U′", "usd_per_vs_vendor": "$/VS, R",
           "cost_gap_ur": "|U′ − R| / R", "false_green_rate": "false-green rate"}


def _display(metric: str) -> str:
    """A metric id as the page names it, e.g. pass_hat_3_unknown_as_1 -> "pass^3 (unknown = 1)"."""
    base, bound = metric.removesuffix("_unknown_as_1"), metric.endswith("_unknown_as_1")
    name = f"pass^{base.removeprefix('pass_hat_')}" if base.startswith("pass_hat_") else DISPLAY.get(base, base)
    return f"{name} (unknown = 1)" if bound else name


def _where(cell: str) -> str:
    return "all levels" if cell == "all" else f"ℓ{cell[1:]}" if cell.startswith("l") else cell


def _span(values: object) -> str:
    """[1, 2, 3, 4, 5] as "1–5"; any other list joined with commas."""
    if not isinstance(values, list) or not values:
        return str(values)
    if all(isinstance(v, int) for v in values) and values == list(range(values[0], values[0] + len(values))):
        return f"{values[0]}–{values[-1]}" if len(values) > 1 else str(values[0])
    return ", ".join(map(str, values))


def _value(value: object) -> str:
    return json.dumps(value, ensure_ascii=False) if isinstance(value, (dict, list)) else str(value)


def _show_out(argv: Sequence[str]) -> str:
    argv = list(argv)
    if "--bundle" in argv and argv.index("--bundle") + 1 < len(argv):
        return _show(argv[argv.index("--bundle") + 1])
    return "<this directory>"


def _shell_arg(arg: str) -> str:
    return shlex.quote(_show(arg) if os.sep in arg or Path(arg).exists() else arg)
