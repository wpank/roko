"""The report's numbers from run records: VS rate, $/VS, pass^k and false greens (S08 §4.12 and T7; paper App. D).

Every function reads `vb.run_record/1` dicts, as `records.jsonl` holds them (S08 §5.4), and follows Appendix D:

- **Which runs.** Rows of the plan-level slice (`task.family == "PL"`) never enter these metrics. Their `task.ladder`
  is a placeholder (5), and S09 §4.9 reports them only descriptively (`plan_slice`). `infra_error` and
  `leak_suspected` runs are excluded and counted. `aborted_cap` and `timeout` runs stay in with VS = 0, and they are
  counted as cap censoring (D.1).
- **Cells.** A cell is an arm, or an (arm, model) pair when the arm's runs in the experiment requested more than one
  model: S09 §4.2's `cheap_direct` runs gpt-oss-120b and the pool's best cheap model, and no metric pools them. A
  run's model is what its attempts requested (`execution.attempts[].model_requested`). One `vb run` pins one model,
  so a record with no attempt takes the model of its run's other records. Such a cell's filter adds
  `model == "<slug>"`, where `model` is that derived field; an arm that ran one model keeps its arm-only cell and
  filter. A routed arm (`ROUTED_ARMS`) may switch models within a run, so its cell is always the arm. A run of any
  other arm that requested two models is an error, and so is a run with no model call in an arm that ran two.
- **Labels.** The headline label VS- is `vs.label`, and 0 when `vs.unknown`. The bound VS+ is 1 when `vs.unknown`.
  Every label metric is computed with VS- and again with VS+, as `<metric>_unknown_as_1`, the result shown beside it.
- **VS rate** (D.1): the mean over tasks of each task's share of verified successes, c/n. A task is an (instance,
  spec variant) pair, and its runs are its (seed, replicate)s; a repeated one is an error, never a second seed. F8
  honeypots (`task.is_honeypot`) are left out and get their own honest-conflict rate.
- **pass^k** (D.3): `passk.pass_k` over the same tasks, per cell and per family x level stratum.
- **$/VS** (D.2): the sum of `costs.api_equiv_usd` over every included run, divided by the sum of VS-. It is null when
  any run's cost is unknown, since one unknown cost makes the cell unmeasurable and it is never computed from the
  rest, and when no run is a VS ("no VS"). `billed_usd` is summed apart and never enters $/VS. For runs that carry a
  vendor figure (`costs.vendor_usd`, such as Claude Code's `total_cost_usd`, R), U' stays the headline, and R/VS and
  delta_UR = |sum U' - sum R| / sum R are shown beside it.
- **False greens** (D.1): a reported pass with VS- = 0. For direct and CLI arms a reported pass is `visible.passed`,
  the census's run of the visible checks on the final tree. For Roko arms it is a final gate verdict of `passed`, and
  final verdicts of `unverified` and `forced_accept` are counted apart. The rate is a share of reported passes, not
  of runs.
- **The plan-level slice** (S09 §4.9, App. D.12; exploratory). Per feature and arm: the verified feature (the PL
  row's label), its cost and its makespan (`execution.finished_at` minus `started_at`). Per arm: the verified
  features out of the features run, with a Clopper-Pearson 95% interval; the cost per verified feature; and the
  median and range of the makespan. Then the `roko_plan` / `fd_claude` ratios as point estimates, and the
  discordant features. Queue waits are read from `execution.queue_wait_s`, which no runner records yet.

A `Cut` is a list of clauses over record fields and the runs they select. Every metric keeps its cut: the runs are
the runs behind the number, and the clauses' text is the MetricRecord's `record_filter`, so the filter shown is the
filter applied. A metric with no run behind it is not emitted.

API:
    Clause(field, op, value); Cut(clauses, rows); cut(records, clauses) -> Cut
    Metric(metric, value, n, estimator, cost_basis, cut, cell, ladder, ci, ci_method, model)
    vs_minus(record) -> int; vs_plus(record) -> int; reported_pass(record) -> bool
    check_unique(records) -> None                    # raises MetricsError when a run repeats or fits no cell
    run_models(records) -> {(experiment_id, arm, run_id): model | None}
    with_models(records) -> list[dict]               # copies, each with its cell's `model`
    cells(records) -> list[(arm, model | None)]      # the report's cells outside the plan-level slice
    cell_name(arm, model) -> str
    arm_metrics(records, experiment_id, arm, ks=(3,), model=None) -> list[Metric]
    false_greens(records) -> list[dict]; excluded(records) -> list[dict]
    plan_slice(records, experiment_id) -> (section: dict | None, metrics: list[Metric])
    clopper_pearson(successes, n, alpha=0.05) -> (low, high)
"""

from __future__ import annotations

import dataclasses
import datetime as dt
import json
import math
import statistics
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from fractions import Fraction

import passk

PLAN_SLICE = "PL"
EXCLUDED = ("infra_error", "leak_suspected")
CENSORED = ("aborted_cap", "timeout")
# S09 §4.2's arm registry: these arms run Roko, and their reported pass is the gate's final verdict.
ROKO_ARMS = frozenset({"roko_fixed", "roko_full", "fr_claude", "roko_plan"})
# S09 §4.2 and §4.9: arms whose runs may switch models (routing, the tier ladder, a planner, escalation).
ROUTED_ARMS = frozenset({"roko_full", "roko_plan", "hybrid"})
NOT_PASSED = ("unverified", "forced_accept")
PL_RATIO = ("roko_plan", "fd_claude")  # S09 §4.9: the slice's ratios are roko_plan / fd_claude


class MetricsError(ValueError):
    """The records cannot be reported honestly, for example because a run repeats."""


@dataclass(frozen=True)
class Clause:
    field: str  # a dotted path into a run record, such as "task.ladder"
    op: str  # "==", "!=", "in" or "not in"
    value: object

    def test(self, record: dict) -> bool:
        actual = record
        for key in self.field.split("."):
            actual = actual[key]
        if self.op == "==":
            return actual == self.value
        if self.op == "!=":
            return actual != self.value
        if self.op in ("in", "not in"):
            return (actual in self.value) == (self.op == "in")
        raise ValueError(f"unknown operator {self.op!r}")

    def __str__(self) -> str:
        return f"{self.field} {self.op} {json.dumps(self.value)}"


@dataclass(frozen=True)
class Cut:
    clauses: tuple[Clause, ...]
    rows: tuple[dict, ...]  # the records the clauses select

    @property
    def filter(self) -> str:
        return " and ".join(map(str, self.clauses))

    def narrow(self, *clauses: Clause) -> Cut:
        return cut(self.rows, (*self.clauses, *clauses))


def cut(records: Iterable[dict], clauses: Sequence[Clause]) -> Cut:
    return Cut(tuple(clauses), tuple(record for record in records if all(c.test(record) for c in clauses)))


@dataclass(frozen=True)
class Metric:
    metric: str
    value: float | None
    n: int
    estimator: str
    cost_basis: str | None  # "api_equiv_usd" or "billed_usd"; None for a metric that is not a cost
    cut: Cut  # the filter and the runs behind the number
    cell: str  # "all", "l<level>", "<family>-l<level>" or "PL"
    ladder: int | None = None
    ci: tuple[float, float] | None = None
    ci_method: str = "none"
    model: str | None = None  # the cell's model when its arm ran more than one, else None


def vs_minus(record: dict) -> int:
    """The headline label: an unknown label counts as 0."""
    return 0 if record["vs"]["unknown"] else record["vs"]["label"]


def vs_plus(record: dict) -> int:
    """The bound: an unknown label counts as 1."""
    return 1 if record["vs"]["unknown"] else record["vs"]["label"]


# (label, metric-name suffix, what the estimator says about unknown labels)
LABELS = ((vs_minus, "", "unknown labels count as 0"),
          (vs_plus, "_unknown_as_1", "unknown labels count as 1 (the bound beside the headline)"))


def final_verdict(record: dict) -> str | None:
    attempts = record["execution"]["attempts"]
    return attempts[-1].get("gate_verdict") if attempts else None


def reported_pass(record: dict) -> bool:
    """Whether the arm reported success: a final gate verdict of `passed` for Roko arms, else the visible checks."""
    if record["arm"] in ROKO_ARMS:
        return final_verdict(record) == "passed"
    return record["visible"]["passed"] is True


def task_key(record: dict) -> tuple[str, str]:
    return record["task"]["instance_id"], record["task"]["spec_variant"]


def check_unique(records: Iterable[dict]) -> None:
    """Raise MetricsError when a cell ran one task twice with the same seed and replicate, or a run fits no cell."""
    seen: dict[tuple, str] = {}
    for record in with_models(records):
        key = (record["arm"], record["model"], *task_key(record), record["seed"], record["replicate"])
        if key in seen:
            raise MetricsError(f"arm {cell_name(key[0], key[1])} ran {key[2]} ({key[3]}) with seed {key[4]} and "
                               f"replicate {key[5]} twice (runs {seen[key]} and {record['run_id']}); a repeat is not "
                               "an independent run")
        seen[key] = record["run_id"]


def run_models(records: Iterable[dict]) -> dict[tuple[str, str, str], str | None]:
    """Each run's model, keyed by (experiment, arm, run id): the one model its attempts requested (a `vb run` pins
    one). None for a routed arm's run and for a run that made no attempt. A run of any other arm that requested two
    models raises MetricsError."""
    requested: dict[tuple[str, str, str], set[str]] = {}
    for record in records:
        key = (record["experiment_id"], record["arm"], record["run_id"])
        attempts = record["execution"]["attempts"]
        requested.setdefault(key, set()).update(attempt["model_requested"] for attempt in attempts)
    models: dict[tuple[str, str, str], str | None] = {}
    for (experiment_id, arm, run_id), found in requested.items():
        if len(found) > 1 and arm not in ROUTED_ARMS:
            raise MetricsError(f"run {run_id} of arm {arm} requested {len(found)} models ({', '.join(sorted(found))}); "
                               f"only a routed arm ({', '.join(sorted(ROUTED_ARMS))}) may switch models within a run")
        models[(experiment_id, arm, run_id)] = found.pop() if len(found) == 1 and arm not in ROUTED_ARMS else None
    return models


def with_models(records: Iterable[dict]) -> list[dict]:
    """Copies of the records, each with `model`, its cell's model: its run's model when its arm ran more than one
    model in the experiment, else None. Raises MetricsError for a run that has no cell (module docstring)."""
    records = list(records)
    by_run = run_models(records)
    ran: dict[tuple[str, str], set[str]] = {}
    for (experiment_id, arm, _), model in by_run.items():
        if model is not None:
            ran.setdefault((experiment_id, arm), set()).add(model)
    out = []
    for record in records:
        models = ran.get((record["experiment_id"], record["arm"]), set())
        model = by_run[(record["experiment_id"], record["arm"], record["run_id"])] if len(models) > 1 else None
        if len(models) > 1 and model is None:
            raise MetricsError(f"run {record['run_id']} of arm {record['arm']} made no model call, so it belongs to "
                               f"none of the models the arm ran ({', '.join(sorted(models))})")
        out.append({**record, "model": model})
    return out


def cell_name(arm: str, model: str | None) -> str:
    """A cell as the report names it: the arm, with its model in parentheses when it has one."""
    return f"{arm} ({model})" if model else arm


def cells(records: Iterable[dict]) -> list[tuple[str, str | None]]:
    """The report's cells outside the plan-level slice: (arm, None) for an arm that ran one model or routes between
    models, and (arm, model) for each model of an arm that ran more than one."""
    return sorted({(row["arm"], row["model"]) for row in with_models(records) if row["task"]["family"] != PLAN_SLICE},
                  key=lambda cell: (cell[0], cell[1] or ""))


def vs_rate(records: Iterable[dict], label: Callable[[dict], int] = vs_minus) -> float | None:
    """App. D.1: the mean over tasks of c/n; None without runs."""
    tasks = _labels_by_task(records, label)
    if not tasks:
        return None
    return float(sum((Fraction(sum(runs), len(runs)) for runs in tasks.values()), Fraction(0)) / len(tasks))


def arm_metrics(records: Iterable[dict], experiment_id: str, arm: str, ks: Sequence[int] = (3,),
                model: str | None = None) -> list[Metric]:
    """Every metric of one cell outside the plan-level slice: overall, at each level, and per family x level. The
    cell is the arm, or the arm and `model` when the arm ran more than one model (see `cells`)."""
    rows = with_models(records)
    ran = sorted({row["model"] for row in rows if row["experiment_id"] == experiment_id and row["arm"] == arm} - {None})
    if (model is None and ran) or (model is not None and model not in ran):
        raise MetricsError(f"arm {arm} ran {', '.join(ran) if ran else 'one model'} in {experiment_id}; its cells "
                           f"are {', '.join(cell_name(arm, name) for name in ran) if ran else arm}, not "
                           f"{cell_name(arm, model)}")
    clauses = [Clause("experiment_id", "==", experiment_id), Clause("arm", "==", arm)]
    if model is not None:
        clauses.append(Clause("model", "==", model))
    base = cut(rows, (*clauses, Clause("task.family", "!=", PLAN_SLICE)))
    out = _cell_metrics(base, arm, ks, "all", None)
    for level in sorted({row["task"]["ladder"] for row in base.rows}):
        out += _cell_metrics(base.narrow(Clause("task.ladder", "==", level)), arm, ks, f"l{level}", level)
    for family, level in sorted({(row["task"]["family"], row["task"]["ladder"]) for row in base.rows}):
        stratum = base.narrow(Clause("task.family", "==", family), Clause("task.ladder", "==", level))
        out += _label_metrics(_kept(stratum), ks, f"{family}-l{level}", level)
    return [dataclasses.replace(metric, model=model) for metric in out]


def false_greens(records: Iterable[dict]) -> list[dict]:
    """Every false green outside the plan-level slice among the included runs, with its run id (App. D.1)."""
    found = [dict(_identity(row), reported_by="gate" if row["arm"] in ROKO_ARMS else "visible",
                  unknown=row["vs"]["unknown"], failed=list(row["vs"]["failed"]))
             for row in with_models(records)
             if row["task"]["family"] != PLAN_SLICE and row["execution"]["status"] not in EXCLUDED
             and reported_pass(row) and not vs_minus(row)]
    return sorted(found, key=_order)


def excluded(records: Iterable[dict]) -> list[dict]:
    """Every run excluded from the metrics (`infra_error`, `leak_suspected`), with its run id and reason."""
    found = [dict(_identity(row), failed=list(row["vs"]["failed"]))
             for row in with_models(records) if row["execution"]["status"] in EXCLUDED]
    return sorted(found, key=_order)


def plan_slice(records: Iterable[dict], experiment_id: str) -> tuple[dict | None, list[Metric]]:
    """S09 §4.9's descriptive report of the plan-level slice rows, and its numbers as metrics labelled PL."""
    base = cut(records, (Clause("experiment_id", "==", experiment_id), Clause("task.family", "==", PLAN_SLICE)))
    if not base.rows:
        return None, []
    kept = _kept(base)
    features = sorted({(row["task"]["instance_id"], row["seed"]) for row in kept.rows})
    seeds = {seed for _, seed in features}
    label = (f"PL (exploratory, {'one' if len(seeds) == 1 else len(seeds)} seed{'s' * (len(seeds) != 1)}, "
             f"{len(features)} feature{'s' * (len(features) != 1)})")
    arms = sorted({row["arm"] for row in kept.rows})
    rows_of = {(row["arm"], row["task"]["instance_id"], row["seed"]): row for row in kept.rows}
    table = [{"feature": feature, "seed": seed,
              "arms": {arm: _feature_cell(rows_of.get((arm, feature, seed))) for arm in arms}}
             for feature, seed in features]

    out: list[Metric] = []
    summary = {}
    for arm in arms:
        mine = kept.narrow(Clause("arm", "==", arm))
        rows = mine.rows
        vf = sum(map(vs_minus, rows))
        spans = [span for span in map(makespan_s, rows) if span is not None]
        cpf, why = _per_vs([row["costs"]["api_equiv_usd"] for row in rows], vf, "a known cost", "no verified feature")
        statuses = [row["execution"]["status"] for row in base.narrow(Clause("arm", "==", arm)).rows]
        summary[arm] = {
            "features": len(rows), "verified": vf, "verified_unknown_as_1": sum(map(vs_plus, rows)),
            "verified_ci95": list(clopper_pearson(vf, len(rows))), "cpf_usd": cpf, "cpf_note": why.removeprefix("; "),
            "makespan_median_s": statistics.median(spans) if spans else None,
            "makespan_range_s": [min(spans), max(spans)] if spans else None, "makespan_recorded": len(spans),
            "cap_censored": sum(row["execution"]["status"] in CENSORED for row in rows),
            **{status: statuses.count(status) for status in EXCLUDED},
        }
        out += _metric(mine, "pl_verified_features", vf, len(rows), f"{label}: the sum of VF over the features run "
                       "(App. D.12); unknown labels count as 0", "PL")
        for label_of, suffix, note in LABELS:
            hits = sum(map(label_of, rows))
            out += _metric(mine, f"pl_vf_rate{suffix}", hits / len(rows), len(rows), f"{label}: verified features / "
                           f"features run, with a Clopper-Pearson 95% interval; {note}", "PL",
                           ci=clopper_pearson(hits, len(rows)), ci_method="clopper_pearson")
        out += _metric(mine, "pl_cpf_usd", cpf, len(rows), f"{label}: the sum of costs.api_equiv_usd over the arm's "
                       f"runs / the sum of VF (App. D.12){why}", "PL", cost_basis="api_equiv_usd")
        vendor = [row["costs"]["vendor_usd"] for row in rows]
        if any(amount is not None for amount in vendor):
            summary[arm]["cpf_vendor_usd"], why = _per_vs(vendor, vf, "a vendor figure", "no verified feature")
            out += _metric(mine, "pl_cpf_vendor_usd", summary[arm]["cpf_vendor_usd"], len(rows), f"{label}: the sum "
                           f"of costs.vendor_usd (R) / the sum of VF, beside the U' headline{why}", "PL",
                           cost_basis="api_equiv_usd")
        if spans:
            for name, value in (("median", statistics.median(spans)), ("min", min(spans)), ("max", max(spans))):
                out += _metric(mine, f"pl_makespan_{name}_s", value, len(spans), f"{label}: the {name} over features "
                               "of execution.finished_at - execution.started_at, in seconds", "PL")

    ratios = {}
    if all(arm in summary for arm in PL_RATIO):
        top, bottom = (summary[arm] for arm in PL_RATIO)
        pair = kept.narrow(Clause("arm", "in", list(PL_RATIO)))
        for key, name in (("cpf_usd", "pl_cpf_ratio"), ("makespan_median_s", "pl_makespan_median_ratio")):
            ratios[name] = top[key] / bottom[key] if top[key] is not None and bottom[key] else None
            out += _metric(pair, name, ratios[name], len(features), f"{label}: {key} of roko_plan / {key} of "
                           "fd_claude, a point estimate with no interval (S09 §4.9)", "PL",
                           cost_basis="api_equiv_usd" if key == "cpf_usd" else None)
    ran = {arm: {(row["task"]["instance_id"], row["seed"]) for row in kept.rows if row["arm"] == arm} for arm in arms}
    verified = {arm: {(row["task"]["instance_id"], row["seed"]) for row in kept.rows
                      if row["arm"] == arm and vs_minus(row)} for arm in arms}
    discordant = {}
    for arm in arms:  # verified by this arm, run but not verified by another
        others = [other for other in arms if other != arm]
        discordant[arm] = sorted(feature for feature, seed in verified[arm]
                                 if any((feature, seed) in ran[o] - verified[o] for o in others))
    section = {
        "label": label,
        "features": [feature for feature, _ in features],
        "table": table,
        "arms": summary,
        "ratios": ratios,
        "discordant": discordant,
        "both_verified": sorted(feature for feature, _ in set.intersection(*verified.values())) if len(arms) > 1
        else [],
        "plan_level_false_greens": sorted(f"{row['arm']}:{row['task']['instance_id']}" for row in kept.rows
                                          if row["arm"] in ROKO_ARMS and reported_pass(row) and not vs_minus(row)),
        "not_recorded": "the planner's cost share, realized parallelism, escalations and gate-rejected integrations "
                        "need per-class costs and task timings that run records do not carry yet",
    }
    return section, out


def makespan_s(record: dict) -> float | None:
    """Seconds from `execution.started_at` to `finished_at`; None when either is missing or unreadable."""
    try:
        start, end = (dt.datetime.fromisoformat(record["execution"][key]) for key in ("started_at", "finished_at"))
        return (end - start).total_seconds()
    except (KeyError, TypeError, ValueError):
        return None


def clopper_pearson(successes: int, n: int, alpha: float = 0.05) -> tuple[float, float]:
    """The exact (1 - alpha) interval for a binomial proportion (App. D.9), by bisection on the binomial tails."""
    if n < 1 or not 0 <= successes <= n:
        raise ValueError(f"need 0 <= successes <= n and n >= 1, not {successes} of {n}")

    def tail(p: float, low: int, high: int) -> float:
        return sum(math.comb(n, i) * p ** i * (1 - p) ** (n - i) for i in range(low, high + 1))

    low = 0.0 if successes == 0 else _bisect(lambda p: tail(p, successes, n) >= alpha / 2)
    high = 1.0 if successes == n else _bisect(lambda p: tail(p, 0, successes) < alpha / 2)
    return low, high


def _cell_metrics(base: Cut, arm: str, ks: Sequence[int], cell: str, ladder: int | None) -> list[Metric]:
    kept = _kept(base)
    runs = kept.rows
    out = _label_metrics(kept, ks, cell, ladder)

    def add(metric: str, value: float | None, n: int, estimator: str, where: Cut = kept, **extra) -> None:
        out.extend(_metric(where, metric, value, n, estimator, cell, ladder=ladder, **extra))

    vs = sum(map(vs_minus, runs))
    u = [row["costs"]["api_equiv_usd"] for row in runs]
    value, why = _per_vs(u, vs, "a known cost", "no VS")
    add("usd_per_vs", value, len(runs), f"the sum of costs.api_equiv_usd over every run / the sum of VS (App. D.2); "
        f"unknown labels count as 0{why}", cost_basis="api_equiv_usd")
    add("spend_usd", _total(u), len(runs), "the sum of costs.api_equiv_usd; null when any run's cost is unknown",
        cost_basis="api_equiv_usd")
    add("spend_usd", _total([row["costs"]["billed_usd"] for row in runs]), len(runs), "the sum of costs.billed_usd, "
        "reported apart and never part of $/VS; null when any run's bill is unknown", cost_basis="billed_usd")
    vendor = [row["costs"]["vendor_usd"] for row in runs]
    if any(amount is not None for amount in vendor):
        value, why = _per_vs(vendor, vs, "a vendor figure", "no VS")
        add("usd_per_vs_vendor", value, len(runs), f"the sum of costs.vendor_usd (R, the vendor's own figure) / the sum "
            f"of VS, beside the U' headline (App. D.2){why}", cost_basis="api_equiv_usd")
        sum_u, sum_r = _total(u), _total(vendor)
        gap = abs(sum_u - sum_r) / sum_r if sum_u is not None and sum_r else None
        add("cost_gap_ur", gap, len(runs), "|sum U' - sum R| / sum R, with U' = costs.api_equiv_usd and R = "
            "costs.vendor_usd (App. D.2)" + ("" if gap is not None else "; null: a figure is unknown or R is 0"),
            cost_basis="api_equiv_usd")

    passes = [row for row in runs if reported_pass(row)]
    add("false_greens", sum(1 - vs_minus(row) for row in passes), len(runs), "reported passes with VS = 0 (App. D.1): "
        "visible.passed for direct and CLI arms, a final gate verdict of passed for Roko arms; unknown labels count "
        "as 0")
    for label, suffix, note in LABELS:
        rate = sum(1 - label(row) for row in passes) / len(passes) if passes else None
        add(f"false_green_rate{suffix}", rate, len(passes), "false greens / reported passes, a share of passes and not "
            f"of runs (App. D.1); {note}" + ("" if passes else "; null: no reported pass"))
    if arm in ROKO_ARMS:
        add("unverified_runs", sum(final_verdict(row) in NOT_PASSED for row in runs), len(runs), "runs whose final "
            "gate verdict is unverified or forced_accept: counted apart, never as passes (App. D.1)")
    add("cap_censored_runs", sum(row["execution"]["status"] in CENSORED for row in runs), len(runs), "runs that ended "
        "aborted_cap or timeout; they stay in with VS = 0 (App. D.1)")
    for status in EXCLUDED:
        add(f"{status}_runs", sum(row["execution"]["status"] == status for row in base.rows), len(base.rows),
            f"runs with status {status}, excluded from every other metric (S08 §4.12)", where=base)
    add("estimated_cost_runs", sum(row["costs"]["source"] == "estimated" for row in runs), len(runs), "runs whose cost "
        "source is estimated: flagged, not measured (App. D.2)")
    add("unknown_cost_runs", sum(amount is None for amount in u), len(runs), "runs whose cost is unknown (null); one "
        "makes $/VS unmeasurable (App. D.2)")
    return out


def _label_metrics(kept: Cut, ks: Sequence[int], cell: str, ladder: int | None) -> list[Metric]:
    """VS rate and pass^k, headline and bound, over the non-honeypot tasks; the honest-conflict rate over F8's."""
    out: list[Metric] = []
    resolve = kept.narrow(Clause("task.is_honeypot", "==", False))
    honeypot = kept.narrow(Clause("task.is_honeypot", "==", True))
    for label, suffix, note in LABELS:
        out += _metric(resolve, f"vs_rate{suffix}", vs_rate(resolve.rows, label), len(resolve.rows), "the mean over "
                       f"tasks of c/n, each task's share of verified successes (App. D.1); F8 honeypots excluded; {note}",
                       cell, ladder=ladder)
        out += _metric(honeypot, f"honest_conflict_rate{suffix}", vs_rate(honeypot.rows, label), len(honeypot.rows),
                       f"vs_rate over F8 honeypot tasks, reported beside the resolve rate (S08 §4.3); {note}", cell,
                       ladder=ladder)
    for k in ks:
        for label, suffix, note in LABELS:
            result = passk.pass_k(_labels_by_task(resolve.rows, label), k)
            skipped = f"; {len(result.skipped)} task(s) with fewer than {k} runs skipped" if result.skipped else ""
            empty = f"; null: no task has {k} runs" if result.value is None else ""
            out += _metric(resolve, f"pass_hat_{k}{suffix}", result.value, len(result.tasks), f"the mean over tasks of "
                           f"C(c,{k})/C(n,{k}) (App. D.3){skipped}{empty}; {note}", cell, ladder=ladder)
    return out


def _metric(where: Cut, metric: str, value: float | None, n: int, estimator: str, cell: str, *,
            cost_basis: str | None = None, ladder: int | None = None, ci: tuple[float, float] | None = None,
            ci_method: str = "none") -> list[Metric]:
    """The metric as a one-item list, or an empty list when no run is behind it."""
    if not where.rows:
        return []
    return [Metric(metric, value, n, estimator, cost_basis, where, cell, ladder, ci, ci_method)]


def _kept(base: Cut) -> Cut:
    return base.narrow(Clause("execution.status", "not in", list(EXCLUDED)))


def _labels_by_task(records: Iterable[dict], label: Callable[[dict], int]) -> dict[tuple, list[int]]:
    tasks: dict[tuple, list[int]] = {}
    for record in records:
        tasks.setdefault(task_key(record), []).append(label(record))
    return tasks


def _per_vs(costs: list[float | None], vs: int, what: str, none: str) -> tuple[float | None, str]:
    """(the summed cost per success, "") or (None, why it is null)."""
    missing = sum(cost is None for cost in costs)
    if missing:
        return None, f"; null: {missing} run(s) without {what}, so the cell is unmeasurable"
    if not vs:
        return None, f"; null: {none}"
    return math.fsum(costs) / vs, ""


def _total(amounts: list[float | None]) -> float | None:
    return None if any(amount is None for amount in amounts) else math.fsum(amounts)


def _feature_cell(row: dict | None) -> dict | None:
    if row is None:
        return None  # the arm did not run this feature
    return {"vf": vs_minus(row), "unknown": row["vs"]["unknown"], "status": row["execution"]["status"],
            "cost_usd": row["costs"]["api_equiv_usd"], "vendor_usd": row["costs"]["vendor_usd"],
            "makespan_s": makespan_s(row), "queue_wait_s": row["execution"].get("queue_wait_s"),
            "run_id": row["run_id"], "record_id": row["record_id"]}


def _identity(row: dict) -> dict:
    """The run's identity in the false-green and excluded lists; `model` only when its arm ran more than one."""
    task = row["task"]
    model = {"model": row["model"]} if row.get("model") is not None else {}
    return {"arm": row["arm"], **model, "run_id": row["run_id"], "record_id": row["record_id"],
            "instance_id": task["instance_id"], "spec_variant": task["spec_variant"], "family": task["family"],
            "ladder": task["ladder"], "seed": row["seed"], "replicate": row["replicate"],
            "status": row["execution"]["status"]}


def _order(item: dict) -> tuple:
    return (item["arm"], item.get("model") or "", item["instance_id"], item["spec_variant"], item["seed"],
            item["replicate"], item["run_id"])


def _bisect(reached: Callable[[float], bool]) -> float:
    """The p in [0, 1] where the monotone predicate `reached` turns true."""
    low, high = 0.0, 1.0
    for _ in range(60):
        middle = (low + high) / 2
        low, high = (low, middle) if reached(middle) else (middle, high)
    return (low + high) / 2
