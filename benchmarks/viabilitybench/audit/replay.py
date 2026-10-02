#!/usr/bin/env python3
"""Lottery replays over a census-labelled stream (S05 §7.1, SC1): does the Hájek interval cover the census rate?

Every green unit of the stream has a census label, so its false-green rate θ_census is known. `replay` draws N
independent lotteries over the same units at each ρ, the way production draws (`lottery.draw`, one key per
lottery), estimates θ from the selected units alone (`estimate.estimate`), and reports for each ρ the mean audits,
n_eff, θ̂_H and interval, the interval's coverage of θ_census, and the bias of θ̂_H and θ̂_HT. SC1's targets are
coverage ≥ 0.93 and |bias| ≤ 0.01 (`TARGET_COVERAGE`, `TARGET_BIAS`).

- **The units** are the census rows of a vs.label file (`--labels`) whose `verdict` is green, except gold units
  (7110), which never enter these numbers. Census strata (`unverified`, `forced_accept`) are drawn at π = 1, so
  their rate needs no lottery: they are counted apart. Y = 1 − vs, so an unknown label is a false green, as VS
  counts it 0; θ_census with unknown labels counted as VS is printed beside it.
- **The lotteries are keyed.** Lottery r uses K_r = `lottery.run_key(secret, "replay-<r>")`, the secret being
  SHA-256 of `--seed`, so a replay reproduces exactly. One set of keys serves every ρ. Each unit's draw uses its own
  run_id, task_id, attempt_key (as the attempt id) and final_commit, as production's would.
- **Tilted selection** needs M3's risk for each unit (`--risk FILE`, JSONL rows {"attempt_key", "r"}) and λ
  (`--lam`); without them the lottery is uniform, as in S05's first slice.
- **Empty samples.** At a low ρ a small stream sometimes selects nothing. That lottery has no Hájek estimate and the
  vacuous interval, which covers; the count is printed, and the Hájek columns average over the other lotteries.
  The HT columns average over every lottery, an empty one estimating 0, which is what makes HT unbiased.
- The interval is Wilson's at Kish n_eff in every cell (gap-a499aa). With about 60 units at ρ = 0.10 a lottery
  audits about 6, so read n_eff beside the coverage (S05.2's risk).

`--write-fixture PATH` writes `build_fixture()`, the reference inputs and outputs of the lottery and the estimators
that roko-gate's `audit` module (7114) must reproduce, to `fixtures/estimators.json` by convention.

Usage: replay.py --labels FILE [--runs N] [--rho R ...] [--seed TEXT] [--risk FILE --lam L] [--json]
       replay.py --write-fixture PATH
Exit status 0 when the report printed, whatever it says about the targets; 2 on a usage or input error.

API:
    Unit(attempt_key, run_id, task_id, final_commit, y, verdict, unknown=False, risk=None)
    units_from_labels(rows, risk=None) -> list[Unit]
    replay(units, *, rhos=RHOS, runs=1000, seed=DEFAULT_SEED, lam=0.0, alpha=0.05) -> dict
    format_report(report) -> str
    build_fixture() -> dict
    main(argv=None) -> int
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

if __package__ in (None, ""):  # run as a script: import the package from the benchmark directory
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import audit  # noqa: E402
from audit import estimate, lottery  # noqa: E402

RHOS = (0.10, 0.15, 0.30)
TARGET_COVERAGE = 0.93
TARGET_BIAS = 0.01
DEFAULT_SEED = "s05-replay"
FIXTURE_VERSION = "vb.audit.fixture/1"


class ReplayError(ValueError):
    """The labels cannot be replayed: no census rows, a green unit without a commit, or a missing risk."""


@dataclass(frozen=True)
class Unit:
    """One green unit of a census-labelled stream."""

    attempt_key: str
    run_id: str
    task_id: str
    final_commit: str
    y: int  # 1 = false green (VS 0, an unknown label included)
    verdict: str
    unknown: bool = False
    risk: float | None = None


def units_from_labels(rows: Iterable[Mapping], risk: Mapping[str, float] | None = None) -> list[Unit]:
    """The green, non-gold units of valid vs.label census rows, with M3's risk per attempt_key when given."""
    units = []
    for row in rows:
        if row["vs_source"] != "census":
            raise ReplayError(f"{row['attempt_key']}: a replay needs census labels, not vs_source {row['vs_source']!r}")
        if row["verdict"] is None or row.get("gold", False):
            continue
        if row["final_commit"] is None:
            raise ReplayError(f"{row['attempt_key']}: a green unit needs the final commit its draw is keyed on")
        units.append(Unit(row["attempt_key"], row["run_id"], row["task_id"], row["final_commit"],
                          audit.false_green(row), row["verdict"], row["unknown"],
                          None if risk is None else risk.get(row["attempt_key"])))
    return units


def replay(units: Sequence[Unit], *, rhos: Sequence[float] = RHOS, runs: int = 1000, seed: str = DEFAULT_SEED,
           lam: float = 0.0, alpha: float = 0.05) -> dict:
    """`runs` keyed lotteries over `units` at each ρ: the report `format_report` prints (module docstring)."""
    if runs < 1:
        raise ValueError(f"runs must be at least 1, not {runs}")
    drawn = [unit for unit in units if unit.verdict not in audit.CENSUS_VERDICTS]
    census = [unit for unit in units if unit.verdict in audit.CENSUS_VERDICTS]
    if not drawn:
        raise ReplayError("no green unit is drawn at the policy's π (every unit is a census stratum or none is green)")
    if lam > 0.0 and any(unit.risk is None for unit in drawn):
        missing = sum(unit.risk is None for unit in drawn)
        raise ReplayError(f"a tilted lottery (lambda {lam}) needs M3's risk for every unit; {missing} have none")
    n = len(drawn)
    theta = sum(unit.y for unit in drawn) / n
    mean_risk = math.fsum(unit.risk for unit in drawn) / n if lam > 0.0 else None
    pis = {rho: [lottery.inclusion_probability(rho, risk=unit.risk, mean_risk=mean_risk, lam=lam) for unit in drawn]
           for rho in rhos}
    cells = {rho: _Cell() for rho in rhos}
    secret = hashlib.sha256(seed.encode("utf-8")).digest()
    for r in range(runs):
        key = lottery.run_key(secret, f"replay-{r}")
        draws = [lottery.draw(key, unit.run_id, unit.task_id, unit.attempt_key, unit.final_commit) for unit in drawn]
        for rho in rhos:
            sample = [(pi, unit.y) for x, pi, unit in zip(draws, pis[rho], drawn) if lottery.is_selected(x, pi)]
            cells[rho].add(estimate.estimate(sample, n, alpha), theta)
    return {
        "runs": runs, "seed": seed, "alpha": alpha, "lambda": lam, "selection": "tilted" if lam > 0.0 else "uniform",
        "n_units": n, "false_greens": sum(unit.y for unit in drawn), "theta_census": theta,
        "unknown_labels": sum(unit.unknown for unit in drawn),
        "theta_census_unknown_as_vs": sum(unit.y for unit in drawn if not unit.unknown) / n,
        "verdicts": {verdict: sum(unit.verdict == verdict for unit in drawn)
                     for verdict in sorted({unit.verdict for unit in drawn})},
        "census_strata": {verdict: {"units": sum(unit.verdict == verdict for unit in census),
                                    "false_greens": sum(unit.y for unit in census if unit.verdict == verdict)}
                          for verdict in audit.CENSUS_VERDICTS},
        "cells": [cells[rho].summary(rho, theta, pis[rho]) for rho in rhos],
    }


def format_report(report: dict) -> str:
    """The replay as a text table, one row per ρ."""
    lines = [
        f"S05 lottery replay: {report['runs']} lotteries per rho over {report['n_units']} green units "
        f"({report['selection']} selection, lambda {report['lambda']}, seed {report['seed']!r}, "
        f"alpha {report['alpha']})",
        f"theta_census = {report['theta_census']:.4f} ({report['false_greens']} false greens of "
        f"{report['n_units']}; {report['unknown_labels']} unknown labels counted as false greens; with them as VS: "
        f"{report['theta_census_unknown_as_vs']:.4f})",
        "verdicts drawn at the policy's pi: "
        + ", ".join(f"{name} {count}" for name, count in report["verdicts"].items()),
        "census strata (pi = 1, reported apart): " + ", ".join(
            f"{name} {cell['units']} units, {cell['false_greens']} false greens"
            for name, cell in report["census_strata"].items()),
        f"targets: coverage >= {TARGET_COVERAGE}, |bias| <= {TARGET_BIAS}",
        "",
        f"{'rho':>5} {'pi_min':>7} {'audits':>7} {'n_eff':>6} {'theta_H':>8} {'CI (mean)':>17} {'coverage':>9} "
        f"{'|bias|':>7} {'HT |bias| (mc se)':>18} {'empty':>6}  targets",
    ]
    for cell in report["cells"]:
        theta_hajek = "-" if cell["mean_theta_hajek"] is None else f"{cell['mean_theta_hajek']:.4f}"
        ci = "-" if cell["mean_ci"] is None else f"[{cell['mean_ci'][0]:.3f}, {cell['mean_ci'][1]:.3f}]"
        bias = "-" if cell["bias_hajek"] is None else f"{abs(cell['bias_hajek']):.4f}"
        ht = f"{abs(cell['bias_ht']):.4f} ({cell['ht_mc_se']:.4f})"
        lines.append(f"{cell['rho']:>5.2f} {cell['pi_min']:>7.3f} {cell['mean_audited']:>7.1f} "
                     f"{cell['mean_n_eff']:>6.1f} {theta_hajek:>8} {ci:>17} {cell['coverage']:>9.3f} {bias:>7} "
                     f"{ht:>18} {cell['empty']:>6}  "
                     f"{'met' if cell['targets_met'] else 'NOT MET'}")
    return "\n".join(lines)


class _Cell:
    """One ρ's running totals over the lotteries."""

    def __init__(self) -> None:
        self.runs = self.covered = self.empty = 0
        self.audited: list[int] = []
        self.n_eff: list[float] = []
        self.hajek: list[float] = []
        self.low: list[float] = []
        self.high: list[float] = []
        self.ht: list[float] = []

    def add(self, est: estimate.Estimate, theta: float) -> None:
        self.runs += 1
        low, high = est.ci
        self.covered += low <= theta <= high
        self.audited.append(est.n_audited)
        self.n_eff.append(est.n_eff)
        self.ht.append(est.theta_ht)
        if est.theta_hajek is None:
            self.empty += 1
            return
        self.hajek.append(est.theta_hajek)
        self.low.append(low)
        self.high.append(high)

    def summary(self, rho: float, theta: float, pis: Sequence[float]) -> dict:
        mean_ht = math.fsum(self.ht) / self.runs
        spread = math.fsum((value - mean_ht) ** 2 for value in self.ht) / max(1, self.runs - 1)
        mean_hajek = math.fsum(self.hajek) / len(self.hajek) if self.hajek else None
        mean_ci = [math.fsum(self.low) / len(self.low), math.fsum(self.high) / len(self.high)] if self.hajek else None
        coverage = self.covered / self.runs
        bias_hajek = None if mean_hajek is None else mean_hajek - theta
        return {
            "rho": rho, "pi_min": min(pis), "pi_mean": math.fsum(pis) / len(pis),
            "mean_audited": sum(self.audited) / self.runs, "mean_n_eff": math.fsum(self.n_eff) / self.runs,
            "mean_theta_hajek": mean_hajek, "mean_ci": mean_ci,
            "coverage": coverage, "bias_hajek": bias_hajek, "bias_ht": mean_ht - theta,
            "ht_mc_se": math.sqrt(spread / self.runs), "empty": self.empty,
            "targets_met": coverage >= TARGET_COVERAGE and bias_hajek is not None and abs(bias_hajek) <= TARGET_BIAS,
        }


def build_fixture() -> dict:
    """The reference inputs and outputs for roko-gate's audit module (7114): every number to 1e-12."""
    secret = bytes(range(32))
    run_id = "run-2026-10-02-fixture"
    key = lottery.run_key(secret, run_id)
    other = lottery.run_key(secret, "run-other")
    draw_inputs = [
        (run_id, "F1-l2-0001", "run-2026-10-02-fixture:plan:F1-l2-0001:1", "3f2c9a1e"),
        (run_id, "F4-l3-0017", "run-2026-10-02-fixture:plan:F4-l3-0017:2", "b81e0c55d3"),
        (run_id, "tâche-ü", "attempt:ü:3", "tree:0123456789abcdef0123456789abcdef01234567"),
        (run_id, "", "", ""),
        ("run-other", "F1-l2-0001", "run-other:plan:F1-l2-0001:1", "3f2c9a1e"),
    ]
    draws = []
    for fields in draw_inputs:
        used = other if fields[0] == "run-other" else key
        x = lottery.draw(used, *fields)
        draws.append({"key_hex": used.hex(), "run_id": fields[0], "task_id": fields[1], "attempt_id": fields[2],
                      "accepted_commit": fields[3], "prf_u": lottery.prf_hex(x), "u": lottery.u_value(x),
                      "selected": {str(pi): lottery.is_selected(x, pi) for pi in (0.05, 0.1, 0.15, 0.3, 0.5, 1.0)}})
    boundaries = []
    for pi in (0.05, 0.15, 0.3, 1.0):
        edge = min(int(pi * lottery.TWO_64), (1 << 64) - 1)
        for x in (0, edge - 1, edge, (1 << 64) - 1):
            boundaries.append({"prf_u": lottery.prf_hex(x), "pi": pi, "selected": lottery.is_selected(x, pi)})
    probabilities = []
    for rho, risk, mean_risk, lam, alpha, pi_max in (
            (0.10, None, None, 0.0, 1.0, 1.0), (0.15, 0.2, 0.1, 0.8, 1.0, 1.0), (0.15, 0.0, 0.1, 0.8, 1.0, 1.0),
            (0.30, 0.9, 0.1, 0.8, 1.0, 1.0), (0.30, 0.9, 0.1, 0.8, 1.0, 0.6), (0.05, 0.05, 0.2, 0.5, 2.0, 1.0),
            (0.15, 0.4, 0.0, 0.8, 1.0, 1.0), (0.5, 1.0, 0.5, 1.0, 0.5, 1.0)):
        probabilities.append({"rho": rho, "risk": risk, "mean_risk": mean_risk, "lam": lam, "alpha": alpha,
                              "pi_max": pi_max, "pi": lottery.inclusion_probability(
                                  rho, risk=risk, mean_risk=mean_risk, lam=lam, alpha=alpha, pi_max=pi_max)})
    tilts = [{"ece": ece, "n_labels": labels, "lambda_max": lottery.LAMBDA_MAX, "ece_ref": lottery.ECE_REF,
              "lam": lottery.tilt(ece, labels)}
             for ece, labels in ((None, 80), (0.02, 49), (0.02, 50), (0.05, 120), (0.10, 200), (0.3, 200))]
    wilsons = [{"p": p, "n": n, "z": estimate.Z95, "ci": list(estimate.wilson(p, n))}
               for p, n in ((0.0, 6.0), (1.0, 6.0), (0.2, 9.0), (0.15, 23.7), (0.5, 0.0), (0.3, 1e6))]
    tilted = [(0.05, 0), (0.05, 1), (0.08, 0), (0.12, 1), (0.4, 1), (0.4, 0), (0.8, 1), (1.0, 0), (1.0, 1)]
    large = [(0.3, 1)] * 12 + [(0.3, 0)] * 48  # n_eff 60 with 12 events: Wald's cell under S05 §4.5's first rule
    estimates = [{"name": name, "n_green": n_green, "alpha": 0.05, "units": [list(unit) for unit in units],
                  "expected": estimate.estimate(units, n_green).record()}
                 for name, n_green, units in (
                     ("wilson", 60, [(0.15, 1), (0.15, 0), (0.15, 0), (0.15, 1), (0.15, 0), (0.15, 0), (0.15, 0),
                                     (0.15, 0), (0.15, 0)]),
                     ("n_eff_60", 200, large), ("tilted", 120, tilted), ("empty", 50, []),
                     ("no_events", 40, [(0.1, 0)] * 6), ("all_events", 40, [(0.2, 1)] * 4),
                     ("census", 40, [(1.0, 1)] * 6 + [(1.0, 0)] * 34))]
    nulls = []
    for n_green, units in ((80, [(0.15, 1), (0.15, None), (0.3, 0), (0.05, None), (1.0, 0)]),):
        low, high = estimate.null_bounds(units, n_green)
        nulls.append({"n_green": n_green, "units": [list(unit) for unit in units], "missing_0": low.record(),
                      "missing_1": high.record()})
    betting = []
    for name, rho, lam, z_max in (("uniform", 0.5, 0.0, 1.0), ("uniform_scaled", 0.5, 0.0, lottery.EPS_FLOOR / 0.5),
                                  ("tilted", 0.3, 0.8, 1.0)):
        z = _fixture_stream(key, rho, lam)
        betting.append({"name": name, "alpha": 0.05, "z_max": z_max, "grid": 100, "z": z,
                        "capital": {str(theta): estimate.hedged_capital([value / z_max for value in z],
                                                                        lottery.EPS_FLOOR * theta / z_max)
                                    for theta in (0.0, 0.1, 0.3, 1.0)},
                        "cs": [None if entry is None else list(entry)
                               for entry in estimate.betting_cs(z, z_max=z_max, grid=100)]})
    return {
        "version": FIXTURE_VERSION,
        "about": "Reference inputs and outputs of benchmarks/viabilitybench/audit (lottery.py, estimate.py) for "
                 "roko-gate's audit module (7114). Regenerate with audit/replay.py --write-fixture; "
                 "audit/tests/test_estimate.py checks this file against the code to 1e-12.",
        "constants": {"eps_floor": lottery.EPS_FLOOR, "z95": estimate.Z95, "draw_tag": lottery.DRAW_TAG,
                      "key_tag": lottery.KEY_TAG, "ci_method": estimate.CI_METHOD, "cs_c": estimate.CS_C,
                      "cs_hedge": estimate.CS_HEDGE},
        "run_key": [{"secret_hex": secret.hex(), "run_id": run_id, "key_hex": key.hex()},
                    {"secret_hex": secret.hex(), "run_id": "run-other", "key_hex": other.hex()}],
        "commitment": [{"key_hex": key.hex(), "run_id": run_id, "commitment": lottery.commitment(key, run_id)}],
        "draws": draws,
        "is_selected": boundaries,
        "inclusion_probability": probabilities,
        "tilt": tilts,
        "wilson": wilsons,
        "estimates": estimates,
        "null_bounds": nulls,
        "betting": betting,
    }


def _fixture_stream(key: bytes, rho: float, lam: float) -> list[float]:
    """The Z_i of 120 units drawn with `key`, whose (risk, label) cycle through eight kinds, 5 of them false greens.

    The rate is high so that the sequence narrows within the fixture.
    """
    kinds = ((0.3, 1), (0.6, 1), (0.9, 0), (0.6, 1), (0.3, 0), (0.9, 1), (0.6, 1), (0.3, 0))
    mean_risk = math.fsum(risk for risk, _ in kinds) / len(kinds)
    out = []
    for i in range(120):
        risk, y = kinds[i % len(kinds)]
        pi = lottery.inclusion_probability(rho, risk=risk, mean_risk=mean_risk, lam=lam)
        x = lottery.draw(key, "fixture-stream", f"T{i:02d}", f"fixture-stream:T{i:02d}:1", f"c{i:02d}")
        out.append(estimate.audit_z(lottery.is_selected(x, pi), y, pi))
    return out


def _load_risk(path: Path) -> dict[str, float]:
    risk = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
            key, value = row["attempt_key"], row["r"]
        except (json.JSONDecodeError, KeyError, TypeError) as err:
            raise ReplayError(f'{path}:{number}: a risk row is {{"attempt_key", "r"}} ({type(err).__name__})') from None
        if not isinstance(value, (int, float)) or isinstance(value, bool) or not 0.0 <= value <= 1.0:
            raise ReplayError(f"{path}:{number}: a risk is a probability in [0, 1], not {value!r}")
        risk[key] = float(value)
    return risk


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Replay S05's audit lottery over a census-labelled stream.")
    parser.add_argument("--labels", type=Path, help="vs.label JSONL with census rows")
    parser.add_argument("--runs", type=int, default=1000, help="lotteries per rho (default 1000)")
    parser.add_argument("--rho", type=float, nargs="+", default=list(RHOS), help="base rates (default 0.10 0.15 0.30)")
    parser.add_argument("--seed", default=DEFAULT_SEED, help="text the lottery keys derive from")
    parser.add_argument("--risk", type=Path, help="JSONL of {\"attempt_key\", \"r\"}: M3's risk, for a tilted lottery")
    parser.add_argument("--lam", type=float, default=0.0, help="the tilt lambda in [0, 1] (needs --risk)")
    parser.add_argument("--json", action="store_true", help="print the report as JSON")
    parser.add_argument("--write-fixture", type=Path, metavar="PATH", help="write the 7114 fixture and exit")
    args = parser.parse_args(argv)
    if args.write_fixture is not None:
        args.write_fixture.write_text(json.dumps(build_fixture(), indent=1, ensure_ascii=False) + "\n",
                                      encoding="utf-8")
        print(f"wrote {args.write_fixture}")
        return 0
    if args.labels is None:
        parser.error("--labels is required")
    if args.lam > 0.0 and args.risk is None:
        parser.error("--lam needs --risk")
    try:
        risk = None if args.risk is None else _load_risk(args.risk)
        units = units_from_labels(audit.load_labels(args.labels), risk)
        report = replay(units, rhos=args.rho, runs=args.runs, seed=args.seed, lam=args.lam)
    except (OSError, ValueError) as err:  # LabelError and ReplayError are ValueErrors
        print(f"replay: {err}", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=1) if args.json else format_report(report))
    return 0


if __name__ == "__main__":
    sys.exit(main())
