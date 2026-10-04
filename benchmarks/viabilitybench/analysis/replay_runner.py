#!/usr/bin/env python3
"""`vb replay`: deterministic replays of a logged campaign, on arm-hashed data, with an A/A check (S09 E6; 3354).

    vb.py replay --experiment LOG1 --adapter aa [--seed N] [--arm LABEL] [--reps N] [--results DIR]
                 [--salt FILE | --unblinded] [--out FILE]

S09 §4.3's replays (R-H4, R-H5, R-H6, R-M3, the closure replays X1-X4 and S10's demo mode) are $0 evidence for G3
only if they are reproducible and calibrated, so all of them run through this one driver.

**The protocol.** A replay is an `Adapter`: a name, a version, the parameters it takes, and `estimate(matrix, rng,
**params) -> dict`, a JSON-able document of estimates. It reads the campaign only through the outcome matrix and
draws randomness only from `rng`, a `random.Random` seeded with the replay's seed. `register` adds one to `ADAPTERS`;
an adapter in a module of its own (`replay_h5.py`, 3356) is imported by name on first use (`ADAPTER_MODULES`).

**The outcome matrix** (`OutcomeMatrix`): one `Row` per run record, in `replay.load`'s stable order (experiment, run,
record id): its experiment, run, seed, arm, model, task (family, instance, level, spec variant), stream, status, VS
label (`metrics.vs_minus`), API-equivalent cost and stream position, with the (blinded) records themselves beside the
rows for an adapter that needs more of them (R-H5 builds S05's census labels from them). Its `sha256` is over the
canonical JSON of both, so a replay's output names exactly the data it read.

**Arm-hashed data.** Before the pre-registration lock, `replay.load` reads every arm through 3341's `Blinder`, whose
salt file (`--salt`, else `blind.DEFAULT_SALT`) the operator keeps: rows name `arm-<hash>` labels, never arm ids.
`--unblinded` reads arm ids, and only once `lock.require` accepts the lock (3345); before that it is refused.

**Determinism.** Rows in a fixed order, one seeded generator, and a canonical output: JSON with sorted keys, fixed
separators and floats rounded to 12 significant digits, with no clock. Two replays of one adapter with one seed on one
tree are byte-identical (`canonical`). `export` writes that text, which S10's replay mode reads as it is, so R-demo
needs no driver of its own.

**The A/A check** (adapter `aa`). One arm's tasks are split at random into two pseudo-arms, each task's runs going
whole to one half (seeds of a task are not independent), and the difference of their mean VS gets a 95% interval
(Welch's normal interval over the per-task means). The halves differ only by chance, so over `reps` seeded splits the
interval should cover 0 at about the nominal 95%: the check passes when the coverage is inside the binomial 99% band
around it. A replay estimator whose intervals fail this on its own data is not calibrated there.

The other adapter here, `vs_rates`, gives each arm's VS rate with its Wilson interval, the house rule (gap-a499aa).

API:
    Row; OutcomeMatrix(rows).sha256() / .arms() / .of_arm(arm); matrix(campaign) -> OutcomeMatrix
    Adapter(name, version, estimate, params=()); ADAPTERS; ADAPTER_MODULES; register(adapter); adapter(name)
    run(results, experiments, name, *, seed, blind=None, params=None) -> dict
    canonical(doc) -> str; export(doc, path) -> Path
    aa_split(matrix, arm, rng) -> dict; aa_check(matrix, rng, arm=None, reps=AA_REPS) -> dict
    mean_difference(a, b, z=Z95) -> (diff, low, high); wilson(successes, n, z=Z95) -> (low, high)
    main(argv) -> int        # --risk FILE and --lam L reach the adapters that take them (R-H5's tilted cells)
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import math
import os
import random
import sys
from collections.abc import Callable, Sequence
from dataclasses import asdict, dataclass, field
from pathlib import Path

import blind
import lock
import metrics
import replay
import report

REPLAY_SCHEMA = "vb.replay/1"
Z95 = 1.959963984540054  # the two-sided 95% normal quantile
Z99 = 2.5758293035489004  # the two-sided 99% normal quantile, for the A/A coverage band
AA_NOMINAL = 0.95
AA_REPS = 200
SIGNIFICANT = 12  # digits a float keeps in the canonical output
ADAPTER_MODULES = {"h5": "replay_h5"}  # adapters in modules of their own, imported on first use


@dataclass(frozen=True)
class Row:
    experiment: str
    run_id: str
    seed: int
    arm: str
    model: str | None
    family: str
    instance: str
    level: int
    spec_variant: str
    stream: str
    status: str
    vs: int
    cost_usd: float | None
    position: int | None

    @property
    def task(self) -> tuple[str, str, str]:
        """The task: its family, instance and spec variant."""
        return self.family, self.instance, self.spec_variant


@dataclass(frozen=True)
class OutcomeMatrix:
    rows: tuple[Row, ...]
    records: tuple[dict, ...] = ()  # the run records behind the rows, in the same order

    def sha256(self) -> str:
        text = canonical({"rows": [asdict(row) for row in self.rows], "records": list(self.records)})
        return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()

    def arms(self) -> list[str]:
        return sorted({row.arm for row in self.rows})

    def of_arm(self, arm: str) -> list[Row]:
        """The arm's rows the report keeps (`metrics.EXCLUDED` left out), in matrix order."""
        return [row for row in self.rows if row.arm == arm and row.status not in metrics.EXCLUDED]


@dataclass(frozen=True)
class Adapter:
    name: str
    version: str
    estimate: Callable[..., dict]  # (matrix, rng, **params) -> a JSON-able document
    params: tuple[str, ...] = field(default=())  # the parameters `estimate` takes, from the command line


ADAPTERS: dict[str, Adapter] = {}


def register(found: Adapter) -> Adapter:
    ADAPTERS[found.name] = found
    return found


def adapter(name: str) -> Adapter:
    """The adapter called `name`, importing its module first if it lives in one (`ADAPTER_MODULES`)."""
    if name not in ADAPTERS and name in ADAPTER_MODULES:
        importlib.import_module(ADAPTER_MODULES[name])
    if name not in ADAPTERS:
        raise KeyError(f"no replay adapter {name!r}; known: {', '.join(sorted({*ADAPTERS, *ADAPTER_MODULES}))}")
    return ADAPTERS[name]


def matrix(campaign: replay.Campaign) -> OutcomeMatrix:
    """The outcome matrix of a loaded campaign, one row per record in its stable order."""
    rows = []
    for entry in campaign.entries:
        record = entry.record
        task = record["task"]
        models = {attempt.get("model_requested") for attempt in record["execution"]["attempts"]}
        cost = record["costs"]["api_equiv_usd"]
        rows.append(Row(experiment=record["experiment_id"], run_id=record["run_id"], seed=record["seed"],
                        arm=record["arm"], model=models.pop() if len(models) == 1 else None, family=task["family"],
                        instance=task["instance_id"], level=task["ladder"],
                        spec_variant=task.get("spec_variant", "precise"),
                        stream=str(record["stream"].get("id", "")), status=record["execution"]["status"],
                        vs=metrics.vs_minus(record), cost_usd=float(cost) if cost is not None else None,
                        position=record["stream"].get("position")))
    return OutcomeMatrix(tuple(rows), tuple(entry.record for entry in campaign.entries))


def run(results: Path, experiments: Sequence[str] | None, name: str, *, seed: int,
        blind: Callable[[str], str] | None = None, params: dict | None = None) -> dict:
    """One replay: load the campaign (blinded through `blind`), build its matrix, and run adapter `name` on it with
    `random.Random(seed)`."""
    campaign = replay.load(Path(results), experiments, blind=blind)
    found = matrix(campaign)
    chosen = adapter(name)
    taken = {key: str(value) if isinstance(value, Path) else value for key, value in (params or {}).items()
             if key in chosen.params and value is not None}
    estimates = chosen.estimate(found, random.Random(seed), **taken)
    return {"schema_version": REPLAY_SCHEMA, "adapter": chosen.name, "adapter_version": chosen.version,
            "seed": seed, "params": taken, "experiments": sorted({row.experiment for row in found.rows}),
            "blinded": campaign.blinded, "analysis_commit": report.analysis_commit(),
            "matrix": {"rows": len(found.rows), "sha256": found.sha256()}, "estimates": estimates}


def canonical(doc: object) -> str:
    """`doc` as canonical JSON: sorted keys, fixed separators, floats to `SIGNIFICANT` digits, no NaN."""
    return json.dumps(_rounded(doc), sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)


def export(doc: dict, path: Path) -> Path:
    """Write `doc`'s canonical text and a newline: what S10's replay mode reads."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(canonical(doc) + "\n", encoding="utf-8")
    return path


# ---------------------------------------------------------------- the A/A check


def mean_difference(a: Sequence[float], b: Sequence[float], z: float = Z95) -> tuple[float, float, float]:
    """mean(a) - mean(b) and its normal interval with Welch's standard error (sample variances)."""
    diff = math.fsum(a) / len(a) - math.fsum(b) / len(b)
    se = math.sqrt(_variance(a) / len(a) + _variance(b) / len(b))
    return diff, diff - z * se, diff + z * se


def aa_split(found: OutcomeMatrix, arm: str, rng: random.Random) -> dict:
    """One A/A split of `arm`'s tasks into two pseudo-arms, and the 95% interval of their difference in mean VS."""
    by_task: dict[tuple, list[int]] = {}
    for row in found.of_arm(arm):
        by_task.setdefault(row.task, []).append(row.vs)
    tasks = sorted(by_task)
    if len(tasks) < 4:
        raise ValueError(f"arm {arm} has {len(tasks)} tasks; an A/A split needs at least 4")
    order = list(tasks)
    rng.shuffle(order)
    first = set(order[: len(order) // 2])
    means = {task: math.fsum(values) / len(values) for task, values in by_task.items()}
    a = [means[task] for task in tasks if task in first]
    b = [means[task] for task in tasks if task not in first]
    diff, low, high = mean_difference(a, b)
    return {"tasks": [len(a), len(b)], "diff": diff, "ci95": [low, high], "covers_zero": low <= 0.0 <= high}


def aa_check(found: OutcomeMatrix, rng: random.Random, arm: str | None = None, reps: int = AA_REPS) -> dict:
    """`reps` seeded A/A splits of `arm` (default: the arm with the most kept rows) and how often the interval
    covers 0, against the nominal 95% within its binomial 99% band."""
    if arm is None:
        counts = {label: len(found.of_arm(label)) for label in found.arms()}
        if not counts:
            raise ValueError("the campaign has no rows to split")
        arm = max(sorted(counts), key=lambda label: counts[label])
    splits = [aa_split(found, arm, rng) for _ in range(int(reps))]
    coverage = sum(split["covers_zero"] for split in splits) / len(splits)
    half = Z99 * math.sqrt(AA_NOMINAL * (1 - AA_NOMINAL) / len(splits))
    band = [AA_NOMINAL - half, min(1.0, AA_NOMINAL + half)]
    return {"arm": arm, "reps": len(splits), "nominal": AA_NOMINAL, "coverage": coverage, "band": band,
            "passed": band[0] <= coverage <= band[1], "tasks": splits[0]["tasks"],
            "mean_diff": math.fsum(split["diff"] for split in splits) / len(splits), "first_splits": splits[:3]}


def wilson(successes: int, n: int, z: float = Z95) -> tuple[float, float]:
    """The Wilson score interval of `successes` out of `n` (the house rule, gap-a499aa)."""
    if n == 0:
        return 0.0, 1.0
    p = successes / n
    centre = (p + z * z / (2 * n)) / (1 + z * z / n)
    spread = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    return max(0.0, centre - spread), min(1.0, centre + spread)


def vs_rates(found: OutcomeMatrix, rng: random.Random) -> dict:
    """Each arm's VS rate over its kept rows, with its Wilson interval; draws nothing from `rng`."""
    rates = {}
    for arm in found.arms():
        rows = found.of_arm(arm)
        passed = sum(row.vs for row in rows)
        rates[arm] = {"n": len(rows), "vs": passed, "rate": passed / len(rows) if rows else None,
                      "wilson95": list(wilson(passed, len(rows)))}
    return {"arms": rates}


register(Adapter("aa", "aa-1", aa_check, params=("arm", "reps")))
register(Adapter("vs_rates", "vs-rates-1", vs_rates))


# ---------------------------------------------------------------- the command


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="vb replay", description="Deterministic replays of a logged campaign.",
                                     allow_abbrev=False)
    parser.add_argument("--experiment", action="append", required=True, help="an experiment id (repeatable)")
    parser.add_argument("--results", type=Path, help="default: $VB_RESULTS, then " + str(report.DEFAULT_RESULTS))
    parser.add_argument("--adapter", required=True, help=f"one of {', '.join(sorted({*ADAPTERS, *ADAPTER_MODULES}))}")
    parser.add_argument("--seed", type=int, default=1, help="the replay's seed (default 1)")
    parser.add_argument("--arm", help="aa: the arm label to split (default: the arm with the most rows)")
    parser.add_argument("--reps", type=int,
                        help=f"aa: the number of splits (default {AA_REPS}); h5 and closure (X3): lotteries")
    parser.add_argument("--risk", type=Path, help='h5: M3\'s risk per unit, JSONL {"attempt_key", "r"}')
    parser.add_argument("--lam", type=float, help="h5: the tilt lambda that goes with --risk")
    parser.add_argument("--salt", type=Path, help=f"the blinding salt (default {blind.DEFAULT_SALT})")
    parser.add_argument("--unblinded", action="store_true", help="read arm ids: only once the lock is committed")
    parser.add_argument("--out", type=Path, help="write the replay here as well as to stdout")
    args = parser.parse_args(argv)
    results = Path(args.results or os.environ.get("VB_RESULTS") or report.DEFAULT_RESULTS).expanduser()
    try:
        if args.unblinded:
            lock.require()
            labels = None
        else:
            labels = blind.Blinder.from_file(args.salt or blind.DEFAULT_SALT).label
        doc = run(results, args.experiment, args.adapter, seed=args.seed, blind=labels,
                  params={"arm": args.arm, "reps": args.reps, "risk": args.risk, "lam": args.lam})
    except (blind.BlindError, lock.LockError, replay.ReplayError, KeyError, ValueError, OSError) as err:
        print(f"replay: {err}", file=sys.stderr)
        return 2
    text = canonical(doc)
    if args.out:
        export(doc, args.out)
    print(text)
    return 0


def _variance(values: Sequence[float]) -> float:
    if len(values) < 2:
        return 0.0
    mean = math.fsum(values) / len(values)
    return math.fsum((value - mean) ** 2 for value in values) / (len(values) - 1)


def _rounded(value: object) -> object:
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"a replay output holds {value}")
        return float(f"{value:.{SIGNIFICANT}g}")
    if isinstance(value, dict):
        return {str(key): _rounded(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_rounded(item) for item in value]
    return value


if __name__ == "__main__":
    sys.exit(main())
