"""Tests for the secondary models (glmm.py, irt.py): reference fits on fixed synthetic data, each against an independent
computation, plus parameter recovery and the record readers (task 3337).

They need the analysis stack (`requirements-analysis.lock`, decision 3336) and skip without it, as `-rs` shows. Run
from the repository root with the benchmark venv, after installing that lock into it:
    benchmarks/viabilitybench/.venv/bin/python -m pip install --require-hashes \\
        -r benchmarks/viabilitybench/requirements-analysis.lock
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/models/test_models.py -q
"""

from __future__ import annotations

import math

import pytest

np = pytest.importorskip("numpy", reason="the analysis stack (requirements-analysis.lock) is not installed")
pytest.importorskip("scipy", reason="the analysis stack (requirements-analysis.lock) is not installed")
from scipy import integrate, optimize  # noqa: E402

import glmm  # noqa: E402
import irt  # noqa: E402
import metrics  # noqa: E402  (analysis/, on sys.path through glmm)
from test_analysis import run_record  # noqa: E402

# Shrout and Fleiss (1979), Table 2: six targets rated by four judges; their ICC(1,1) is .17.
SHROUT_FLEISS = [[9, 2, 5, 8], [6, 1, 3, 2], [8, 4, 6, 8], [7, 1, 2, 6], [10, 5, 6, 9], [6, 2, 4, 7]]


def glmm_data(tasks: int, sigma: float, beta: tuple[float, ...], reps: int, seed: int) -> tuple[list, list, list]:
    """H3-shaped data: every task under spec x tier, `reps` runs per cell, a normal task intercept of SD sigma."""
    rng = np.random.default_rng(seed)
    y, X, groups = [], [], []
    for task in range(tasks):
        u = rng.normal(0.0, sigma)
        for spec in (0, 1):
            for tier in (0, 1):
                row = [1.0, float(spec), float(tier), float(spec * tier)]
                p = 1.0 / (1.0 + math.exp(-(sum(b * x for b, x in zip(beta, row)) + u)))
                for _ in range(reps):
                    y.append(int(rng.random() < p))
                    X.append(row)
                    groups.append(f"T{task:03d}")
    return y, X, groups


def quad_loglik(beta: np.ndarray, sigma: float, y: list, X: list, groups: list) -> float:
    """The marginal log-likelihood by scipy's adaptive quadrature over each task's intercept, no Gauss-Hermite."""
    eta = np.asarray(X) @ beta
    outcome = np.asarray(y, dtype=float)
    total = 0.0
    for group in sorted(set(groups)):
        rows = np.array([index for index, name in enumerate(groups) if name == group])

        def integrand(z: float, rows=rows) -> float:
            linear = eta[rows] + sigma * z
            return math.exp(float(np.sum(outcome[rows] * linear - np.logaddexp(0.0, linear))) - z * z / 2) / math.sqrt(
                2 * math.pi)

        total += math.log(integrate.quad(integrand, -12, 12, epsabs=0, epsrel=1e-12, limit=200)[0])
    return total


def grid_loglik(values: np.ndarray, a: np.ndarray, c: np.ndarray) -> float:
    """The 2PL marginal log-likelihood on an 801-point trapezoid grid over θ in [-8, 8], no Gauss-Hermite."""
    theta = np.linspace(-8, 8, 801)
    density = np.exp(-theta ** 2 / 2) / math.sqrt(2 * math.pi)
    linear = a[:, None] * theta[None, :] + c[:, None]
    log_pass, log_fail = -np.logaddexp(0.0, -linear), -np.logaddexp(0.0, linear)
    seen = ~np.isnan(values)
    passed = np.where(seen, values, 0.0)
    joint = np.exp(passed @ log_pass + (seen - passed) @ log_fail) * density[None, :]
    return float(np.sum(np.log(np.trapezoid(joint, theta, axis=1))))


def test_glmm_and_irt_reproduce_reference_fits():
    """Each model's fit equals a reference fit by an independent route: the GLMM's adaptive Gauss-Hermite likelihood
    against scipy's adaptive quadrature, maximized by another optimizer; the IRT's EM against direct maximization of
    a trapezoid-grid marginal likelihood. Tolerances: likelihoods 1e-6, parameters 2e-3."""
    y, X, groups = glmm_data(tasks=12, sigma=1.2, beta=(-0.3, 1.1, -0.7, 0.8), reps=2, seed=3337)
    fitted = glmm.fit(y, X, groups, names=glmm.H3_NAMES)
    assert fitted.converged and fitted.n_obs == 96 and fitted.n_groups == 12
    for beta, sigma in ((np.array(fitted.beta), fitted.sigma), (np.array([0.1, -0.2, 0.3, 0.4]), 2.0)):
        assert glmm.loglik(beta, sigma, y, X, groups) == pytest.approx(quad_loglik(beta, sigma, y, X, groups),
                                                                       abs=1e-6)
    reference = optimize.minimize(lambda theta: -quad_loglik(theta[:4], math.exp(theta[4]), y, X, groups),
                                  np.zeros(5), method="Nelder-Mead",
                                  options={"xatol": 1e-7, "fatol": 1e-10, "maxiter": 20000, "maxfev": 20000})
    assert np.allclose(fitted.beta, reference.x[:4], atol=2e-3) and fitted.sigma == pytest.approx(
        math.exp(reference.x[4]), abs=2e-3)
    assert fitted.loglik == pytest.approx(-reference.fun, abs=1e-6)

    rng = np.random.default_rng(42)
    a_true, b_true = np.array([0.8, 1.2, 1.6, 1.0, 2.0]), np.array([-1.0, -0.3, 0.2, 0.8, 1.3])
    theta = rng.normal(0.0, 1.0, 300)
    values = (rng.random((300, 5)) < 1 / (1 + np.exp(-a_true * (theta[:, None] - b_true)))).astype(float)
    values[rng.random(values.shape) < 0.1] = np.nan  # respondents who did not run a task
    matrix = irt.Matrix(respondents=tuple(range(300)), items=tuple(range(5)), values=values,
                        levels={item: item + 1 for item in range(5)})
    found = irt.fit(matrix)
    a, b = np.array(found.a), np.array(found.b)
    assert found.converged and found.extreme == {}
    assert irt.marginal_loglik(values, a, -a * b) == pytest.approx(grid_loglik(values, a, -a * b), abs=1e-6)
    reference = optimize.minimize(lambda params: -grid_loglik(values, params[:5], params[5:]),
                                  np.r_[np.ones(5), np.zeros(5)], method="BFGS", options={"gtol": 1e-8})
    assert np.allclose(a, reference.x[:5], atol=2e-3) and np.allclose(-a * b, reference.x[5:], atol=2e-3)
    assert found.loglik == pytest.approx(-reference.fun, abs=1e-6)


def test_glmm_recovers_a_planted_interaction_and_its_task_variance():
    beta = (-0.2, 1.0, -0.8, 0.9)
    y, X, groups = glmm_data(tasks=250, sigma=1.0, beta=beta, reps=2, seed=7)
    fitted = glmm.fit(y, X, groups, names=glmm.H3_NAMES)
    for name, true in zip(glmm.H3_NAMES, beta):
        low, high = fitted.ci(name)
        assert low <= true <= high, (name, fitted.coef(name), (low, high))
    assert abs(fitted.sigma - 1.0) <= 2.5 * fitted.sigma_se and not fitted.at_boundary
    assert glmm.icc_latent(fitted.sigma) == pytest.approx(fitted.sigma ** 2 / (fitted.sigma ** 2 + math.pi ** 2 / 3))


def test_glmm_without_task_variance_is_logistic_regression():
    """Every task has the same outcomes, so the likelihood peaks at σ = 0: the fit sits at the bound and its
    coefficients are plain logistic regression's (fit here by direct maximization)."""
    y, X, groups = [], [], []
    for task in range(30):
        for row, outcome in (([1.0, 0, 0, 0], 0), ([1.0, 1, 0, 0], 1), ([1.0, 0, 1, 0], 1), ([1.0, 1, 1, 1], 1),
                             ([1.0, 0, 0, 0], 1), ([1.0, 1, 1, 1], 0), ([1.0, 1, 0, 0], 0), ([1.0, 0, 1, 0], 0)):
            y.append(outcome)
            X.append(row)
            groups.append(task)
    fitted = glmm.fit(y, X, groups, names=glmm.H3_NAMES)
    assert fitted.at_boundary and fitted.sigma_se is None
    design, outcome = np.asarray(X), np.asarray(y, dtype=float)
    plain = optimize.minimize(lambda b: float(np.sum(np.logaddexp(0.0, design @ b) - outcome * (design @ b))),
                              np.zeros(4), method="BFGS", options={"gtol": 1e-10})
    assert np.allclose(fitted.beta, plain.x, atol=1e-3)


def test_h3_rows_take_precise_and_vague_cells_of_both_tiers():
    records = []
    for index, (arm, variant, label) in enumerate((("roko_fixed", "precise", 1), ("roko_fixed", "vague", 0),
                                                   ("fr_claude", "precise", 1), ("fr_claude", "vague", 1))):
        record = run_record("F1-l2-0001", 1, arm=arm, run_id=f"h3-{index}", label=label,
                            model="claude-opus-5-5" if arm == "fr_claude" else None)
        record["task"]["spec_variant"] = variant
        records.append(record)
    refined = run_record("F1-l2-0001", 2, arm="roko_fixed", run_id="h3-r")
    refined["task"]["spec_variant"] = "refined"  # level R is recovery, not a cell of the interaction
    other_model = run_record("F1-l2-0002", 1, arm="roko_fixed", run_id="h3-glm", model="glm-4.7")
    failed = run_record("F1-l2-0003", 1, arm="fr_claude", run_id="h3-x", status="infra_error",
                        model="claude-opus-5-5")
    y, X, groups, names = glmm.h3_rows([*records, refined, other_model, failed])
    assert names == ("intercept", "spec", "tier", "spec:tier") and groups == ["F1-l2-0001"] * 4
    assert y == [1, 0, 1, 1] and X == [[1.0, 1.0, 1.0, 1.0], [1.0, 0.0, 1.0, 0.0], [1.0, 1.0, 0.0, 0.0],
                                       [1.0, 0.0, 0.0, 0.0]]
    with pytest.raises(ValueError):
        glmm.h3_interaction([refined])


def test_icc_matches_shrout_and_fleiss_and_reads_an_arms_seeds():
    icc = glmm.icc_anova({target: ratings for target, ratings in enumerate(SHROUT_FLEISS)})
    assert round(icc, 2) == 0.17  # their published ICC(1,1)
    # Exactly: BMS = 11.24 (to 2 places) and WMS = 6.26, so (BMS - WMS) / (BMS + 3 WMS).
    grand = sum(map(sum, SHROUT_FLEISS)) / 24
    bms = 4 * sum((sum(row) / 4 - grand) ** 2 for row in SHROUT_FLEISS) / 5
    wms = sum(sum((value - sum(row) / 4) ** 2 for value in row) for row in SHROUT_FLEISS) / 18
    assert (round(bms, 2), round(wms, 2)) == (11.24, 6.26) and icc == pytest.approx((bms - wms) / (bms + 3 * wms))
    records = [run_record(f"F1-l1-{task:04d}", seed, arm="roko_full", label=label)
               for task, labels in enumerate(((1, 1, 1), (0, 0, 0), (1, 1, 0), (0, 0, 1)), 1)
               for seed, label in enumerate(labels, 1)]
    records.append(run_record("F1-l1-0001", 4, arm="roko_full", label=0, status="infra_error"))  # left out
    by_task = {task: labels for task, labels in enumerate(((1, 1, 1), (0, 0, 0), (1, 1, 0), (0, 0, 1)))}
    assert glmm.arm_icc(records, "roko_full") == pytest.approx(glmm.icc_anova(by_task))
    with pytest.raises(ValueError):
        glmm.icc_anova({"a": [1, 1], "b": [1, 1]})  # no variance at all


def test_irt_recovers_planted_items_and_reads_the_campaigns_outcome_matrix():
    rng = np.random.default_rng(5)
    a_true, b_true = np.linspace(0.6, 2.0, 12), np.linspace(-1.5, 1.5, 12)
    theta = rng.normal(0.0, 1.0, 2000)
    values = (rng.random((2000, 12)) < 1 / (1 + np.exp(-a_true * (theta[:, None] - b_true)))).astype(float)
    found = irt.fit(irt.Matrix(respondents=tuple(range(2000)), items=tuple(range(12)), values=values,
                               levels={item: 1 + item // 3 for item in range(12)}))
    assert np.mean(np.abs(np.array(found.b) - b_true)) < 0.12 and np.corrcoef(found.b, b_true)[0, 1] > 0.98
    assert np.mean(np.abs(np.array(found.a) - a_true)) < 0.15
    levels = irt.level_difficulty(found, {item: 1 + item // 3 for item in range(12)})
    assert [entry["mean_b"] for entry in levels.values()] == sorted(entry["mean_b"] for entry in levels.values())

    records = [run_record(instance, seed, arm=arm, run_id=f"log1-{arm}", label=label)
               for arm, labels in (("roko_fixed", (1, 0, 1, 1)), ("cheap_direct", (1, 0, 0, 1)))
               for seed in (1, 2)
               for instance, label in zip(("F1-l1-0001", "F1-l2-0002", "F1-l3-0003", "F2-l1-0004"), labels)]
    records[-1] = run_record("F2-l1-0004", 2, arm="cheap_direct", run_id="log1-cheap_direct", label=0)
    matrix = irt.outcome_matrix(records)
    variant = records[0]["task"]["spec_variant"]
    assert matrix.respondents == (("cheap_direct", "", 1), ("cheap_direct", "", 2), ("roko_fixed", "", 1),
                                  ("roko_fixed", "", 2))
    assert matrix.items == tuple((instance, variant) for instance in ("F1-l1-0001", "F1-l2-0002", "F1-l3-0003",
                                                                       "F2-l1-0004"))
    assert matrix.levels == {item: level for item, level in zip(matrix.items, (1, 2, 3, 1))}
    fitted = irt.fit(matrix)  # F1-l1-0001 all passed, F1-l2-0002 all failed: neither has a finite b
    assert fitted.extreme == {matrix.items[0]: "all passed", matrix.items[1]: "all failed"}
    assert set(fitted.items) == {matrix.items[2], matrix.items[3]}
    summary = irt.level_difficulty(fitted, matrix.levels)
    assert summary[2] == {"mean_b": None, "n": 0, "too_easy": 0, "too_hard": 1}
    assert summary[1]["too_easy"] == 1 and summary[1]["n"] == 1
    with pytest.raises(metrics.MetricsError):
        irt.outcome_matrix([*records, records[0]])
