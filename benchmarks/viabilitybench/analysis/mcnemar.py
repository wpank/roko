"""McNemar's exact test on paired binary outcomes, S09 §4.1's robustness check (`mcnemar1947note`; task 3335).

Two arms run the same tasks. For each task, take one outcome per arm, VS on seed 1 (`seed_pairs`), so the pairs are
independent across tasks. Only the discordant pairs carry information about a difference: b tasks where the first
arm succeeded and the second failed, c the other way round. Under H0 (the arms succeed equally often) each
discordant pair goes either way with probability 1/2, so b ~ Binomial(b + c, 1/2) and the exact two-sided p-value is

    p = min(1, 2 · P(X ≤ min(b, c))),  X ~ Binomial(b + c, 1/2);   p = 1 when b + c = 0.

The tail is summed exactly (`fractions.Fraction`), so the float is the correctly rounded p whatever b + c is.
The estimate beside it is the paired difference in success rates, (b − c) / n over the n pairs.

`seed_pairs` reads run records as `metrics.py` does: the task is (instance id, spec variant), the label VS- (an
unknown label counts as 0), `infra_error` and `leak_suspected` runs are left out, and so are tasks one arm did not
run on that seed (listed, never padded).

API:
    McNemar(b, c, n, p, difference)
    exact_p(b, c) -> float
    compare(pairs: Iterable[(int, int)]) -> McNemar
    seed_pairs(records, first_arm, second_arm, *, seed=1) -> (pairs: dict[task, (int, int)], missing: list[task])
"""

from __future__ import annotations

import math
from collections.abc import Iterable
from dataclasses import dataclass
from fractions import Fraction

import metrics


@dataclass(frozen=True)
class McNemar:
    b: int  # pairs where the first arm succeeded and the second failed
    c: int  # pairs where the second arm succeeded and the first failed
    n: int  # every pair, concordant ones included
    p: float  # exact two-sided p-value
    difference: float | None  # (b − c) / n, the first arm's rate minus the second's; None without pairs


def exact_p(b: int, c: int) -> float:
    """The exact two-sided McNemar p-value for b and c discordant pairs (module docstring)."""
    if b < 0 or c < 0:
        raise ValueError(f"discordant counts are at least 0, not b={b}, c={c}")
    total = b + c
    if total == 0:
        return 1.0
    tail = sum((math.comb(total, k) for k in range(min(b, c) + 1)), 0)
    return float(min(Fraction(1), Fraction(2 * tail, 2 ** total)))


def compare(pairs: Iterable[tuple[int, int]]) -> McNemar:
    """McNemar's exact test on (first arm's outcome, second arm's outcome) pairs, each 0 or 1."""
    pairs = list(pairs)
    for index, pair in enumerate(pairs):
        if len(pair) != 2 or any(isinstance(value, bool) or value not in (0, 1) for value in pair):
            raise ValueError(f"pair {index} is not two outcomes of 0 or 1: {pair!r}")
    b = sum(1 for first, second in pairs if first == 1 and second == 0)
    c = sum(1 for first, second in pairs if first == 0 and second == 1)
    difference = (b - c) / len(pairs) if pairs else None
    return McNemar(b=b, c=c, n=len(pairs), p=exact_p(b, c), difference=difference)


def seed_pairs(records: Iterable[dict], first_arm: str, second_arm: str, *,
               seed: int = 1) -> tuple[dict[tuple, tuple[int, int]], list[tuple]]:
    """Per task, the two arms' VS- on `seed`, and the tasks only one arm ran on it, both in task order."""
    labels: dict[str, dict[tuple, int]] = {first_arm: {}, second_arm: {}}
    for record in records:
        arm = record["arm"]
        if (arm not in labels or record["seed"] != seed or record["task"]["family"] == metrics.PLAN_SLICE
                or record["execution"]["status"] in metrics.EXCLUDED):
            continue
        key = metrics.task_key(record)
        if key in labels[arm]:
            raise metrics.MetricsError(f"arm {arm} ran {key[0]} ({key[1]}) twice on seed {seed}; McNemar needs "
                                       "one outcome per task and arm")
        labels[arm][key] = metrics.vs_minus(record)
    first, second = labels[first_arm], labels[second_arm]
    pairs = {task: (first[task], second[task]) for task in sorted(first.keys() & second.keys())}
    missing = sorted(first.keys() ^ second.keys())
    return pairs, missing
