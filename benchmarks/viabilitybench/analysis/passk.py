"""pass^k: how likely k independent runs of a task are all verified successes (paper App. D.3; S09 §4.1).

For task j with n_j independent runs and c_j verified successes, C(c_j, k) / C(n_j, k) is unbiased for p_j^k when
the runs are i.i.d. with success probability p_j (yao2024taubench). The estimate is the plain mean over tasks:

    pass^k = (1/J) * sum_j C(c_j, k) / C(n_j, k)

It runs over the J tasks with n_j >= k. A task with fewer runs has no pass^k, so it is skipped and listed, never
padded. Only independent runs count, so the caller passes one label per distinct seed, never a resumed or retried
run. The sum is exact (`fractions.Fraction`), so the float is the correctly rounded mean whatever the task order.
`stratified` splits the tasks by stratum (family x level in S09 §4.1) and estimates each stratum on its own.

API:
    task_pass_k(successes: int, runs: int, k: int) -> Fraction
    PassK(k, value, tasks, skipped)          # value is None when no task has k runs
    pass_k(labels: Mapping[T, Sequence[int]], k: int) -> PassK
    stratified(labels: Mapping[T, Sequence[int]], strata: Mapping[T, S], k: int) -> dict[S, PassK]
"""

from __future__ import annotations

import math
from collections.abc import Hashable, Mapping, Sequence
from dataclasses import dataclass
from fractions import Fraction


@dataclass(frozen=True)
class PassK:
    k: int
    value: float | None  # None when no task has k runs
    tasks: tuple  # the tasks averaged over, sorted
    skipped: tuple  # the tasks with fewer than k runs, sorted


def task_pass_k(successes: int, runs: int, k: int) -> Fraction:
    """C(successes, k) / C(runs, k), one task's unbiased estimate of p^k."""
    if k < 1:
        raise ValueError(f"k must be at least 1, not {k}")
    if not 0 <= successes <= runs:
        raise ValueError(f"successes must lie between 0 and runs ({runs}), not {successes}")
    if runs < k:
        raise ValueError(f"a task with {runs} run(s) has no pass^{k}")
    return Fraction(math.comb(successes, k), math.comb(runs, k))


def pass_k(labels: Mapping[Hashable, Sequence[int]], k: int) -> PassK:
    """The mean over tasks of `task_pass_k`. `labels` maps each task to its runs' labels, 0 or 1."""
    terms, used, skipped = [], [], []
    for task in sorted(labels):
        runs = labels[task]
        if any(label not in (0, 1) for label in runs):
            raise ValueError(f"task {task!r}: labels must be 0 or 1, not {list(runs)!r}")
        if len(runs) < k:
            skipped.append(task)
            continue
        terms.append(task_pass_k(sum(runs), len(runs), k))
        used.append(task)
    value = float(sum(terms, Fraction(0)) / len(terms)) if terms else None
    return PassK(k=k, value=value, tasks=tuple(used), skipped=tuple(skipped))


def stratified(labels: Mapping[Hashable, Sequence[int]], strata: Mapping[Hashable, Hashable], k: int) -> dict:
    """pass^k within each stratum, keyed by stratum in sorted order."""
    groups: dict = {}
    for task, runs in labels.items():
        groups.setdefault(strata[task], {})[task] = runs
    return {stratum: pass_k(groups[stratum], k) for stratum in sorted(groups)}
