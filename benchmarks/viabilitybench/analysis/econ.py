#!/usr/bin/env python3
"""The M3 economics report: every arm and every routing policy, priced and verified (S04 §4.9; decision 6103 (b);
task 6123).

    econ.py --records PATH [PATH ...] [--traces FILE ...] [--out econ-report.json] [--b 2000] [--seed 0]

**Inputs.** The `vb.run_record/1` rows (a `.jsonl` file, or a directory: its `*.jsonl` files and each run directory's
`records.jsonl`), every row validated against the schema, and the policy traces `roko learn self-model replay`
writes (6120's `TraceLine`: one JSON line per policy, ordering and task, with `policy`, `ordering`, `task`, `family`,
`arms`, `passed` and `cost_usd`). The records use one price snapshot (S08 §4.12).

**Rules** are `metrics.py`'s. The plan-level slice (family PL) never enters, `infra_error` and `leak_suspected` runs
are excluded and counted, and censored runs stay in with VS = 0. The label is VS-, `vs.label` with an unknown label
as 0, so no number here comes from the visible gates alone (SC5). A cell is a `metrics.cells` cell: an arm, or an arm
and a model when the arm ran more than one. Rates leave F8 honeypots out; costs keep every included run, as
`metrics.py`'s $/VS does. One run of unknown cost (`costs.api_equiv_usd` null, or `costs.source` unknown) makes a
cell's CPR null: the cell leaves CPR, the oracle and the Pareto set, and `cost_coverage.excluded_arms` names it
(S04 §4.8).

**Costs.** A run's attempt spend is `costs.api_equiv_usd`, the P1 basis. Its audit spend is `vs.cost_usd.audit` when
the record carries S05's cost split, else 0: a census label makes no audit call (`audit/labels.py`). CPR is the total
spend over the VS count with audit spend in (S04 §9.12's default), and `cpr_excl_audit_usd` stands beside it because
that default is unconfirmed. `cost_per_task_usd` is the mean spend per task run, the Pareto axis.

**Per arm** (`arms`): the VS rate, and the bound with an unknown label as 1; CPR with and without audit spend; pass^k
(`passk.pass_k`) and pass@k = mean_j [1 - C(n_j - c_j, k) / C(n_j, k)] (Chen et al., 2021) for k in {1, 3, 5},
over independent seeds only: one label per distinct seed, its lowest replicate (jiang2026beyond); the variance (the
within-task mean of p(1 - p)·n/(n - 1), the SD across seeds of the suite VS rate, ICC(1) from a one-way ANOVA with
unequal task sizes, and per task the cost's CV and p90/p50 across seeds); the envelope by family and level; regret
against the cross-seed oracle; and bootstrap intervals.

**Per policy** (`policies`, from the traces): the same rates and costs, the attempts per task run and the envelope. A
trace line joins the run records the way the replay's matrix reads them (6120's `Matrix::from_records`: per task and
arm, the labelled run of known cost with the lowest seed). A line on the plan-level slice, or one whose attempts used
an excluded run, is left out and counted; its attempts' audit spend comes from those runs, and is unknown when a run
is missing. Orderings replay one matrix, so they are not independent seeds: a policy has no pass^k or pass@k, and its
variance is the SD across orderings of the suite VS rate. A policy named `<base>@<setting>` (the π* sweep, as in
`lcb_aci@0.85`) is also a point of `sweeps.<base>`.

**Ceilings.** β is the share of tasks on which every run of every cell failed (VS-, with a Clopper-Pearson interval,
and again with an unknown label as 1). The cross-seed oracle picks, for each task and seed A, the cell that passed on
A at the lowest spend (the cheapest when none passed) and scores that cell on every other seed B. Its VS rate is the
mean over tasks and its CPR the pooled spend over the VS count. Regret is the oracle's VS rate minus a cell's or a
policy's.

**Pareto** (`pareto`): a point per arm and policy with a known spend, (cost_per_task_usd, vs_rate). A point is on the
frontier when no other point costs at most as much with at least its VS rate, one of the two strictly. Its iso-VS cost
is the cheapest frontier point with at least its VS rate.

**Intervals.** S09's family x level stratified paired bootstrap (`bootstrap.paired_bootstrap`, S09.E3), B = 2,000 by
default: tasks are resampled within strata, and seeds (a policy's orderings) within tasks. A VS rate gets a percentile
interval, each run weighted 1/n_j so that a replicate's rate is the mean over its drawn tasks; CPR, a ratio, gets BCa,
with the percentile interval standing in where BCa is undefined. A CPR replicate with no VS takes UNDEFINED_CPR, and a
bound above UNBOUNDED is written null. S04's family-cluster bootstrap is a sensitivity check only (S09 §4.4 H4: six
clusters are too few to be primary), and it is not computed here.

**Output** (`vb.econ_report/1`): sorted keys, floats rounded to 6 decimal places and one trailing newline, so the same
inputs write the same bytes. Exit status: 0 when written, 1 when the inputs cannot be reported, 2 on a file error.

API:
    SCHEMA, KS, B, UNDEFINED_CPR, UNBOUNDED; EconError
    load_records(paths) -> list[dict]; load_traces(paths) -> list[dict]
    attempt_usd(record) -> float | None; audit_usd(record) -> float | None
    pass_at_k(labels, k) -> passk.PassK
    independent_runs(rows) -> (runs, repeats_left_out)
    variance(runs) -> dict; icc1(groups) -> float | None
    beta(rows) -> dict; cross_seed_oracle(runs) -> dict; pareto(points) -> dict
    build(records, traces=(), *, b=B, seed=0, alpha=0.05) -> dict
    dumps(report) -> str; write(report, path) -> Path; render(report) -> str
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from collections import Counter
from collections.abc import Callable, Hashable, Iterable, Mapping, Sequence
from fractions import Fraction
from pathlib import Path

import bootstrap
import metrics
import passk
import report  # its import puts schema/ on sys.path
import validate  # noqa: E402  (schema/validate.py)

SCHEMA = "vb.econ_report/1"
KS = (1, 3, 5)
B = 2_000  # S04 §4.9's resamples
DIGITS = 6
UNDEFINED_CPR = 1e9  # a replicate's CPR when it holds no VS (envelope.UNDEFINED_C's convention)
UNBOUNDED = 1e6  # no real CPR reaches a million dollars per VS: a bound above it came from replicates with no VS
SWEEP = "@"
TRACE_KEYS = ("policy", "ordering", "task", "family", "arms", "passed", "cost_usd")
LABEL = ("VS- (an unknown label counts as 0), never the visible gates (SC5); CPR includes audit spend (S04 §9.12) "
         "and cpr_excl_audit_usd stands beside it")
NOT_INDEPENDENT = "orderings replay one matrix cell per task and arm, so they are not independent seeds"


class EconError(ValueError):
    """The inputs cannot be reported honestly: the message says why."""


# --- inputs -----------------------------------------------------------------------------------------------------


def load_records(paths: Iterable[Path]) -> list[dict]:
    """The run records in `paths`, in path order. EconError lists every row that is not JSON or fails the schema."""
    records, problems = [], []
    for file in _record_files(paths):
        for number, line in enumerate(file.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip():
                continue
            where = f"{file}:{number}"
            try:
                doc = json.loads(line)
            except ValueError as err:
                problems.append(f"{where}: not JSON ({err})")
                continue
            errors = validate.validate("run-record", doc)
            problems += [f"{where}: {error}" for error in errors]
            if not errors:
                records.append(doc)
    if problems:
        raise EconError("invalid run records:\n  " + "\n  ".join(problems))
    if not records:
        raise EconError("no run records")
    return records


def load_traces(paths: Iterable[Path]) -> list[dict]:
    """The policy trace lines in `paths`, in path order. EconError for a line that is not a trace line."""
    lines = []
    for path in paths:
        for number, text in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if not text.strip():
                continue
            try:
                line = json.loads(text)
            except ValueError as err:
                raise EconError(f"{path}:{number}: not JSON ({err})") from None
            missing = [key for key in TRACE_KEYS if not isinstance(line, dict) or key not in line]
            if missing:
                raise EconError(f"{path}:{number}: a trace line needs {', '.join(missing)}")
            lines.append(line)
    return lines


def _record_files(paths: Iterable[Path]) -> list[Path]:
    files = []
    for path in paths:
        if path.is_dir():
            files += sorted(path.glob("*.jsonl")) + sorted(path.glob("*/records.jsonl"))
        elif path.is_file():
            files.append(path)
        else:
            raise EconError(f"no such file or directory: {path}")
    return files


# --- costs and labels -------------------------------------------------------------------------------------------


def attempt_usd(record: Mapping) -> float | None:
    """The run's attempt spend on the P1 basis, `costs.api_equiv_usd`; None when it is unknown."""
    costs = record["costs"]
    return None if costs.get("source") == "unknown" else costs["api_equiv_usd"]


def audit_usd(record: Mapping) -> float | None:
    """The run's audit spend: `vs.cost_usd.audit` when the record carries S05's cost split (None when that is
    unknown), else 0.0, since a census label makes no audit call."""
    split = record["vs"].get("cost_usd")
    return split.get("audit") if isinstance(split, Mapping) else 0.0


def _spend(record: Mapping) -> float | None:
    attempt, audit = attempt_usd(record), audit_usd(record)
    return None if attempt is None or audit is None else attempt + audit


def _costs(attempt: Sequence[float | None], audit: Sequence[float | None], vs: int) -> dict:
    """Spend, CPR with and without audit spend, and the mean spend per task run, over one set of runs."""
    unknown = sum(amount is None for amount in attempt)
    audit_unknown = sum(amount is None for amount in audit)
    spend = None if unknown else math.fsum(attempt)
    audit_spend = None if audit_unknown else math.fsum(audit)
    total = None if spend is None or audit_spend is None else spend + audit_spend
    if unknown:
        note = f"null: {unknown} run(s) of unknown cost, so the cell is unmeasurable"
    elif not vs:
        note = "null: no VS"
    elif audit_unknown:
        note = f"cpr_usd null: {audit_unknown} run(s) of unknown audit spend"
    else:
        note = ""
    return {
        "spend_usd": spend,
        "audit_usd": audit_spend,
        "cpr_usd": total / vs if total is not None and vs else None,
        "cpr_excl_audit_usd": spend / vs if spend is not None and vs else None,
        "cost_per_task_usd": total / len(attempt) if total is not None and attempt else None,
        "unknown_cost_runs": unknown,
        "unknown_audit_runs": audit_unknown,
        "cpr_note": note,
    }


def pass_at_k(labels: Mapping[Hashable, Sequence[int]], k: int) -> passk.PassK:
    """pass@k, the mean over tasks of 1 - C(n - c, k) / C(n, k): each complement is `passk.task_pass_k` of the task's
    failures, so the sum stays exact. Tasks with fewer than k runs are skipped and listed, as in `passk.pass_k`."""
    terms, used, skipped = [], [], []
    for task in sorted(labels):
        runs = labels[task]
        if any(label not in (0, 1) for label in runs):
            raise ValueError(f"task {task!r}: labels must be 0 or 1, not {list(runs)!r}")
        if len(runs) < k:
            skipped.append(task)
            continue
        terms.append(1 - passk.task_pass_k(len(runs) - sum(runs), len(runs), k))
        used.append(task)
    value = float(sum(terms, Fraction(0)) / len(terms)) if terms else None
    return passk.PassK(k=k, value=value, tasks=tuple(used), skipped=tuple(skipped))


def independent_runs(rows: Iterable[Mapping]) -> tuple[list[Mapping], int]:
    """One run per (cell, task, seed), its lowest replicate, and how many repeats were left out: pass^k, pass@k, the
    variance and the oracle count independent seeds only."""
    first: dict[tuple, Mapping] = {}
    count = 0
    for row in rows:
        count += 1
        key = (row["arm"], row.get("model"), metrics.task_key(row), row["seed"])
        kept = first.get(key)
        if kept is None or row["replicate"] < kept["replicate"]:
            first[key] = row
    return list(first.values()), count - len(first)


def _labels(runs: Iterable[Mapping]) -> dict[tuple, list[int]]:
    """Each task's VS- labels in seed order."""
    by_task: dict[tuple, dict[int, int]] = {}
    for row in runs:
        by_task.setdefault(metrics.task_key(row), {})[row["seed"]] = metrics.vs_minus(row)
    return {task: [seeds[seed] for seed in sorted(seeds)] for task, seeds in by_task.items()}


def _passk(result: passk.PassK) -> dict:
    return {"value": result.value, "tasks": len(result.tasks), "skipped": len(result.skipped)}


# --- variance ---------------------------------------------------------------------------------------------------


def variance(runs: Sequence[Mapping]) -> dict:
    """S04 §4.9's variance row over one cell's independent runs (non-honeypot)."""
    by_task: dict[tuple, dict[int, Mapping]] = {}
    for row in runs:
        by_task.setdefault(metrics.task_key(row), {})[row["seed"]] = row
    groups = {task: [metrics.vs_minus(seeds[seed]) for seed in sorted(seeds)] for task, seeds in by_task.items()}
    within = []
    for labels in groups.values():
        if len(labels) > 1:
            p = Fraction(sum(labels), len(labels))
            within.append(p * (1 - p) * len(labels) / (len(labels) - 1))
    suite: dict[int, list[int]] = {}
    for seeds in by_task.values():
        for seed, row in seeds.items():
            suite.setdefault(seed, []).append(metrics.vs_minus(row))
    rates = [sum(labels) / len(labels) for _, labels in sorted(suite.items())]
    by_cost = {}
    for task, seeds in sorted(by_task.items(), key=lambda item: _task_name(item[0])):
        amounts = [_spend(seeds[seed]) for seed in sorted(seeds)]
        if len(amounts) < 2 or None in amounts:
            continue
        mean, median = math.fsum(amounts) / len(amounts), _quantile(amounts, 0.5)
        by_cost[_task_name(task)] = {"cv": statistics.stdev(amounts) / mean if mean > 0 else None,
                                     "p90_p50": _quantile(amounts, 0.9) / median if median > 0 else None}
    cvs = [cost["cv"] for cost in by_cost.values() if cost["cv"] is not None]
    spreads = [cost["p90_p50"] for cost in by_cost.values() if cost["p90_p50"] is not None]
    return {
        "within_task": float(sum(within, Fraction(0)) / len(within)) if within else None,
        "seed_sd": statistics.stdev(rates) if len(rates) > 1 else None,
        "icc1": icc1(list(groups.values())),
        "cost_cv_median": statistics.median(cvs) if cvs else None,
        "cost_p90_p50_median": statistics.median(spreads) if spreads else None,
        "cost_by_task": by_cost,
    }


def icc1(groups: Sequence[Sequence[int]]) -> float | None:
    """ICC(1) from a one-way random-effects ANOVA with tasks as groups, unequal sizes through
    n0 = (N - sum n_j^2 / N) / (J - 1). None with fewer than two tasks, no task run twice, or no variance at all."""
    groups = [list(group) for group in groups if group]
    tasks, runs = len(groups), sum(len(group) for group in groups)
    if tasks < 2 or runs <= tasks:
        return None
    grand = Fraction(sum(map(sum, groups)), runs)
    means = [Fraction(sum(group), len(group)) for group in groups]
    between = sum((len(group) * (mean - grand) ** 2 for group, mean in zip(groups, means)), Fraction(0))
    within = sum((sum(((y - mean) ** 2 for y in group), Fraction(0)) for group, mean in zip(groups, means)),
                 Fraction(0))
    msb, msw = between / (tasks - 1), within / (runs - tasks)
    n0 = (runs - Fraction(sum(len(group) ** 2 for group in groups), runs)) / (tasks - 1)
    denominator = msb + (n0 - 1) * msw
    return float((msb - msw) / denominator) if denominator else None


def _quantile(values: Sequence[float], q: float) -> float:
    """The q quantile (0-1), by linear interpolation between order statistics."""
    data = sorted(values)
    rank = q * (len(data) - 1)
    low, high = math.floor(rank), math.ceil(rank)
    return data[low] + (rank - low) * (data[high] - data[low])


def _task_name(task: tuple) -> str:
    instance, variant = task
    return instance if variant is None else f"{instance}/{variant}"


# --- ceilings and the frontier ----------------------------------------------------------------------------------


def beta(rows: Sequence[Mapping]) -> dict:
    """β over the included non-honeypot runs of every cell: the share of tasks whose every run failed."""
    by_task: dict[tuple, list[Mapping]] = {}
    for row in rows:
        by_task.setdefault(metrics.task_key(row), []).append(row)
    if not by_task:
        return {"tasks": 0, "all_failed": 0, "beta": None, "ci": None, "ci_method": "clopper_pearson",
                "beta_unknown_as_1": None}
    failed = sum(all(not metrics.vs_minus(row) for row in runs) for runs in by_task.values())
    bound = sum(all(not metrics.vs_plus(row) for row in runs) for runs in by_task.values())
    tasks = len(by_task)
    return {"tasks": tasks, "all_failed": failed, "beta": failed / tasks,
            "ci": list(metrics.clopper_pearson(failed, tasks)), "ci_method": "clopper_pearson",
            "beta_unknown_as_1": bound / tasks}


def cross_seed_oracle(runs: Sequence[Mapping]) -> dict:
    """Select on seed A, score on seed B, over independent runs of priced cells (module docstring)."""
    table: dict[tuple, dict[int, dict[str, tuple[int, float]]]] = {}
    for row in runs:
        cell = metrics.cell_name(row["arm"], row.get("model"))
        by_seed = table.setdefault(metrics.task_key(row), {}).setdefault(row["seed"], {})
        by_seed[cell] = (metrics.vs_minus(row), _spend(row))
    rates, labels, spent, skipped = [], [], [], 0
    for task in sorted(table, key=_task_name):
        by_seed = table[task]
        scored = []
        for seed_a in sorted(by_seed):
            cells = by_seed[seed_a]
            chosen = min(cells, key=lambda cell: (-cells[cell][0], cells[cell][1], cell))
            scored += [by_seed[seed_b][chosen] for seed_b in sorted(by_seed)
                       if seed_b != seed_a and chosen in by_seed[seed_b]]
        if not scored:
            skipped += 1
            continue
        rates.append(Fraction(sum(label for label, _ in scored), len(scored)))
        labels += [label for label, _ in scored]
        spent += [amount for _, amount in scored]
    if not rates:
        return {"tasks": 0, "skipped_tasks": skipped, "pairs": 0, "vs_rate": None, "cpr_usd": None,
                "cost_per_task_usd": None, "note": "no task has two seeds of one cell"}
    vs = sum(labels)
    return {"tasks": len(rates), "skipped_tasks": skipped, "pairs": len(labels),
            "vs_rate": float(sum(rates, Fraction(0)) / len(rates)),
            "cpr_usd": math.fsum(spent) / vs if vs else None,
            "cost_per_task_usd": math.fsum(spent) / len(spent), "note": ""}


def pareto(points: Sequence[Mapping]) -> dict:
    """The frontier over `points` ({"name", "kind", "cost_per_task_usd", "vs_rate"}): lower cost and higher VS rate
    are better. Every point comes back marked, with its iso-VS cost and what reaching it would save."""
    ordered = sorted(points, key=lambda p: (p["cost_per_task_usd"], -p["vs_rate"], p["kind"], p["name"]))

    def dominated(point: Mapping) -> bool:
        return any(other["cost_per_task_usd"] <= point["cost_per_task_usd"] and other["vs_rate"] >= point["vs_rate"]
                   and (other["cost_per_task_usd"] < point["cost_per_task_usd"] or other["vs_rate"] > point["vs_rate"])
                   for other in ordered)

    frontier = [point for point in ordered if not dominated(point)]
    marked = []
    for point in ordered:
        # The cheapest of the highest-VS points is never dominated, so some frontier point always qualifies.
        iso = min(other["cost_per_task_usd"] for other in frontier if other["vs_rate"] >= point["vs_rate"])
        marked.append({**point, "on_frontier": not dominated(point), "iso_vs_cost_usd": iso,
                       "iso_vs_saving_usd": point["cost_per_task_usd"] - iso})
    return {"points": marked, "frontier": [f"{point['kind']}:{point['name']}" for point in frontier]}


# --- intervals --------------------------------------------------------------------------------------------------


def _rate_ci(rows: Sequence[Mapping], label: Callable[[Mapping], int], *, b: int, seed: int,
             alpha: float) -> dict | None:
    """The percentile interval of the VS rate, each run weighted 1/n_j (module docstring)."""
    if not rows:
        return None
    weight = Counter(bootstrap.task_key(row) for row in rows)

    def rate(bag: Sequence[Mapping]) -> float:
        shares = [1 / weight[bootstrap.task_key(row)] for row in bag]
        return math.fsum(share * label(row) for share, row in zip(shares, bag)) / math.fsum(shares)

    result = bootstrap.paired_bootstrap(rows, rate, b=b, seed=seed, alpha=alpha)
    return {"low": result.low, "high": result.high, "method": result.method}


def _cpr_ci(rows: Sequence[Mapping], label: Callable[[Mapping], int], spend: Callable[[Mapping], float], *, b: int,
            seed: int, alpha: float) -> dict:
    """The BCa interval of the pooled spend over the VS count (percentile where BCa is undefined)."""

    def cpr(bag: Sequence[Mapping]) -> float:
        vs = sum(label(row) for row in bag)
        return math.fsum(spend(row) for row in bag) / vs if vs else UNDEFINED_CPR

    try:
        result = bootstrap.paired_bootstrap(rows, cpr, b=b, seed=seed, alpha=alpha, method="bca")
    except bootstrap.BootstrapError:
        result = bootstrap.paired_bootstrap(rows, cpr, b=b, seed=seed, alpha=alpha)
    return {"low": _bound(result.low), "high": _bound(result.high), "method": result.method}


def _bound(value: float) -> float | None:
    return None if value > UNBOUNDED else value


# --- arms and policies ------------------------------------------------------------------------------------------


def _arm(rows: list[Mapping], oracle_vs: float | None, *, b: int, seed: int, alpha: float) -> dict:
    """One cell's numbers; `rows` are its included runs, honeypots too."""
    resolve = [row for row in rows if not row["task"]["is_honeypot"]]
    runs, repeats = independent_runs(resolve)
    labels = _labels(runs)
    vs = sum(map(metrics.vs_minus, rows))
    entry = {
        "runs": len(rows),
        "tasks": len({metrics.task_key(row) for row in resolve}),
        "seeds": sorted({row["seed"] for row in rows}),
        "vs": vs,
        "vs_rate": metrics.vs_rate(resolve),
        "vs_rate_unknown_as_1": metrics.vs_rate(resolve, metrics.vs_plus),
        **_costs([attempt_usd(row) for row in rows], [audit_usd(row) for row in rows], vs),
        "pass_hat_k": {str(k): _passk(passk.pass_k(labels, k)) for k in KS},
        "pass_at_k": {str(k): _passk(pass_at_k(labels, k)) for k in KS},
        "repeats_left_out": repeats,
        "variance": variance(runs),
        "envelope": _envelope(rows, metrics.vs_minus, attempt_usd, audit_usd),
    }
    entry["regret_vs"] = _regret(oracle_vs, entry["vs_rate"])
    entry["ci"] = {"vs_rate": _rate_ci(resolve, metrics.vs_minus, b=b, seed=seed, alpha=alpha),
                   "cpr_usd": None if entry["cpr_usd"] is None else
                   _cpr_ci(rows, metrics.vs_minus, _spend, b=b, seed=seed, alpha=alpha)}
    return entry


def _policy(rows: list[Mapping], oracle_vs: float | None, *, b: int, seed: int, alpha: float) -> dict:
    """One policy's numbers; `rows` are its kept trace lines, shaped like run records for the bootstrap."""
    resolve = [row for row in rows if not row["task"]["is_honeypot"]]
    vs = sum(_passed(row) for row in rows)
    suite: dict[int, list[int]] = {}
    for row in resolve:
        suite.setdefault(row["seed"], []).append(_passed(row))
    rates = [sum(labels) / len(labels) for _, labels in sorted(suite.items())]
    entry = {
        "runs": len(rows),
        "tasks": len({bootstrap.task_key(row) for row in resolve}),
        "orderings": len({row["seed"] for row in rows}),
        "vs": vs,
        "vs_rate": metrics.vs_rate(resolve, _passed),
        **_costs([row["cost_usd"] for row in rows], [row["audit_usd"] for row in rows], vs),
        "attempts_per_task": sum(row["attempts"] for row in rows) / len(rows),
        "pass_hat_k": None,
        "pass_at_k": None,
        "pass_k_note": NOT_INDEPENDENT,
        "variance": {"ordering_sd": statistics.stdev(rates) if len(rates) > 1 else None},
        "envelope": _envelope(rows, _passed, lambda row: row["cost_usd"], lambda row: row["audit_usd"]),
    }
    entry["regret_vs"] = _regret(oracle_vs, entry["vs_rate"])
    entry["ci"] = {"vs_rate": _rate_ci(resolve, _passed, b=b, seed=seed, alpha=alpha),
                   "cpr_usd": None if entry["cpr_usd"] is None else
                   _cpr_ci(rows, _passed, lambda row: row["cost_usd"] + row["audit_usd"], b=b, seed=seed,
                           alpha=alpha)}
    return entry


def _passed(row: Mapping) -> int:
    return row["passed"]


def _regret(oracle_vs: float | None, vs_rate: float | None) -> float | None:
    return None if oracle_vs is None or vs_rate is None else oracle_vs - vs_rate


def _envelope(rows: Sequence[Mapping], label: Callable[[Mapping], int], attempt: Callable[[Mapping], float | None],
              audit: Callable[[Mapping], float | None]) -> dict:
    """The VS rate and CPR in each family x level cell, keyed "<family>-l<level>" as `metrics.py` names cells."""
    cells: dict[tuple, list[Mapping]] = {}
    for row in rows:
        cells.setdefault((row["task"]["family"], row["task"]["ladder"]), []).append(row)
    out = {}
    for family, level in sorted(cells, key=lambda cell: (cell[0], -1 if cell[1] is None else cell[1])):
        mine = cells[(family, level)]
        resolve = [row for row in mine if not row["task"]["is_honeypot"]]
        vs = sum(label(row) for row in mine)
        costs = _costs([attempt(row) for row in mine], [audit(row) for row in mine], vs)
        out[f"{family}-l{'?' if level is None else level}"] = {
            "runs": len(mine),
            "tasks": len({bootstrap.task_key(row) for row in resolve}),
            "vs_rate": metrics.vs_rate(resolve, label) if resolve else None,
            "cpr_usd": costs["cpr_usd"],
            "cpr_excl_audit_usd": costs["cpr_excl_audit_usd"],
        }
    return out


def _model(record: Mapping) -> str | None:
    """The model the run's first attempt asked for; None when it recorded none."""
    attempts = record.get("execution", {}).get("attempts") or [{}]
    return attempts[0].get("model_requested")


def arm_label(arm: str, model: str | None) -> str:
    """6120's `arm_label`: the matrix arm of `arm`'s runs of `model` when the arm ran several models,
    `<arm>[<model>]`, or `<arm>[?]` for a run that recorded no model (gap-b10978)."""
    return f"{arm}[{'?' if model is None else model}]"


def _matrix(records: Iterable[Mapping]) -> dict[tuple[str, str], Mapping]:
    """6120's `Matrix::from_records` cells: per (instance, arm, model), the labelled run of known cost with the lowest
    seed, the first one read on a tie. An arm whose runs asked for several models is one arm per model (`arm_label`,
    gap-b10978), as the replay's traces name it; an arm that ran one model keeps its name."""
    runs = [record for record in records if record["vs"].get("label") is not None and attempt_usd(record) is not None]
    models: dict[str, set[str | None]] = {}
    for record in runs:
        models.setdefault(record["arm"], set()).add(_model(record))
    cells: dict[tuple[str, str], Mapping] = {}
    for record in runs:
        arm = record["arm"]
        if len(models[arm]) > 1:
            arm = arm_label(arm, _model(record))
        key = (record["task"]["instance_id"], arm)
        kept = cells.get(key)
        if kept is None or record["seed"] < kept["seed"]:
            cells[key] = record
    return cells


def _trace_rows(traces: Sequence[Mapping], records: Sequence[Mapping]) -> tuple[dict[str, list[dict]], dict]:
    """The kept trace lines by policy, each shaped like a run record (task, seed = ordering), and the line counts."""
    seen: set[tuple] = set()
    for line in traces:
        key = (line["policy"], line["ordering"], line["task"])
        if key in seen:
            raise EconError(f"policy {key[0]} ran task {key[2]} twice in ordering {key[1]}")
        seen.add(key)
    cells = _matrix(records)
    tasks = {record["task"]["instance_id"]: record["task"] for record in records}
    counts = {"lines": len(traces), "kept": 0, "plan_slice": 0, "touched_excluded": 0, "unmatched": 0}
    by_policy: dict[str, list[dict]] = {}
    for line in traces:
        task = tasks.get(line["task"])
        if task is not None and task["family"] == metrics.PLAN_SLICE:
            counts["plan_slice"] += 1
            continue
        used = [cells.get((line["task"], arm)) for arm in line["arms"]]
        if any(cell is not None and cell["execution"]["status"] in metrics.EXCLUDED for cell in used):
            counts["touched_excluded"] += 1
            continue
        matched = task is not None and None not in used
        counts["unmatched"] += not matched
        counts["kept"] += 1
        audits = [audit_usd(cell) for cell in used] if matched else [None]
        by_policy.setdefault(line["policy"], []).append({
            "task": {"instance_id": line["task"], "spec_variant": None, "family": line["family"],
                     "ladder": task["ladder"] if task else None, "is_honeypot": bool(task and task["is_honeypot"])},
            "seed": line["ordering"],
            "passed": int(bool(line["passed"])),
            "cost_usd": line["cost_usd"],
            "audit_usd": None if None in audits else math.fsum(audits),
            "attempts": len(line["arms"]),
        })
    return by_policy, counts


def _sweeps(policies: Mapping[str, Mapping]) -> dict:
    """Each `<base>@<setting>` policy as a point of its base's sweep, in setting order."""
    sweeps: dict[str, list[dict]] = {}
    for name, entry in policies.items():
        base, sep, setting = name.partition(SWEEP)
        if not sep:
            continue
        try:
            value: float | str = float(setting)
        except ValueError:
            value = setting
        sweeps.setdefault(base, []).append({"policy": name, "setting": value, "vs_rate": entry["vs_rate"],
                                            "cost_per_task_usd": entry["cost_per_task_usd"],
                                            "cpr_usd": entry["cpr_usd"], "regret_vs": entry["regret_vs"]})
    for points in sweeps.values():
        points.sort(key=lambda point: (1, 0.0, point["setting"]) if isinstance(point["setting"], str)
                    else (0, point["setting"], ""))
    return sweeps


def _coverage(by_cell: Mapping[str, Sequence[Mapping]]) -> dict:
    runs = [row for rows in by_cell.values() for row in rows]
    known = sum(attempt_usd(row) is not None for row in runs)
    excluded = []
    for cell, rows in sorted(by_cell.items()):
        unknown = sum(attempt_usd(row) is None for row in rows)
        if unknown:
            excluded.append({"arm": cell, "runs": len(rows), "unknown_cost_runs": unknown})
    return {"runs": len(runs), "known": known, "share": known / len(runs) if runs else None,
            "audit_known": sum(audit_usd(row) is not None for row in runs),
            "label_known": sum(not row["vs"]["unknown"] for row in runs),
            "excluded_arms": excluded,
            "note": "an arm with a run of unknown cost leaves CPR, the oracle and the Pareto set (S04 §4.8)"}


# --- the report -------------------------------------------------------------------------------------------------


def build(records: Sequence[Mapping], traces: Sequence[Mapping] = (), *, b: int = B, seed: int = 0,
          alpha: float = 0.05) -> dict:
    """The economics report of `records` and the policy `traces` (module docstring)."""
    records = list(records)
    if not records:
        raise EconError("no run records")
    metrics.check_unique(records)
    snapshot = report.single_snapshot(records)
    rows = metrics.with_models(records)
    kept = [row for row in rows
            if row["task"]["family"] != metrics.PLAN_SLICE and row["execution"]["status"] not in metrics.EXCLUDED]
    by_cell: dict[str, list[Mapping]] = {}
    for row in kept:
        by_cell.setdefault(metrics.cell_name(row["arm"], row["model"]), []).append(row)
    resolve = [row for row in kept if not row["task"]["is_honeypot"]]
    priced = {cell for cell, cell_rows in by_cell.items() if all(_spend(row) is not None for row in cell_rows)}
    runs, _ = independent_runs(row for row in resolve if metrics.cell_name(row["arm"], row["model"]) in priced)
    oracle = cross_seed_oracle(runs)
    arms = {cell: _arm(cell_rows, oracle["vs_rate"], b=b, seed=seed, alpha=alpha)
            for cell, cell_rows in sorted(by_cell.items())}
    by_policy, counts = _trace_rows(list(traces), records)
    policies = {name: _policy(lines, oracle["vs_rate"], b=b, seed=seed, alpha=alpha)
                for name, lines in sorted(by_policy.items())}
    points = [{"name": name, "kind": kind, "cost_per_task_usd": entry["cost_per_task_usd"],
               "vs_rate": entry["vs_rate"]}
              for kind, group in (("arm", arms), ("policy", policies)) for name, entry in group.items()
              if entry["cost_per_task_usd"] is not None and entry["vs_rate"] is not None]
    statuses = Counter(row["execution"]["status"] for row in rows)
    return {
        "schema_version": SCHEMA,
        "price_snapshot_id": snapshot,
        "experiments": sorted({record["experiment_id"] for record in records}),
        "label": LABEL,
        "b": b,
        "seed": seed,
        "alpha": alpha,
        "ks": list(KS),
        "strata": list(bootstrap.DEFAULT_STRATA),
        "runs": {"records": len(records), "included": len(kept),
                 "plan_slice": sum(row["task"]["family"] == metrics.PLAN_SLICE for row in rows),
                 **{status: statuses[status] for status in metrics.EXCLUDED}},
        "arms": arms,
        "policies": policies,
        "sweeps": _sweeps(policies),
        "ceilings": {"beta": beta(resolve), "cross_seed_oracle": oracle},
        "pareto": pareto(points),
        "cost_coverage": _coverage(by_cell),
        "traces": counts,
    }


def _rounded(value: object) -> object:
    """`value` with every float rounded to DIGITS places (a non-finite one becomes null) and mappings as dicts."""
    if isinstance(value, float):
        if not math.isfinite(value):
            return None
        value = round(value, DIGITS)
        return 0.0 if value == 0 else value
    if isinstance(value, Mapping):
        return {str(key): _rounded(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_rounded(item) for item in value]
    return value


def dumps(doc: Mapping) -> str:
    """The report's text: sorted keys and fixed rounding, so the same inputs give the same bytes."""
    return json.dumps(_rounded(doc), sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def write(doc: Mapping, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(dumps(doc), encoding="utf-8")
    return path


def render(doc: Mapping) -> str:
    """The printed summary: one line per arm and policy, then the ceilings and the frontier."""

    def number(value: object, digits: int = 4) -> str:
        return "n/a" if value is None else f"{value:.{digits}f}"

    lines = [f"econ: {len(doc['arms'])} arm(s), {len(doc['policies'])} policy(ies), price snapshot "
             f"{doc['price_snapshot_id']}, B = {doc['b']}"]
    for kind, group in (("arm", doc["arms"]), ("policy", doc["policies"])):
        for name, entry in group.items():
            lines.append(f"  {kind} {name}: VS {number(entry['vs_rate'], 3)}, CPR {number(entry['cpr_usd'])} "
                         f"(excl. audit {number(entry['cpr_excl_audit_usd'])}), $/task "
                         f"{number(entry['cost_per_task_usd'])}" + (f"; {entry['cpr_note']}" if entry["cpr_note"]
                                                                   else ""))
    ceilings = doc["ceilings"]
    lines.append(f"  beta {number(ceilings['beta']['beta'], 3)} over {ceilings['beta']['tasks']} task(s); cross-seed "
                 f"oracle VS {number(ceilings['cross_seed_oracle']['vs_rate'], 3)}")
    lines.append(f"  frontier: {', '.join(doc['pareto']['frontier']) or 'none'}")
    excluded = doc["cost_coverage"]["excluded_arms"]
    if excluded:
        lines.append("  excluded from CPR (unknown cost): " + ", ".join(
            f"{item['arm']} ({item['unknown_cost_runs']} of {item['runs']} runs)" for item in excluded))
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        records = load_records(Path(path) for path in args.records)
        traces = load_traces(Path(path) for path in args.traces)
        doc = build(records, traces, b=args.b, seed=args.seed)
        out = write(doc, Path(args.out))
    except (EconError, metrics.MetricsError, report.ReportError, bootstrap.BootstrapError) as err:
        print(f"econ: {err}", file=sys.stderr)
        return 1
    except OSError as err:
        print(f"econ: {err}", file=sys.stderr)
        return 2
    print(render(doc))
    print(f"econ: wrote {out}", file=sys.stderr)
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="econ.py", description="The M3 economics report (S04 §4.9).")
    parser.add_argument("--records", nargs="+", required=True,
                        help="run records: .jsonl files, or directories of them or of run directories")
    parser.add_argument("--traces", nargs="*", default=[], help="policy traces from `roko learn self-model replay`")
    parser.add_argument("--out", default="econ-report.json", help="where to write the report")
    parser.add_argument("--b", type=int, default=B, help=f"bootstrap resamples (default {B})")
    parser.add_argument("--seed", type=int, default=0, help="the bootstrap's seed (default 0)")
    return parser


if __name__ == "__main__":
    sys.exit(main())
