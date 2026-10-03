"""Secondary analyses: the logistic GLMM with a random intercept per task, and the ICC of VS across seeds (S09 §4.1,
§4.4 H2 and H3; task 3337 under decision 3336).

numpy and scipy, pinned in `requirements-analysis.lock`, are imported only here under `analysis/models/` (decision
3336): no primary analysis depends on them.

**The model.** For observation i of task g,

    logit P(VS_i = 1) = x_i'β + u_g,   u_g ~ N(0, σ²), independent across tasks,

fit by maximum likelihood. Each task's integral over u_g = σ·z uses adaptive Gauss–Hermite quadrature (`nodes`
points, 25 by default, as lme4's `glmer(nAGQ = 25)`): the integrand's mode and curvature in z centre and scale the
nodes, so few nodes integrate it closely. scipy's L-BFGS-B maximizes over (β, σ) with σ ≥ 0, as lme4 does, so a task
variance of 0 shows as σ̂ = 0 at that bound (`at_boundary`), where the model is plain logistic regression and the
quadrature is exact. Standard errors come from the inverse observed information, a central-difference Hessian of the
negative log-likelihood, and the intervals are Wald's on the log-odds scale; at the bound σ has no standard error.

**H3's secondary** (`h3_rows`, `h3_interaction`): VS ~ spec × tier + (1 | task) over H3's cells. spec is 1 for
`precise` and 0 for `vague`, tier 1 for cheap (`roko_fixed` on gpt-oss-120b) and 0 for frontier (`fr_claude`); the
task is the instance, shared by both variants and tiers. The `spec:tier` coefficient is the log-odds analogue of the
primary's Δ = (VS_P − VS_V)_cheap − (VS_P − VS_V)_frontier: positive when specs matter more for the cheap tier.

**ICC (H2's secondary).** `icc_anova` is ICC(1), Shrout and Fleiss's one-way random-effects model with tasks as
targets and seeds as replicates, on the 0/1 labels (k0 replaces k when tasks have unequal seeds); `arm_icc` takes it
over one arm's runs. `icc_latent` is the latent-scale ICC of the random-intercept model, σ² / (σ² + π²/3).

Records are read as `metrics.py` reads them: the label is VS- (an unknown label counts as 0), `infra_error` and
`leak_suspected` runs are left out, and so are the plan-level slice's rows.

API:
    GlmmFit(names, beta, se, cov, sigma, sigma_se, at_boundary, loglik, converged, n_obs, n_groups, nodes, alpha)
        .coef(name) -> float; .ci(name) -> (low, high)
    loglik(beta, sigma, y, X, groups, nodes=NODES) -> float
    fit(y, X, groups, *, names=None, nodes=NODES, alpha=0.05) -> GlmmFit
    h3_rows(records, *, cheap_arm, cheap_model, frontier_arm) -> (y, X, groups, names)
    h3_interaction(records, **h3_rows options) -> GlmmFit
    icc_anova(labels: Mapping[task, Sequence[int]]) -> float
    icc_latent(sigma) -> float
    arm_icc(records, arm) -> float
"""

from __future__ import annotations

import math
import sys
from collections.abc import Hashable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from statistics import NormalDist

import numpy as np
from scipy import optimize

ANALYSIS_DIR = Path(__file__).resolve().parents[1]
if str(ANALYSIS_DIR) not in sys.path:
    sys.path.insert(0, str(ANALYSIS_DIR))
import metrics  # noqa: E402  (analysis/: the record conventions)

NODES = 25
SIGMA_MAX = 100.0
NEWTON_STEPS = 50
H3_NAMES = ("intercept", "spec", "tier", "spec:tier")
H3_CHEAP = ("roko_fixed", "gpt-oss-120b")
H3_FRONTIER = "fr_claude"


@dataclass(frozen=True)
class GlmmFit:
    names: tuple[str, ...]
    beta: tuple[float, ...]
    se: tuple[float, ...]
    cov: tuple[tuple[float, ...], ...]  # of (β, σ), or β alone at the boundary: the inverse observed information
    sigma: float
    sigma_se: float | None  # None at the boundary
    at_boundary: bool  # σ̂ = 0: no task variance, plain logistic regression
    loglik: float
    converged: bool
    n_obs: int
    n_groups: int
    nodes: int
    alpha: float

    def coef(self, name: str) -> float:
        return self.beta[self.names.index(name)]

    def ci(self, name: str) -> tuple[float, float]:
        """The Wald interval of one coefficient at the fit's alpha."""
        index = self.names.index(name)
        z = NormalDist().inv_cdf(1 - self.alpha / 2)
        return self.beta[index] - z * self.se[index], self.beta[index] + z * self.se[index]


def loglik(beta: Sequence[float], sigma: float, y: Sequence[int], X: Sequence[Sequence[float]],
           groups: Sequence[Hashable], nodes: int = NODES) -> float:
    """The marginal log-likelihood of (β, σ) by adaptive Gauss–Hermite quadrature (module docstring)."""
    data = _Data(y, X, groups)
    return data.loglik(np.asarray(beta, dtype=float), float(sigma), nodes)


def fit(y: Sequence[int], X: Sequence[Sequence[float]], groups: Sequence[Hashable], *,
        names: Sequence[str] | None = None, nodes: int = NODES, alpha: float = 0.05) -> GlmmFit:
    """The maximum-likelihood fit of the random-intercept logistic model (module docstring)."""
    data = _Data(y, X, groups)
    p = data.X.shape[1]
    names = tuple(names) if names is not None else tuple(f"x{index}" for index in range(p))
    if len(names) != p:
        raise ValueError(f"{len(names)} names for {p} columns")

    def negative(theta: np.ndarray) -> float:
        return -data.loglik(theta[:p], float(theta[p]), nodes)

    start = np.append(_logistic_start(data), 1.0)
    bounds = [(None, None)] * p + [(0.0, SIGMA_MAX)]
    result = optimize.minimize(negative, start, method="L-BFGS-B", bounds=bounds,
                               options={"maxiter": 500, "ftol": 1e-13, "gtol": 1e-8})
    theta = result.x
    at_boundary = theta[p] <= 1e-8
    free = p if at_boundary else p + 1  # at the boundary, σ is not a free parameter of the information
    hessian = _hessian(negative, theta, free)
    try:
        cov = np.linalg.inv(hessian)
    except np.linalg.LinAlgError:
        cov = np.full((free, free), np.nan)
    se = np.sqrt(np.clip(np.diag(cov)[:p], 0.0, None))
    sigma = 0.0 if at_boundary else float(theta[p])
    sigma_se = None if at_boundary else math.sqrt(max(cov[p, p], 0.0))
    return GlmmFit(names=names, beta=tuple(map(float, theta[:p])), se=tuple(map(float, se)),
                   cov=tuple(tuple(map(float, row)) for row in cov), sigma=sigma, sigma_se=sigma_se,
                   at_boundary=bool(at_boundary), loglik=float(-result.fun), converged=bool(result.success),
                   n_obs=len(data.y), n_groups=data.n_groups, nodes=nodes, alpha=alpha)


def h3_rows(records: Iterable[dict], *, cheap_arm: str = H3_CHEAP[0], cheap_model: str = H3_CHEAP[1],
            frontier_arm: str = H3_FRONTIER) -> tuple[list[int], list[list[float]], list[str], tuple[str, ...]]:
    """H3's cells as model rows: y (VS-), X (1, spec, tier, spec:tier), the task (instance) of each row, and the
    columns' names (module docstring)."""
    records = list(records)
    models = metrics.run_models(records)
    y, X, groups = [], [], []
    for record in records:
        arm, task = record["arm"], record["task"]
        if (arm not in (cheap_arm, frontier_arm) or task["spec_variant"] not in ("precise", "vague")
                or task["family"] == metrics.PLAN_SLICE or record["execution"]["status"] in metrics.EXCLUDED):
            continue
        if arm == cheap_arm and models[(record["experiment_id"], arm, record["run_id"])] != cheap_model:
            continue
        spec, tier = float(task["spec_variant"] == "precise"), float(arm == cheap_arm)
        y.append(metrics.vs_minus(record))
        X.append([1.0, spec, tier, spec * tier])
        groups.append(task["instance_id"])
    return y, X, groups, H3_NAMES


def h3_interaction(records: Iterable[dict], *, alpha: float = 0.05, nodes: int = NODES, **options: str) -> GlmmFit:
    """The GLMM of H3's cells; its `spec:tier` coefficient and interval are the secondary's estimate."""
    y, X, groups, names = h3_rows(records, **options)
    if not y:
        raise ValueError("no H3 rows: no precise or vague run of the cheap or the frontier arm")
    return fit(y, X, groups, names=names, nodes=nodes, alpha=alpha)


def icc_anova(labels: Mapping[Hashable, Sequence[int]]) -> float:
    """ICC(1): (MSB − MSW) / (MSB + (k0 − 1)·MSW) over tasks (targets) and their runs (replicates)."""
    groups = [list(map(float, runs)) for _, runs in sorted(labels.items(), key=lambda item: repr(item[0]))]
    groups = [runs for runs in groups if runs]
    sizes = [len(runs) for runs in groups]
    total, count = sum(map(sum, groups)), sum(sizes)
    if len(groups) < 2 or count - len(groups) < 1:
        raise ValueError("ICC(1) needs at least two tasks and one task with two or more runs")
    grand = total / count
    between = sum(size * (sum(runs) / size - grand) ** 2 for runs, size in zip(groups, sizes)) / (len(groups) - 1)
    within = sum(sum((value - sum(runs) / len(runs)) ** 2 for value in runs) for runs in groups) / (count - len(groups))
    k0 = (count - sum(size * size for size in sizes) / count) / (len(groups) - 1)
    denominator = between + (k0 - 1) * within
    if denominator == 0:
        raise ValueError("every run has the same label, so the ICC is undefined")
    return (between - within) / denominator


def icc_latent(sigma: float) -> float:
    """σ² / (σ² + π²/3): the share of latent (logistic) variance between tasks."""
    return sigma * sigma / (sigma * sigma + math.pi ** 2 / 3)


def arm_icc(records: Iterable[dict], arm: str) -> float:
    """ICC(1) of one arm's VS- across seeds, tasks as targets (H2's secondary)."""
    labels: dict[tuple, list[int]] = {}
    for record in records:
        if (record["arm"] == arm and record["task"]["family"] != metrics.PLAN_SLICE
                and record["execution"]["status"] not in metrics.EXCLUDED):
            labels.setdefault(metrics.task_key(record), []).append(metrics.vs_minus(record))
    return icc_anova(labels)


class _Data:
    """The design, sorted by group so each group's rows are contiguous (`np.add.reduceat`)."""

    def __init__(self, y: Sequence[int], X: Sequence[Sequence[float]], groups: Sequence[Hashable]) -> None:
        y_array, x_array = np.asarray(y, dtype=float), np.asarray(X, dtype=float)
        if x_array.ndim != 2 or len(y_array) != len(x_array) or len(groups) != len(y_array) or not len(y_array):
            raise ValueError(f"y ({len(y_array)}), X ({x_array.shape}) and groups ({len(groups)}) must match")
        if not np.isin(y_array, (0.0, 1.0)).all():
            raise ValueError("outcomes are 0 or 1")
        index = {group: number for number, group in enumerate(sorted(set(groups), key=repr))}
        codes = np.array([index[group] for group in groups])
        order = np.argsort(codes, kind="stable")
        self.y, self.X, self.codes = y_array[order], x_array[order], codes[order]
        self.n_groups = len(index)
        self.starts = np.flatnonzero(np.r_[True, self.codes[1:] != self.codes[:-1]])

    def loglik(self, beta: np.ndarray, sigma: float, nodes: int) -> float:
        eta = self.X @ beta
        z = self._modes(eta, sigma)
        mu = _expit(eta + sigma * z[self.codes])
        curvature = sigma * sigma * np.add.reduceat(mu * (1 - mu), self.starts) + 1.0
        scale = 1.0 / np.sqrt(curvature)
        t, w = np.polynomial.hermite.hermgauss(nodes)
        points = z[:, None] + math.sqrt(2.0) * scale[:, None] * t[None, :]  # (groups, nodes)
        linear = eta[:, None] + sigma * points[self.codes]  # (rows, nodes)
        terms = self.y[:, None] * linear - np.logaddexp(0.0, linear)
        h = np.add.reduceat(terms, self.starts, axis=0) - points ** 2 / 2  # log integrand at each node
        logs = np.log(w)[None, :] + h + t[None, :] ** 2
        peak = logs.max(axis=1)
        per_group = np.log(scale) - 0.5 * math.log(math.pi) + peak + np.log(np.exp(logs - peak[:, None]).sum(axis=1))
        return float(per_group.sum())

    def _modes(self, eta: np.ndarray, sigma: float) -> np.ndarray:
        """Each group's mode of its log integrand in z, by Newton's method (the integrand is log-concave)."""
        z = np.zeros(self.n_groups)
        for _ in range(NEWTON_STEPS):
            mu = _expit(eta + sigma * z[self.codes])
            gradient = sigma * np.add.reduceat(self.y - mu, self.starts) - z
            curvature = sigma * sigma * np.add.reduceat(mu * (1 - mu), self.starts) + 1.0
            step = gradient / curvature
            z += step
            if np.max(np.abs(step)) < 1e-12:
                break
        return z


def _logistic_start(data: _Data) -> np.ndarray:
    """Plain logistic regression's coefficients (no task effect), a start for the optimizer."""

    def negative(beta: np.ndarray) -> float:
        linear = data.X @ beta
        return float(np.sum(np.logaddexp(0.0, linear) - data.y * linear))

    return optimize.minimize(negative, np.zeros(data.X.shape[1]), method="BFGS").x


def _hessian(function, theta: np.ndarray, free: int, step: float = 1e-4) -> np.ndarray:
    """The central-difference Hessian of `function` in the first `free` coordinates of `theta`."""
    hessian = np.zeros((free, free))
    for i in range(free):
        for j in range(i, free):
            total = 0.0
            for sign_i, sign_j, weight in ((1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1)):
                point = theta.copy()
                point[i] += sign_i * step
                point[j] += sign_j * step
                total += weight * function(point)
            hessian[i, j] = hessian[j, i] = total / (4 * step * step)
    return hessian


def _expit(values: np.ndarray) -> np.ndarray:
    return 0.5 * (1.0 + np.tanh(0.5 * values))
