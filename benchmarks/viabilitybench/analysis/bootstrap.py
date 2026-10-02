"""The paired bootstrap, stratified by family and level (S08 S4.1; S09 S4.1, S4.3; gap-46fd19 / work item 3320).

Every interval on the pilot page, the envelope (S09's H1, task 3338) and the final report rests on this: a
caller-supplied `statistic` over a bag of `vb.run_record/1` rows (as `metrics.py` reads them), resampled the way
the campaign is designed to vary:

- **Strata** (default family x level, i.e. `row["task"]["family"]` and `row["task"]["ladder"]`) are never mixed: a
  replicate resamples within each stratum on its own, and the resampled rows from every stratum are pooled before
  `statistic` runs on them.
- **Tasks** (by default `(row["task"]["instance_id"], row["task"].get("spec_variant"))`, `metrics.task_key`'s
  convention) are resampled with replacement inside their stratum, as many draws as the stratum has tasks.
- **Seeds** are resampled with replacement inside their task, as many draws as the task has seeds. Every row of the
  drawn (task, seed) pair comes along together, whatever arm it is, so a statistic that reads more than one arm's
  rows for the same task and seed stays paired across a replicate: this is what "paired" means here, since every
  arm runs the same instances. `statistic` does its own pairing (matching rows by arm within what it is given); this
  module only decides which (task, seed) pairs a replicate sees.

B = 10,000 replicates by default (S08 S4.1), each a deterministic draw from `random.Random(seed)`, so the same
inputs and the same seed give the same bounds on every call and every platform (stdlib's `random.Random` promises
that, same as `common/hmac_seed.py`, though this module has nothing to do with the benchmark secret and never needs
HMAC: nobody has to reproduce a bootstrap replicate from an id alone).

Two kinds of interval:
- **Percentile** (default): the `alpha/2` and `1 - alpha/2` quantiles of the replicate statistics.
- **BCa** (`method="bca"`), for ratio statistics such as a VS ratio or a $/VS ratio, whose sampling distribution is
  skewed enough that the plain percentile interval is biased. The bias-correction z0 and the acceleration a (from a
  leave-one-task-out jackknife of `statistic`, dropping one task's rows at a time from the full, unresampled data)
  follow Efron & Tibshirani (1993) ch. 14; `a` needs at least two jackknife values that differ, so a run with one
  task raises BootstrapError.

API:
    BootstrapError(ValueError)
    BootstrapResult(estimate, low, high, b, alpha, method, replicates)
    paired_bootstrap(rows, statistic, strata=("family", "ladder"), b=10_000, seed=0, alpha=0.05,
                     method="percentile") -> BootstrapResult
    task_key(row) -> tuple                          # metrics.task_key's convention, read locally (no import cycle)
"""

from __future__ import annotations

import math
from collections.abc import Callable, Hashable, Mapping, Sequence
from dataclasses import dataclass
from random import Random
from statistics import NormalDist

_NORMAL = NormalDist()
DEFAULT_STRATA = ("family", "ladder")
DEFAULT_B = 10_000
DEFAULT_ALPHA = 0.05


class BootstrapError(ValueError):
    """The rows, the strata or the statistic cannot support a bootstrap replicate."""


@dataclass(frozen=True)
class BootstrapResult:
    estimate: float  # statistic(rows): the observed value, never resampled
    low: float
    high: float
    b: int
    alpha: float
    method: str  # "percentile" or "bca"
    replicates: tuple[float, ...]  # length b, in draw order


def task_key(row: Mapping) -> tuple:
    """(instance_id, spec_variant) of a run record, `metrics.task_key`'s convention, without importing it: a
    stdlib-only module stays free of a cycle with the rest of `analysis/`."""
    return row["task"]["instance_id"], row["task"].get("spec_variant")


def paired_bootstrap(rows: Sequence[Mapping], statistic: Callable[[Sequence[Mapping]], float],
                     strata: Sequence[str] = DEFAULT_STRATA, b: int = DEFAULT_B, seed: int = 0,
                     alpha: float = DEFAULT_ALPHA, method: str = "percentile") -> BootstrapResult:
    """The paired bootstrap of `statistic` over `rows` (see the module docstring).

    `statistic` takes a sequence of rows (a plain list, not grouped) and returns one float; it is called once on
    `rows` for the point estimate, and once per replicate on a resampled bag of the same rows (some repeated, some
    absent, as a bootstrap draws).
    """
    rows = list(rows)
    if not rows:
        raise BootstrapError("no rows to bootstrap")
    if b < 1:
        raise BootstrapError(f"b must be at least 1, not {b}")
    if not 0 < alpha < 1:
        raise BootstrapError(f"alpha must lie strictly between 0 and 1, not {alpha}")
    tree = _tree(rows, strata)
    estimate = float(statistic(rows))
    rng = Random(seed)
    replicates = tuple(float(statistic(_resample(tree, rng))) for _ in range(b))
    if method == "percentile":
        low, high = _percentile(replicates, 100 * alpha / 2), _percentile(replicates, 100 * (1 - alpha / 2))
    elif method == "bca":
        low, high = _bca_bounds(rows, strata, statistic, estimate, replicates, alpha)
    else:
        raise BootstrapError(f"method must be 'percentile' or 'bca', not {method!r}")
    return BootstrapResult(estimate=estimate, low=low, high=high, b=b, alpha=alpha, method=method,
                           replicates=replicates)


# --- the resampling tree --------------------------------------------------------------------------------------


def _tree(rows: Sequence[Mapping], strata: Sequence[str]) -> dict[tuple, dict[tuple, dict[int, list[Mapping]]]]:
    """stratum -> task -> seed -> its rows. Every grouping key is hashable and the groups are built once; a
    replicate only ever reads this structure, so resampling itself does no re-grouping."""
    tree: dict[tuple, dict[tuple, dict[int, list[Mapping]]]] = {}
    for row in rows:
        stratum = _stratum(row, strata)
        task = task_key(row)
        seed = row["seed"]
        tree.setdefault(stratum, {}).setdefault(task, {}).setdefault(seed, []).append(row)
    return tree


def _stratum(row: Mapping, strata: Sequence[str]) -> tuple:
    try:
        return tuple(row["task"][name] for name in strata)
    except KeyError as err:
        raise BootstrapError(f"a row's task has no {err}; strata are {', '.join(strata)}") from None


def _resample(tree: Mapping[tuple, Mapping[tuple, Mapping[int, list[Mapping]]]], rng: Random) -> list[Mapping]:
    """One bootstrap replicate: every stratum resampled on its own, then pooled."""
    drawn: list[Mapping] = []
    for stratum in sorted(tree, key=repr):
        by_task = tree[stratum]
        tasks = sorted(by_task, key=repr)
        for _ in range(len(tasks)):
            task = tasks[rng.randrange(len(tasks))]
            by_seed = by_task[task]
            seeds = sorted(by_seed)
            for _ in range(len(seeds)):
                drawn.extend(by_seed[seeds[rng.randrange(len(seeds))]])
    return drawn


# --- percentile and BCa bounds ----------------------------------------------------------------------------------


def _percentile(values: Sequence[float], pct: float) -> float:
    """The `pct`th percentile of `values` (0-100), linear interpolation between order statistics."""
    data = sorted(values)
    if not data:
        raise BootstrapError("no replicates to take a percentile of")
    rank = (max(0.0, min(100.0, pct)) / 100) * (len(data) - 1)
    low, high = math.floor(rank), math.ceil(rank)
    if low == high:
        return data[low]
    return data[low] + (rank - low) * (data[high] - data[low])


def _bca_bounds(rows: Sequence[Mapping], strata: Sequence[str], statistic: Callable[[Sequence[Mapping]], float],
                estimate: float, replicates: Sequence[float], alpha: float) -> tuple[float, float]:
    """Efron & Tibshirani (1993) ch. 14: bias-correction z0 from the replicates, acceleration a from a
    leave-one-task-out jackknife."""
    thetas = _jackknife(rows, strata, statistic)
    if len(set(thetas)) < 2:
        raise BootstrapError("BCa needs at least two tasks whose jackknife statistic differs")
    less = sum(1 for value in replicates if value < estimate)
    p0 = min(max(less / len(replicates), 1 / (2 * len(replicates))), 1 - 1 / (2 * len(replicates)))
    z0 = _NORMAL.inv_cdf(p0)
    mean_theta = sum(thetas) / len(thetas)
    deviations = [mean_theta - theta for theta in thetas]
    numerator = sum(deviation ** 3 for deviation in deviations)
    denominator = 6 * sum(deviation ** 2 for deviation in deviations) ** 1.5
    acceleration = numerator / denominator if denominator else 0.0

    def quantile(level: float) -> float:
        z = z0 + _NORMAL.inv_cdf(level)
        denom = 1 - acceleration * z
        if denom == 0:
            raise BootstrapError("BCa's acceleration makes the adjusted quantile undefined at this alpha")
        return _NORMAL.cdf(z0 + z / denom)

    return _percentile(replicates, 100 * quantile(alpha / 2)), _percentile(replicates, 100 * quantile(1 - alpha / 2))


def _jackknife(rows: Sequence[Mapping], strata: Sequence[str],
              statistic: Callable[[Sequence[Mapping]], float]) -> list[float]:
    """`statistic` with each task's rows left out in turn, across every stratum (the jackknife units are tasks,
    not individual rows, matching what the bootstrap resamples)."""
    by_task: dict[Hashable, list[int]] = {}
    for index, row in enumerate(rows):
        by_task.setdefault((_stratum(row, strata), task_key(row)), []).append(index)
    thetas = []
    for task, indices in sorted(by_task.items(), key=repr):
        excluded = set(indices)
        remaining = [row for index, row in enumerate(rows) if index not in excluded]
        if not remaining:
            raise BootstrapError(f"task {task[1]!r} is every row of its stratum; the jackknife needs rows left over")
        thetas.append(float(statistic(remaining)))
    return thetas
