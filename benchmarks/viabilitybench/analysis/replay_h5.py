"""R-H5: S05's audit-lottery replay over LOG1 (S09 §4.3 "Replays" and H5 (i); S05 SC1; task 3356).

    vb.py replay --experiment LOG1 --adapter h5 [--seed N] [--reps LOTTERIES] [--arm LABEL] [--risk FILE --lam L]

An adapter under `replay_runner` (3354), so it runs deterministically on arm-hashed data like every replay. It takes
the Roko records of LOG1's blocks A and E (streams `p1_core` and `log1_f8_honeypots`, every one VS-census
labelled), turns them into S05's census `vs.label` rows (`audit.labels.label_rows`) and green units
(`audit.replay.units_from_labels`), and runs S05's keyed lottery replay (`audit.replay.replay`, 1,000 lotteries by
default) at ρ ∈ {0.10, 0.15, 0.30} for each selection. Each cell reports its coverage of θ_census, the bias of the
Hájek estimate, its θ ratio (mean θ̂_H / θ_census), Kish n_eff and whether S05 SC1's targets hold (coverage ≥ 0.93,
|bias| ≤ 0.01). The primary cell is ρ = 0.15 with tilted selection, Hájek with a Wilson interval at Kish n_eff
(S09 H5 (i)); the other eight cells and θ̂_HT are secondary.

**Which records.** With arms hashed the adapter cannot name `roko_fixed`, so by default it takes the records whose
attempts carry a gate verdict, which only the Roko arms record; `--arm LABEL` (the label `Blinder.label` gives
`roko_fixed`, which needs no lock) takes one arm instead.

**The selections.**
- `uniform`: λ = 0, S05's first-slice lottery.
- `tilted`: S05's default π, tilted by M3's risk for each unit (`--risk`, JSONL rows {"attempt_key", "r"}) with
  `--lam` (S05 gates λ by M3's calibration, `audit.lottery.tilt`). Without M3's risk (R-M3 has not run on LOG1)
  the tilted cells, the primary one among them, are not evaluated: the replay never stands another selection in.
- `ai`: an AI selector's choices of what to audit, which LOG1 does not record, so these cells are not evaluated
  until such a selector's choices exist to replay.

**Seeds.** S05's lotteries are keyed by a seed text; the adapter derives it from the replay's generator
(`r-h5-<64 bits>`), so one replay seed always gives the same lotteries and the same bytes.

**The draw order.** `estimate` folds each lottery into its cells. A sequential estimand needs the draws themselves:
`draw_order` gives each lottery's draw for each unit, keyed the same way, and `stream_units` one stream's units in its
own position order (X3's false-green step, `replay_closure.py`).

API:
    SELECTIONS, PRIMARY, STREAMS, LOTTERIES
    estimate(matrix, rng, reps=LOTTERIES, risk=None, lam=None, arm=None) -> dict
    units(matrix, *, arm=None, streams=STREAMS, risk=None) -> list[audit.replay.Unit]
    stream_units(matrix, stream, *, arm=None) -> list[audit.replay.Unit]
    draw_order(units, rng, reps=LOTTERIES) -> dict
"""

from __future__ import annotations

import json
import random
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path

if str(Path(__file__).resolve().parents[1]) not in sys.path:  # the benchmark directory, for the audit package
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import replay_runner  # noqa: E402
from audit import labels as audit_labels  # noqa: E402
from audit import replay as lottery_replay  # noqa: E402

SELECTIONS = ("uniform", "tilted", "ai")
PRIMARY = (0.15, "tilted")  # S09 H5 (i), S05 SC1
STREAMS = ("p1_core", "log1_f8_honeypots")  # LOG1's blocks A and E
LOTTERIES = 1000


def units(found: replay_runner.OutcomeMatrix, *, arm: str | None = None, streams: Sequence[str] = STREAMS,
          risk: Mapping[str, float] | None = None) -> list[lottery_replay.Unit]:
    """The green census units of the matrix's block A and E records: one arm's, or every gated (Roko) record's."""
    chosen = [record for row, record in zip(found.rows, found.records)
              if row.stream in streams and (row.arm == arm if arm is not None else _gated(record))]
    return lottery_replay.units_from_labels(audit_labels.label_rows(chosen), risk)


def stream_units(found: replay_runner.OutcomeMatrix, stream: str, *,
                 arm: str | None = None) -> list[lottery_replay.Unit]:
    """The green census units of one stream, in its position order: one arm's, or every gated (Roko) record's."""
    chosen = sorted((record for row, record in zip(found.rows, found.records)
                     if row.stream == stream and (row.arm == arm if arm is not None else _gated(record))),
                    key=lambda record: record["stream"].get("position") or 0)
    return lottery_replay.units_from_labels(audit_labels.label_rows(chosen))


def draw_order(drawn: Sequence[lottery_replay.Unit], rng: random.Random, reps: int = LOTTERIES) -> dict:
    """The lottery replay's per-position draw order over `drawn`: each of `reps` lotteries' draw for each unit, in
    order, with the seed `estimate` would derive from `rng`."""
    seed = f"r-h5-{rng.getrandbits(64):016x}"
    return {"seed": seed, "lotteries": int(reps), "draws": lottery_replay.draw_order(drawn, runs=int(reps), seed=seed)}


def estimate(found: replay_runner.OutcomeMatrix, rng: random.Random, reps: int = LOTTERIES, risk: str | None = None,
             lam: float | None = None, arm: str | None = None) -> dict:
    """R-H5's nine cells over the matrix (module docstring)."""
    risks = _risk(Path(risk)) if risk else None
    drawn = units(found, arm=arm, risk=risks)
    if not drawn:
        raise ValueError("no green census unit in LOG1's blocks A and E to replay")
    seed = f"r-h5-{rng.getrandbits(64):016x}"
    reports = {"uniform": lottery_replay.replay(drawn, runs=int(reps), seed=seed, lam=0.0)}
    reasons = {"ai": "no AI selector's choices are recorded in LOG1"}
    if risks is None or not lam:
        reasons["tilted"] = "no M3 risk for the units (--risk with --lam): R-M3 has not run on LOG1"
    else:
        reports["tilted"] = lottery_replay.replay(drawn, runs=int(reps), seed=seed, lam=float(lam))
    first = reports["uniform"]
    cells = []
    for selection in SELECTIONS:
        for rho in lottery_replay.RHOS:
            cell = {"rho": rho, "selection": selection, "primary": (rho, selection) == PRIMARY}
            if selection in reports:
                [summary] = [one for one in reports[selection]["cells"] if one["rho"] == rho]
                hajek, theta = summary["mean_theta_hajek"], reports[selection]["theta_census"]
                cell.update(summary, evaluated=True,
                            theta_ratio=hajek / theta if hajek is not None and theta else None)
            else:
                cell.update(evaluated=False, reason=reasons[selection])
            cells.append(cell)
    [primary] = [cell for cell in cells if cell["primary"]]
    verdict = ("not evaluated" if not primary["evaluated"] else "pass" if primary["targets_met"] else "fail")
    return {"lotteries": int(reps), "seed": seed, "lambda": lam if "tilted" in reports else None,
            "n_units": first["n_units"], "false_greens": first["false_greens"], "theta_census": first["theta_census"],
            "unknown_labels": first["unknown_labels"], "census_strata": first["census_strata"],
            "targets": {"coverage": lottery_replay.TARGET_COVERAGE, "bias": lottery_replay.TARGET_BIAS},
            "primary": {"rho": PRIMARY[0], "selection": PRIMARY[1], "verdict": verdict}, "cells": cells}


replay_runner.register(replay_runner.Adapter("h5", "r-h5-1", estimate, params=("reps", "risk", "lam", "arm")))


def _gated(record: Mapping) -> bool:
    """Whether the record's attempts carry a gate verdict: a Roko arm's, whatever its blinded label."""
    return any(attempt.get("gate_verdict") for attempt in record["execution"]["attempts"])


def _risk(path: Path) -> dict[str, float]:
    """M3's risk per attempt_key, from JSONL rows {"attempt_key", "r"}."""
    risks = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict) or not isinstance(row.get("attempt_key"), str) or not isinstance(
                row.get("r"), (int, float)):
            raise ValueError(f"{path}:{number}: not a {{\"attempt_key\", \"r\"}} row")
        risks[row["attempt_key"]] = float(row["r"])
    return risks
