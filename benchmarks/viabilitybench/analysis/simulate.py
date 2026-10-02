#!/usr/bin/env python3
"""Synthetic campaigns: does the analysis plan do what it claims? (S09 E3's acceptance, SC 4; task 3339.)

    simulate.py [--planted 0.15] [--sims 1000] [--b 10000] [--workers N] [--seed 3339] [--out report.json]

Each simulated campaign draws P1-core's shape afresh: six families × five levels × 4 tasks (120), 3 seeds, and two
arms, A (as `roko_full`) and B (as `fd_claude`), run on the same tasks. A task's chance of success under B is drawn
from its level's band (`LEVEL_BANDS`, harder levels lower). Under A it is B's plus the planted difference plus the
task's own deviation, uniform on [−`spread`, `spread`] (tasks differ in how much the harness helps), so the population
has E[p_A − p_B] = `planted` exactly. Given its task, each run succeeds independently of the other arm's, unless
`coupling` > 0 makes a seed's two runs share one uniform draw with that probability. Each run costs a lognormal amount
around its arm's mean (`COSTS`, CV 0.3).

The criteria, each run through the analysis code itself (`bootstrap.py`, `envelope.py`, `cuped.py`, `cs.py`):
- `bootstrap_rate_coverage`: the family × ℓ stratified percentile interval for A's VS rate covers its population
  mean; band 0.93-0.97 (S09 E3).
- `bootstrap_diff_coverage`: the paired, family × ℓ stratified percentile interval for VS rate(A) − VS rate(B)
  covers `planted`; band 0.93-0.97 (S09 E3).
- `bootstrap_ratio_coverage`: the BCa interval for R = VS rate(A) / VS rate(B) covers the population ratio; band
  0.93-0.97.
- `cuped_coverage`: CUPED's interval for a between-task contrast (each task assigned to one arm by a coin, its
  covariate the mean of three pre-assignment runs under B, drawn apart from the campaign's runs, as LOG1's would
  be) covers `planted`; band 0.93-0.97.
- `cs_anytime_coverage`: the confidence sequence over the per-task paired differences, in stream order, covers
  `planted` at every time; at least 0.95.
- `envelope_fwer_r` and `envelope_fwer_c`: under the global null at the bars, the share of campaigns with E* ≥ 1
  at α = 0.05, the envelope's own level when H1 holds the whole α (its least favourable case); at most 0.05. The
  R-null puts R = X = 0.90 at every level with C far below Y; the C-null puts C = Y = 0.30 at level 1 with A
  `planted` ahead of B, so R clears X and C is the binding bar. Under a global null a false claim needs level 1
  claimed first, so these runs test level 1 alone (`envelope(levels=(1,))`). The null campaigns have no task-level
  spread, so the null holds on every task.

Every campaign has its own seed (the run's `--seed` plus its index), so a report is the same whatever `--workers`.
The JSON report (`vb.simulation/1`) records the design, each criterion's value, band and Monte Carlo standard
error, and the resample counts: B per interval and the total number of bootstrap replicates drawn.

1,000 campaigns at B = 10,000 is heavy in pure Python: run the full simulation once, off-CI (`--workers` spreads the
campaigns over processes); `test_simulate.py` runs a fast version with a wider band.

**The full run** (2026-10-03, 1,000 campaigns, B = 10,000, 70 million replicates, 39 minutes on 6 workers) is kept for
the lock in `reports/simulation/simulate-planted-0.15.json`. Six criteria are in band; the stratified bootstrap of one
arm's VS rate covers 0.975 ± 0.005, above the band, and the difference (0.968) and the ratio (0.970) sit at its top.
S09 §4.1's two-stage resampling (tasks within strata, then seeds within tasks) counts the seeds' noise twice, so it is
conservative. In 300-campaign checks, resampling tasks alone covers 0.90 (four tasks per stratum: the stratified
bootstrap's variance is (n_h − 1)/n_h of the true one), and resampling n_h − 1 tasks per stratum, tasks alone, covers
0.95 for the rate, the difference and the ratio. Which one S09 uses is the author's decision; `bootstrap.py` keeps
§4.1's method until then.

API:
    LEVEL_BANDS, FAMILIES, TASKS_PER_STRATUM, SEEDS, COSTS, CRITERIA
    campaign(rng, *, planted=0.0, scale=1.0, coupling=0.0, spread=EFFECT_SPREAD, cost_ratio=None, tasks=...,
             seeds=..., cost_cv=0.3) -> list[dict]
    population_ratio(planted, scale=1.0) -> float; population_rate(planted) -> float
    simulate_one(index, *, planted, b, seed, ...) -> dict          # one campaign's outcomes
    simulate(*, planted=0.15, sims=1000, b=10_000, seed=3339, workers=1, ...) -> dict    # the report
    main(argv=None) -> int
"""

from __future__ import annotations

import argparse
import json
import math
import sys
import time
from concurrent.futures import ProcessPoolExecutor
from functools import partial
from pathlib import Path
from random import Random

import bootstrap
import cs
import cuped
import envelope

SCHEMA = "vb.simulation/1"
FAMILIES = ("F1", "F2", "F3", "F4", "F5", "F7")  # P1-core's six families (S08 §4.7)
LEVEL_BANDS = {1: (0.55, 0.75), 2: (0.45, 0.65), 3: (0.30, 0.50), 4: (0.15, 0.35), 5: (0.10, 0.30)}  # p_B ~ U
EFFECT_SPREAD = 0.10  # how much tasks differ in A's advantage: planted + U(-spread, spread), mean planted
TASKS_PER_STRATUM = 4  # 6 families x 5 levels x 4 = 120 tasks, P1-core's size
SEEDS = 3
COSTS = {"A": 0.02, "B": 0.40}  # mean $ per run: a cheap arm and a frontier one
CRITERIA = {"bootstrap_rate_coverage": (0.93, 0.97), "bootstrap_diff_coverage": (0.93, 0.97),
            "bootstrap_ratio_coverage": (0.93, 0.97), "cuped_coverage": (0.93, 0.97),
            "cs_anytime_coverage": (0.95, 1.0), "envelope_fwer_r": (0.0, 0.05), "envelope_fwer_c": (0.0, 0.05)}
OUTCOMES = {"bootstrap_rate_coverage": "rate_covers", "bootstrap_diff_coverage": "diff_covers",
            "bootstrap_ratio_coverage": "ratio_covers", "cuped_coverage": "cuped_covers",
            "cs_anytime_coverage": "cs_covers", "envelope_fwer_r": "fwer_r", "envelope_fwer_c": "fwer_c"}


def campaign(rng: Random, *, planted: float = 0.0, scale: float = 1.0, coupling: float = 0.0,
             spread: float = EFFECT_SPREAD, cost_ratio: float | None = None, tasks: int = TASKS_PER_STRATUM,
             seeds: int = SEEDS, cost_cv: float = 0.3) -> list[dict]:
    """One campaign's rows for arms A and B (module docstring): p_A = scale·p_B + planted + U(-spread, spread) per
    task, and A's mean cost `cost_ratio` × B's when given (else COSTS)."""
    lowest = scale * min(low for low, _ in LEVEL_BANDS.values()) + planted - spread
    highest = scale * max(high for _, high in LEVEL_BANDS.values()) + planted + spread
    if not (0 <= lowest and highest <= 1):
        raise ValueError(f"planted {planted} with spread {spread} takes some task's p_A outside [0, 1]")
    mean_cost = dict(COSTS) if cost_ratio is None else {"A": cost_ratio * COSTS["B"], "B": COSTS["B"]}
    sigma = math.sqrt(math.log(1 + cost_cv ** 2))
    rows = []
    for family in FAMILIES:
        for level, (low, high) in LEVEL_BANDS.items():
            for task in range(1, tasks + 1):
                p_b = rng.uniform(low, high)
                p = {"A": scale * p_b + planted + (rng.uniform(-spread, spread) if spread else 0.0), "B": p_b}
                instance = f"{family}-l{level}-{task:04d}"
                for seed in range(1, seeds + 1):
                    shared = rng.random()
                    draws = {"A": shared, "B": shared} if rng.random() < coupling else {"A": rng.random(),
                                                                                        "B": rng.random()}
                    for arm in ("A", "B"):
                        usd = mean_cost[arm] * math.exp(rng.gauss(-sigma * sigma / 2, sigma))
                        rows.append(_row(arm, family, level, instance, seed, int(draws[arm] < p[arm]), usd) | {
                            "_p_b": p_b})
    return rows


def population_rate(planted: float) -> float:
    """A's VS rate in the population: B's mean over the level bands, every level equally weighted, plus `planted`."""
    return sum((low + high) / 2 for low, high in LEVEL_BANDS.values()) / len(LEVEL_BANDS) + planted


def population_ratio(planted: float, scale: float = 1.0) -> float:
    """R in the population: mean p_A / mean p_B over the level bands, every level equally weighted."""
    mean_b = population_rate(0.0)
    return (scale * mean_b + planted) / mean_b


def simulate_one(index: int, *, planted: float, b: int, seed: int, coupling: float = 0.0,
                 spread: float = EFFECT_SPREAD) -> dict:
    """One planted campaign's interval checks and the two null campaigns' level-1 claims (module docstring)."""
    rng = Random(seed + index)
    rows = campaign(rng, planted=planted, coupling=coupling, spread=spread)
    rate = bootstrap.paired_bootstrap(rows, _rate_a, b=b, seed=seed + index)
    diff = bootstrap.paired_bootstrap(rows, _difference, b=b, seed=seed + index)
    ratio = bootstrap.paired_bootstrap(rows, _ratio, b=b, seed=seed + index, method="bca")
    truth = population_ratio(planted)
    contrast = _cuped(rows, rng)
    stream = _paired_stream(rows, rng)
    fwer = {}
    level_one = sum(LEVEL_BANDS[1]) / 2  # B's mean at level 1, where a null campaign's claim is decided
    c_null = envelope.Y * (level_one + planted) / level_one  # A's cost ratio that puts C_1 at Y exactly
    for name, null in (("r", {"scale": envelope.X, "cost_ratio": 0.1}),
                       ("c", {"planted": planted, "cost_ratio": c_null})):
        null_rows = campaign(rng, coupling=coupling, spread=0.0, **null)
        found = envelope.envelope(null_rows, arm="A", reference="B", b=b, seed=seed + index, levels=(1,))
        fwer[name] = found.e_star >= 1
    return {"index": index, "rate_covers": rate.low <= population_rate(planted) <= rate.high,
            "diff_covers": diff.low <= planted <= diff.high,
            "ratio_covers": ratio.low <= truth <= ratio.high,
            "cuped_covers": contrast.ci[0] <= planted <= contrast.ci[1],
            "cs_covers": cs.covers(stream, planted, lo=-1.0, hi=1.0), "fwer_r": fwer["r"], "fwer_c": fwer["c"],
            "replicates": 3 * b + 4 * b}  # three planted intervals, then R and C at level 1 in each null campaign


def simulate(*, planted: float = 0.15, sims: int = 1000, b: int = bootstrap.DEFAULT_B, seed: int = 3339,
             workers: int = 1, coupling: float = 0.0, spread: float = EFFECT_SPREAD) -> dict:
    """Run `sims` campaigns and report every criterion (module docstring)."""
    if sims < 1 or b < 1 or workers < 1:
        raise ValueError("sims, b and workers must each be at least 1")
    campaign(Random(0), planted=planted, spread=spread)  # refuses a design that leaves [0, 1] before any work
    started = time.monotonic()
    one = partial(simulate_one, planted=planted, b=b, seed=seed, coupling=coupling, spread=spread)
    if workers == 1:
        outcomes = [one(index) for index in range(sims)]
    else:
        with ProcessPoolExecutor(max_workers=workers) as pool:
            outcomes = list(pool.map(one, range(sims), chunksize=max(1, sims // (4 * workers))))
    criteria = {}
    for name, field in OUTCOMES.items():
        share = sum(outcome[field] for outcome in outcomes) / sims
        low, high = CRITERIA[name]
        criteria[name] = {"value": share, "band": [low, high], "in_band": low <= share <= high,
                          "mc_se": math.sqrt(share * (1 - share) / sims)}
    design = {"families": list(FAMILIES), "levels": {str(level): list(band) for level, band in LEVEL_BANDS.items()},
              "tasks_per_stratum": TASKS_PER_STRATUM, "seeds": SEEDS, "costs": COSTS, "coupling": coupling,
              "spread": spread, "population_rate": population_rate(planted),
              "population_ratio": population_ratio(planted), "alpha": 0.05,
              "envelope_bars": {"X": envelope.X, "Y": envelope.Y}}
    return {"schema_version": SCHEMA, "planted": planted, "sims": sims, "seed": seed, "design": design,
            "resamples": {"b_per_interval": b, "intervals_per_campaign": 7,
                          "total_replicates": sum(outcome["replicates"] for outcome in outcomes)},
            "criteria": criteria, "all_in_band": all(entry["in_band"] for entry in criteria.values()),
            "seconds": round(time.monotonic() - started, 1)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--planted", type=float, default=0.15, help="the planted difference in VS rate (default 0.15)")
    parser.add_argument("--sims", type=int, default=1000, help="campaigns to simulate (default 1000)")
    parser.add_argument("--b", type=int, default=bootstrap.DEFAULT_B, help="bootstrap replicates per interval")
    parser.add_argument("--seed", type=int, default=3339, help="the first campaign's seed")
    parser.add_argument("--workers", type=int, default=1, help="processes to spread the campaigns over")
    parser.add_argument("--coupling", type=float, default=0.0, help="share of seeds whose two runs share a draw")
    parser.add_argument("--spread", type=float, default=EFFECT_SPREAD, help="half-width of a task's own effect")
    parser.add_argument("--out", type=Path, help="write the JSON report here as well")
    args = parser.parse_args(argv)
    try:
        report = simulate(planted=args.planted, sims=args.sims, b=args.b, seed=args.seed, workers=args.workers,
                          coupling=args.coupling, spread=args.spread)
    except ValueError as err:
        parser.error(str(err))
    text = json.dumps(report, indent=2)
    if args.out is not None:
        args.out.write_text(text + "\n", encoding="utf-8")
    print(text)
    return 0 if report["all_in_band"] else 1


def _row(arm: str, family: str, level: int, instance: str, seed: int, label: int, usd: float) -> dict:
    """The fields of a run record that the bootstrap, the envelope and metrics' readers use."""
    return {"arm": arm, "seed": seed, "run_id": f"sim-{arm}", "task": {"family": family, "ladder": level,
                                                                       "instance_id": instance,
                                                                       "spec_variant": "precise",
                                                                       "is_honeypot": False},
            "execution": {"status": "completed"}, "vs": {"label": label, "unknown": False},
            "costs": {"api_equiv_usd": usd}, "_vs": label}


def _rate_a(rows: list[dict]) -> float:
    vs = [row["_vs"] for row in rows if row["arm"] == "A"]
    return sum(vs) / len(vs)


def _difference(rows: list[dict]) -> float:
    counts = {"A": [0, 0], "B": [0, 0]}
    for row in rows:
        counts[row["arm"]][0] += row["_vs"]
        counts[row["arm"]][1] += 1
    return counts["A"][0] / counts["A"][1] - counts["B"][0] / counts["B"][1]


def _ratio(rows: list[dict]) -> float:
    counts = {"A": [0, 0], "B": [0, 0]}
    for row in rows:
        counts[row["arm"]][0] += row["_vs"]
        counts[row["arm"]][1] += 1
    if not counts["B"][0]:
        return 0.0
    return (counts["A"][0] / counts["A"][1]) / (counts["B"][0] / counts["B"][1])


def _tasks(rows: list[dict]) -> dict[str, dict]:
    by_task: dict[str, dict] = {}
    for row in rows:
        task = by_task.setdefault(row["task"]["instance_id"], {"A": [], "B": [], "p_b": row["_p_b"]})
        task[row["arm"]].append(row["_vs"])
    return by_task


def _cuped(rows: list[dict], rng: Random) -> cuped.Cuped:
    """A between-task contrast: a coin gives each task one arm; its covariate is three pre-assignment runs under B,
    drawn apart from the campaign's runs (as LOG1's are), so it carries none of their noise."""
    first, second = [], []
    for runs in _tasks(rows).values():
        covariate = sum(rng.random() < runs["p_b"] for _ in range(3)) / 3
        arm = "A" if rng.random() < 0.5 else "B"
        (first if arm == "A" else second).append((sum(runs[arm]) / len(runs[arm]), covariate))
    return cuped.adjusted_contrast(first, second)


def _paired_stream(rows: list[dict], rng: Random) -> list[float]:
    """Per task, the mean of A's runs minus B's, in a random stream order."""
    stream = [sum(runs["A"]) / len(runs["A"]) - sum(runs["B"]) / len(runs["B"]) for runs in _tasks(rows).values()]
    rng.shuffle(stream)
    return stream


if __name__ == "__main__":
    sys.exit(main())
