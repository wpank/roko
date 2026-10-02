"""CUPED: a between-task contrast adjusted by one pre-assignment, arm-blind covariate (S09 §4.1, S03 §4.5;
`deng2013improving`; task 3335).

Two groups of tasks, each run by one arm (a between-task live contrast), give outcomes Y and a covariate X that was
fixed before the arms were assigned. With θ = Cov(X, Y) / Var(X) over both groups pooled, each outcome is adjusted to

    Y' = Y − θ·(X − X̄),

and the adjusted contrast is the difference in means of Y', (Ȳ_1 − Ȳ_2) − θ·(X̄_1 − X̄_2). Since X was fixed before
assignment it says nothing about which arm a task got, so the adjustment removes the outcome variance X explains and
leaves the contrast's expectation alone. The interval is Wald's on the adjusted outcomes, (Ȳ'_1 − Ȳ'_2) ± z·se with
se² = s'²_1/n_1 + s'²_2/n_2 (sample variances), θ taken as known (its estimation error is O(1/n), Deng et al. §3).
The unadjusted difference in means (DiM) and its interval are reported beside it, and so is S03 C7's variance
reduction factor VRF = Var(DiM) / Var(CUPED) (target ≥ 1.3).

**The covariate** (S09 §4.1, S03 §4.5). In bench campaigns, X_t is the task's LOG1 mean VS under `roko_fixed`
gpt-oss-120b, or its family × ℓ mean when LOG1 never ran the task, and the lock freezes it (`prereg.lock.json`);
`log1_covariates` computes it from LOG1's records. For the always-on production holdout it is S04's arm-blind
forecast. It is never measured after assignment, and never the arm's own outcome.

API:
    Cuped(estimate, se, ci, dim, dim_se, dim_ci, theta, vrf, n_first, n_second, alpha)
    adjusted_contrast(first, second, alpha=0.05) -> Cuped      # each a sequence of (y, x) per task
    log1_covariates(records, tasks, *, arm="roko_fixed", model="gpt-oss-120b")
        -> (covariates: dict[task, float], from_stratum: list[task])
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction
from statistics import NormalDist

import metrics

LOG1_ARM, LOG1_MODEL = "roko_fixed", "gpt-oss-120b"  # S09 §4.1: the bench covariate's arm and model


@dataclass(frozen=True)
class Cuped:
    estimate: float  # the adjusted contrast, first minus second
    se: float
    ci: tuple[float, float]
    dim: float  # the unadjusted difference in means
    dim_se: float
    dim_ci: tuple[float, float]
    theta: float  # Cov(X, Y) / Var(X), pooled
    vrf: float | None  # Var(DiM) / Var(CUPED); None when the adjusted variance is 0
    n_first: int
    n_second: int
    alpha: float


def adjusted_contrast(first: Sequence[tuple[float, float]], second: Sequence[tuple[float, float]],
                      alpha: float = 0.05) -> Cuped:
    """The CUPED contrast of `first` minus `second`, each a sequence of (outcome y, covariate x) per task."""
    first, second = [_pair(item) for item in first], [_pair(item) for item in second]
    if len(first) < 2 or len(second) < 2:
        raise ValueError(f"each group needs at least two tasks for a variance, not {len(first)} and {len(second)}")
    if not 0 < alpha < 1:
        raise ValueError(f"alpha must lie strictly between 0 and 1, not {alpha}")
    pooled = first + second
    x_mean = math.fsum(x for _, x in pooled) / len(pooled)
    y_mean = math.fsum(y for y, _ in pooled) / len(pooled)
    var_x = math.fsum((x - x_mean) ** 2 for _, x in pooled)
    if var_x == 0:
        raise ValueError("the covariate is constant over every task, so it cannot adjust anything")
    theta = math.fsum((x - x_mean) * (y - y_mean) for y, x in pooled) / var_x
    adjusted = [[y - theta * (x - x_mean) for y, x in group] for group in (first, second)]
    raw = [[y for y, _ in group] for group in (first, second)]
    z = NormalDist().inv_cdf(1 - alpha / 2)
    estimate, se = _difference(adjusted)
    dim, dim_se = _difference(raw)
    return Cuped(estimate=estimate, se=se, ci=(estimate - z * se, estimate + z * se), dim=dim, dim_se=dim_se,
                 dim_ci=(dim - z * dim_se, dim + z * dim_se), theta=theta,
                 vrf=dim_se ** 2 / se ** 2 if se > 0 else None, n_first=len(first), n_second=len(second),
                 alpha=alpha)


def log1_covariates(records: Iterable[dict], tasks: Iterable[Mapping], *, arm: str = LOG1_ARM,
                    model: str = LOG1_MODEL) -> tuple[dict[tuple, float], list[tuple]]:
    """Each task's covariate from LOG1's records: its mean VS- under `arm` with `model`, or the mean over its family x
    level stratum's tasks when LOG1 never ran it (module docstring). `tasks` are run records' `task` objects; the
    second value lists the tasks that took their stratum's mean. Raises MetricsError when a stratum has no LOG1 run."""
    records = list(records)
    models = metrics.run_models(records)
    labels: dict[tuple, list[int]] = {}
    strata: dict[tuple, set[tuple]] = {}
    for record in records:
        if (record["arm"] != arm or record["task"]["family"] == metrics.PLAN_SLICE
                or record["execution"]["status"] in metrics.EXCLUDED
                or models[(record["experiment_id"], record["arm"], record["run_id"])] != model):
            continue
        key = metrics.task_key(record)
        labels.setdefault(key, []).append(metrics.vs_minus(record))
        strata.setdefault((record["task"]["family"], record["task"]["ladder"]), set()).add(key)
    means = {key: float(Fraction(sum(runs), len(runs))) for key, runs in labels.items()}
    covariates, from_stratum = {}, []
    for task in tasks:
        key = (task["instance_id"], task["spec_variant"])
        if key in means:
            covariates[key] = means[key]
            continue
        stratum = (task["family"], task["ladder"])
        members = sorted(strata.get(stratum, ()))
        if not members:
            raise metrics.MetricsError(f"LOG1 has no {arm} {model} run in {stratum[0]}-l{stratum[1]}, so task {key[0]} "
                                       "has no covariate")
        covariates[key] = math.fsum(means[member] for member in members) / len(members)
        from_stratum.append(key)
    return dict(sorted(covariates.items())), sorted(set(from_stratum))


def _difference(groups: list[list[float]]) -> tuple[float, float]:
    """Mean of the first group minus the second's, and its standard error from the two sample variances."""
    means = [math.fsum(group) / len(group) for group in groups]
    variances = [math.fsum((value - mean) ** 2 for value in group) / (len(group) - 1)
                 for group, mean in zip(groups, means)]
    return means[0] - means[1], math.sqrt(sum(var / len(group) for var, group in zip(variances, groups)))


def _pair(item: tuple[float, float]) -> tuple[float, float]:
    y, x = item
    for name, value in (("outcome", y), ("covariate", x)):
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError(f"a task's {name} is a finite number, not {value!r}")
    return float(y), float(x)
