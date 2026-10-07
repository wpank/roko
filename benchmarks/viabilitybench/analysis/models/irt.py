"""The 2PL item response model over a campaign's outcome matrix, and each level's difficulty on its scale (S09 §4.1:
"2PL IRT, the envelope on an empirical difficulty scale"; task 3337 under decision 3336).

    P(VS = 1 | respondent p, task j) = 1 / (1 + exp(−a_j·(θ_p − b_j))),   θ_p ~ N(0, 1)

**The matrix** (`outcome_matrix`). Respondents are the campaign's (arm, model, seed) triples and items its tasks
(instance, spec variant); an entry is VS- (an unknown label counts as 0), and missing where the respondent did not run
the task. Records are read as `metrics.py` reads them: `infra_error` and `leak_suspected` runs and the plan-level
slice are left out, and a repeated (respondent, task) is an error.

**The fit** (`fit`) is marginal maximum likelihood by Bock and Aitkin's EM: θ is integrated on `nodes` Gauss–Hermite
points, and each M-step is a Newton solve of every item's weighted logistic regression in the slope-intercept form
a·θ + c (so b = −c / a), with a kept in [A_MIN, A_MAX]. The N(0, 1) prior fixes the scale, so b is in SDs of the
respondents' ability. An item every respondent passed, or every one failed, has no finite b: it is left out of the
fit and listed as not estimable (`extreme`), with the side it fell on.

**Level difficulty** (`level_difficulty`): the mean b over each level's estimable tasks, the empirical scale on which
the envelope's levels can be read, with the counts of tasks left out at either extreme.

numpy and scipy, pinned in `requirements-analysis.lock`, are imported only under `analysis/models/` (decision 3336).

API:
    Matrix(respondents, items, values, levels)              # values: respondents x items, NaN where missing
    outcome_matrix(records) -> Matrix
    IrtFit(items, a, b, extreme, loglik, iterations, converged, nodes)
    fit(matrix, *, nodes=NODES, tol=1e-8, max_iter=MAX_ITER) -> IrtFit
    marginal_loglik(values, a, c, nodes=NODES) -> float     # the EM's objective, for checks
    level_difficulty(fit, levels: Mapping[item, level]) -> dict[level, {"mean_b", "n", "too_easy", "too_hard"}]
"""

from __future__ import annotations

import math
import sys
from collections.abc import Hashable, Iterable, Mapping
from dataclasses import dataclass
from pathlib import Path

import numpy as np

ANALYSIS_DIR = Path(__file__).resolve().parents[1]
if str(ANALYSIS_DIR) not in sys.path:
    sys.path.insert(0, str(ANALYSIS_DIR))
import metrics  # noqa: E402  (analysis/: the record conventions)

NODES = 41
MAX_ITER = 2000
A_MIN, A_MAX = 0.05, 20.0
NEWTON_STEPS = 25


@dataclass(frozen=True)
class Matrix:
    respondents: tuple[tuple, ...]  # (arm, model, seed)
    items: tuple[tuple, ...]  # (instance id, spec variant)
    values: np.ndarray  # respondents x items: 0.0, 1.0, or NaN where the respondent did not run the item
    levels: dict  # item -> its task's level (task.ladder)


@dataclass(frozen=True)
class IrtFit:
    items: tuple[Hashable, ...]  # the estimable items, in matrix order
    a: tuple[float, ...]
    b: tuple[float, ...]
    extreme: dict  # item -> "all passed" or "all failed": no finite b
    loglik: float
    iterations: int
    converged: bool
    nodes: int


def outcome_matrix(records: Iterable[dict]) -> Matrix:
    """The respondents x tasks matrix of VS- (module docstring)."""
    records = metrics.with_models(records)
    cells: dict[tuple, dict[tuple, int]] = {}
    levels: dict[tuple, int] = {}
    for record in records:
        if record["task"]["family"] == metrics.PLAN_SLICE or record["execution"]["status"] in metrics.EXCLUDED:
            continue
        respondent = (record["arm"], record["model"] or "", record["seed"])
        item = metrics.task_key(record)
        row = cells.setdefault(respondent, {})
        if item in row:
            raise metrics.MetricsError(f"{metrics.cell_name(respondent[0], respondent[1] or None)} ran {item[0]} "
                                       f"({item[1]}) twice on seed {respondent[2]}")
        row[item] = metrics.vs_minus(record)
        levels[item] = record["task"]["ladder"]
    respondents = tuple(sorted(cells))
    items = tuple(sorted({item for row in cells.values() for item in row}))
    values = np.full((len(respondents), len(items)), np.nan)
    column = {item: index for index, item in enumerate(items)}
    for i, respondent in enumerate(respondents):
        for item, label in cells[respondent].items():
            values[i, column[item]] = label
    return Matrix(respondents=respondents, items=items, values=values, levels=dict(sorted(levels.items())))


def fit(matrix: Matrix, *, nodes: int = NODES, tol: float = 1e-8, max_iter: int = MAX_ITER) -> IrtFit:
    """The 2PL's marginal maximum-likelihood item parameters (module docstring)."""
    values = np.asarray(matrix.values, dtype=float)
    observed = ~np.isnan(values)
    passed = np.where(observed, values, 0.0)
    counts, successes = observed.sum(axis=0), passed.sum(axis=0)
    extreme = {}
    keep = []
    for index, item in enumerate(matrix.items):
        if counts[index] and successes[index] == counts[index]:
            extreme[item] = "all passed"
        elif counts[index] and successes[index] == 0:
            extreme[item] = "all failed"
        elif counts[index]:
            keep.append(index)
    if not keep:
        raise ValueError("no item has both a pass and a fail, so no item is estimable")
    y, seen = passed[:, keep], observed[:, keep].astype(float)
    theta, prior = _quadrature(nodes)
    rate = np.clip(successes[keep] / counts[keep], 0.01, 0.99)
    a, c = np.ones(len(keep)), np.log(rate / (1 - rate))
    previous, converged, iteration = -math.inf, False, 0
    for iteration in range(1, max_iter + 1):
        posterior, current = _e_step(y, seen, a, c, theta, prior)
        expected_n, expected_r = seen.T @ posterior, y.T @ posterior  # items x nodes
        a, c = _m_step(expected_n, expected_r, theta, a, c)
        if abs(current - previous) < tol:
            converged = True
            break
        previous = current
    loglik = _e_step(y, seen, a, c, theta, prior)[1]
    return IrtFit(items=tuple(matrix.items[index] for index in keep), a=tuple(map(float, a)),
                  b=tuple(map(float, -c / a)), extreme=extreme, loglik=float(loglik), iterations=iteration,
                  converged=converged, nodes=nodes)


def marginal_loglik(values: np.ndarray, a: np.ndarray, c: np.ndarray, nodes: int = NODES) -> float:
    """Σ_p log ∫ Π_j P(y_pj | θ) φ(θ) dθ on the quadrature the fit uses, NaN entries skipped."""
    values = np.asarray(values, dtype=float)
    observed = ~np.isnan(values)
    theta, prior = _quadrature(nodes)
    return float(_e_step(np.where(observed, values, 0.0), observed.astype(float), np.asarray(a, dtype=float),
                         np.asarray(c, dtype=float), theta, prior)[1])


def level_difficulty(result: IrtFit, levels: Mapping[Hashable, int]) -> dict[int, dict]:
    """Per level: the mean b of its estimable items, how many there are, and how many fell at either extreme."""
    found: dict[int, dict] = {}
    for item, b in zip(result.items, result.b):
        found.setdefault(levels[item], {"b": [], "too_easy": 0, "too_hard": 0})["b"].append(b)
    for item, side in result.extreme.items():
        entry = found.setdefault(levels[item], {"b": [], "too_easy": 0, "too_hard": 0})
        entry["too_easy" if side == "all passed" else "too_hard"] += 1
    return {level: {"mean_b": math.fsum(entry["b"]) / len(entry["b"]) if entry["b"] else None, "n": len(entry["b"]),
                    "too_easy": entry["too_easy"], "too_hard": entry["too_hard"]}
            for level, entry in sorted(found.items())}


def _quadrature(nodes: int) -> tuple[np.ndarray, np.ndarray]:
    """θ points and N(0, 1) weights: Gauss–Hermite for ∫ e^(−t²), with θ = √2·t and weights w / √π."""
    t, w = np.polynomial.hermite.hermgauss(nodes)
    return math.sqrt(2.0) * t, w / math.sqrt(math.pi)


def _e_step(y: np.ndarray, seen: np.ndarray, a: np.ndarray, c: np.ndarray, theta: np.ndarray,
            prior: np.ndarray) -> tuple[np.ndarray, float]:
    """Each respondent's posterior over the nodes, and the marginal log-likelihood."""
    linear = a[:, None] * theta[None, :] + c[:, None]  # items x nodes
    log_pass, log_fail = -np.logaddexp(0.0, -linear), -np.logaddexp(0.0, linear)
    joint = y @ log_pass + (seen - y) @ log_fail + np.log(prior)[None, :]  # respondents x nodes
    peak = joint.max(axis=1, keepdims=True)
    mass = np.exp(joint - peak)
    total = mass.sum(axis=1, keepdims=True)
    return mass / total, float(np.sum(peak[:, 0] + np.log(total[:, 0])))


def _m_step(expected_n: np.ndarray, expected_r: np.ndarray, theta: np.ndarray, a: np.ndarray,
            c: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Every item's weighted logistic regression of the expected passes on θ, by Newton's method in (a, c)."""
    a, c = a.copy(), c.copy()
    for _ in range(NEWTON_STEPS):
        p = 1.0 / (1.0 + np.exp(-(a[:, None] * theta[None, :] + c[:, None])))
        residual, weight = expected_r - expected_n * p, expected_n * p * (1 - p)
        grad_a, grad_c = (residual * theta).sum(axis=1), residual.sum(axis=1)
        h_aa, h_ac, h_cc = (weight * theta ** 2).sum(axis=1), (weight * theta).sum(axis=1), weight.sum(axis=1)
        det = h_aa * h_cc - h_ac ** 2
        safe = det > 1e-12
        step_a = np.where(safe, (h_cc * grad_a - h_ac * grad_c) / np.where(safe, det, 1.0), 0.0)
        step_c = np.where(safe, (h_aa * grad_c - h_ac * grad_a) / np.where(safe, det, 1.0), 0.0)
        a, c = np.clip(a + step_a, A_MIN, A_MAX), c + step_c
        if max(np.max(np.abs(step_a)), np.max(np.abs(step_c))) < 1e-10:
            break
    return a, c
