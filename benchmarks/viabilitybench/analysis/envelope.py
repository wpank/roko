"""H1's envelope: fixed-sequence tests over levels 1-5 (S09 §4.2 and §4.4 H1; D3; task 3338).

For each level j, on the tasks of levels ≤ j that both arms ran,

    R_j = VS rate(`roko_full`) / VS rate(`fd_claude`),   C_j = $/VS(`roko_full`) / $/VS(`fd_claude`),

with paired, family × ℓ stratified bootstrap intervals (`bootstrap.py`, BCa: both are ratios) at H1's local level a,
two-sided (1 − a). Level j is claimed when R_j's lower bound is at least X = 0.90 and C_j's upper bound at most
Y = 0.30 (D3). The levels are tested in order and the chain stops at the first failure: E*, the last level claimed,
is 0 when level 1 fails. Within H1 the fixed sequence needs no further adjustment (§4.2). Where BCa is undefined
because no task's removal changes the ratio (every run of both arms passed, say), the percentile interval stands
in; it is then exact, since every replicate gives the same ratio.

**The p-values Holm needs** (§4.4 H1's *Holm p*): p_R,j = 2 × the one-sided bootstrap p of H0: R_j ≤ X and
p_C,j = 2 × that of H0: C_j ≥ Y, each inverted from its own BCa interval (`bootstrap.p_value`), so p ≤ a exactly
when the (1 − a) bound passes, up to the interpolation between order statistics; p_ℓj = max(p_R,j, p_C,j), the
intersection-union test over the two conditions; and p̃_j = max(p_ℓ1 … p_ℓj), the chain's p for level j. H1 enters
graphical Holm (`holm.py`) with p̃_1, and `e_star_at(a)` reads E* at whatever local level Holm gives H1.

**Rows.** The records are read as `metrics.py` reads them: VS- (an unknown label counts as 0), `infra_error` and
`leak_suspected` runs, the plan-level slice and F8 honeypots left out, censored runs in with VS = 0. Only tasks that
both arms ran count (the design is paired), and every row of a drawn (task, seed) comes along in a replicate. A
rate here is the share of verified runs, pooled over the tasks, which is `metrics.vs_rate`'s mean over tasks when
every task has the same number of runs, as P1's design gives; $/VS is the summed `api_equiv_usd` over the summed VS.
A replicate whose ratio is undefined, an arm with no VS in it, takes the value least favourable to the claim (R = 0,
C = UNDEFINED_C). A level whose ratio is undefined on the data itself, or with a run of unknown cost, cannot be
claimed, and the chain stops there.

**The honest envelope report** (§4.2) also needs each level on its own (`level_ratios`: R and C at level j alone,
T7's `envelope_ratio_*_level`) and the per-level table for every arm (`level_table`: VS rate, $/VS and pass^3 from
`metrics.arm_metrics`). `envelope_metrics` turns both into the MetricRecord names the F3 and T7 scripts read.

API:
    X, Y, LEVELS, ARM, REFERENCE, UNDEFINED_C
    Level(level, n_tasks, n_runs, r, c, r_low, r_high, c_low, c_high, p_r, p_c, p_level, p_chain, claimed, why)
    Envelope(levels, e_star, alpha, arm, reference, b, seed, x, y); .e_star_at(a) -> int
    envelope(records, *, alpha=0.05, arm=ARM, reference=REFERENCE, x=X, y=Y, b=10_000, seed=0, levels=LEVELS)
        -> Envelope
    level_ratios(records, *, alpha=0.05, arm=ARM, reference=REFERENCE, b=10_000, seed=0) -> list[Level]
    level_table(records, experiment_id, *, ks=(3,)) -> list[dict]
    envelope_metrics(result, records, *, alone=()) -> list[metrics.Metric]
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass

import bootstrap
import metrics
from metrics import Clause

X, Y = 0.90, 0.30  # D3: the author's bars for R's lower bound and C's upper bound
LEVELS = (1, 2, 3, 4, 5)
ARM, REFERENCE = "roko_full", "fd_claude"
UNDEFINED_C = 1e9  # a replicate's C when an arm has no VS in it: far above any Y
METHOD = "bca"


@dataclass(frozen=True)
class Level:
    level: int
    n_tasks: int
    n_runs: int
    r: float | None
    c: float | None
    r_low: float | None
    r_high: float | None
    c_low: float | None
    c_high: float | None
    p_r: float  # 1 when R is undefined
    p_c: float
    p_level: float  # max(p_r, p_c)
    p_chain: float  # max of p_level over this level and every one before it
    claimed: bool  # the chain reached this level and both bounds passed at the envelope's alpha
    why: str  # why it was not claimed; "" when it was


@dataclass(frozen=True)
class Envelope:
    levels: tuple[Level, ...]
    e_star: int
    alpha: float
    arm: str
    reference: str
    b: int
    seed: int
    x: float
    y: float

    def e_star_at(self, a: float) -> int:
        """E* at local level `a`: the levels whose chain p is at most `a`, counted from level 1."""
        claimed = 0
        for level in self.levels:
            if level.p_chain > a:
                break
            claimed += 1
        return claimed


def envelope(records: Iterable[dict], *, alpha: float = 0.05, arm: str = ARM, reference: str = REFERENCE,
             x: float = X, y: float = Y, b: int = bootstrap.DEFAULT_B, seed: int = 0,
             levels: Sequence[int] = LEVELS) -> Envelope:
    """The cumulative chain over levels 1-5 at local level `alpha` (module docstring). `levels`, a prefix of 1-5,
    tests only those: under a global null a false claim needs level 1 first, so `simulate.py` tests (1,)."""
    if tuple(levels) != LEVELS[:len(levels)] or not levels:
        raise ValueError(f"levels must be a non-empty prefix of {LEVELS}, not {tuple(levels)}")
    rows, _ = _rows(records, arm, reference)
    tested, running, chain_open = [], 0.0, True
    for level in levels:
        found = _level([row for row in rows if row["task"]["ladder"] <= level], level, alpha, arm, x, y, b,
                       seed + level)
        running = max(running, found.p_level)
        why = found.why or ("" if chain_open else "the chain stopped at an earlier level")
        claimed = chain_open and not found.why
        chain_open = claimed
        tested.append(Level(**{**found.__dict__, "p_chain": running, "claimed": claimed, "why": why}))
    e_star = sum(1 for _ in _leading(tested))
    return Envelope(levels=tuple(tested), e_star=e_star, alpha=alpha, arm=arm, reference=reference, b=b, seed=seed,
                    x=x, y=y)


def level_ratios(records: Iterable[dict], *, alpha: float = 0.05, arm: str = ARM, reference: str = REFERENCE,
                 x: float = X, y: float = Y, b: int = bootstrap.DEFAULT_B, seed: int = 0) -> list[Level]:
    """R and C at each level on its own (not cumulative), for the honest report; never part of the chain."""
    rows, _ = _rows(records, arm, reference)
    out = []
    for level in LEVELS:
        found = _level([row for row in rows if row["task"]["ladder"] == level], level, alpha, arm, x, y, b,
                       seed + 100 + level)
        out.append(Level(**{**found.__dict__, "p_chain": found.p_level, "claimed": not found.why}))
    return out


def level_table(records: Iterable[dict], experiment_id: str, *, ks: Sequence[int] = (3,)) -> list[dict]:
    """Every cell's VS rate, $/VS and pass^k at each level alone, from `metrics.arm_metrics` (§4.2's table)."""
    records = list(records)
    wanted = {"vs_rate", "usd_per_vs", *(f"pass_hat_{k}" for k in ks)}
    table = []
    for arm, model, harness in metrics.cells(records):
        found = metrics.arm_metrics(records, experiment_id, arm, ks=ks, model=model, harness=harness)
        for level in LEVELS:
            values = {metric.metric: metric.value for metric in found
                      if metric.cell == f"l{level}" and metric.metric in wanted}
            if values:
                table.append({"cell": metrics.cell_name(arm, model, harness), "level": level,
                              **{name: values.get(name) for name in sorted(wanted)}})
    return table


def envelope_metrics(result: Envelope, records: Iterable[dict], *, alone: Sequence[Level] = ()) -> list[metrics.Metric]:
    """The MetricRecords' names the F3 and T7 scripts read: `envelope_ratio_r` and `envelope_ratio_c` per level
    (cumulative), `envelope_ratio_r_level` and `envelope_ratio_c_level` (each level alone, from `level_ratios`), and
    `envelope_level`, E* at the envelope's alpha."""
    rows, unpaired = _rows(records, result.arm, result.reference)
    clauses = [Clause("arm", "in", [result.arm, result.reference]), Clause("task.family", "!=", metrics.PLAN_SLICE),
               Clause("task.is_honeypot", "==", False), Clause("execution.status", "not in", list(metrics.EXCLUDED))]
    if unpaired:  # the filter shown is the filter applied: name the tasks both arms ran
        clauses.append(Clause("task.instance_id", "in", sorted({row["task"]["instance_id"] for row in rows})))
    base = metrics.cut(rows, clauses)
    method = f"paired_stratified_bootstrap_{METHOD}"
    out = []
    for cumulative, levels in ((True, result.levels), (False, alone)):
        for level in levels:
            where = base.narrow(Clause("task.ladder", "in", list(range(1, level.level + 1))) if cumulative
                                else Clause("task.ladder", "==", level.level))
            suffix, scope = ("", f"levels <= {level.level}") if cumulative else ("_level", f"level {level.level}")
            for name, value, low, high, cost in (("r", level.r, level.r_low, level.r_high, None),
                                                 ("c", level.c, level.c_low, level.c_high, "api_equiv_usd")):
                if value is None:
                    continue
                estimator = (f"{'VS rate' if name == 'r' else '$/VS'} of {result.arm} over {result.reference}'s on "
                             f"{scope}; paired, family x level stratified BCa bootstrap (B = {result.b}), "
                             f"two-sided {1 - result.alpha:.4g} interval")
                out.append(metrics.Metric(metric=f"envelope_ratio_{name}{suffix}", value=value, n=level.n_tasks,
                                          estimator=estimator, cost_basis=cost, cut=where, cell=f"l{level.level}",
                                          ladder=level.level, ci=(low, high), ci_method=method))
    out.append(metrics.Metric(metric="envelope_level", value=float(result.e_star), n=len(_tasks(rows)),
                              estimator=f"E*: the last level j whose R_j lower bound is >= {result.x} and C_j upper "
                                        f"bound <= {result.y} at alpha {result.alpha:.4g}, tested in order (S09 §4.2)",
                              cost_basis=None, cut=base, cell="all", ladder=None, ci=None, ci_method="none"))
    return out


def _rows(records: Iterable[dict], arm: str, reference: str) -> tuple[list[dict], set[tuple]]:
    """Both arms' included rows on the tasks both ran, each with its VS- and cost precomputed; and the tasks only
    one arm ran, left out."""
    kept = [record for record in records
            if record["arm"] in (arm, reference) and record["task"]["family"] != metrics.PLAN_SLICE
            and not record["task"].get("is_honeypot") and record["execution"]["status"] not in metrics.EXCLUDED]
    ran = {name: {metrics.task_key(row) for row in kept if row["arm"] == name} for name in (arm, reference)}
    paired = ran[arm] & ran[reference]
    rows = [{**row, "_vs": metrics.vs_minus(row), "_usd": row["costs"]["api_equiv_usd"]}
            for row in kept if metrics.task_key(row) in paired]
    return rows, ran[arm] ^ ran[reference]


def _level(rows: list[dict], level: int, alpha: float, arm: str, x: float, y: float, b: int, seed: int) -> Level:
    """One level's ratios, bounds and p-values over `rows`; `why` says what keeps it from being claimed on its own."""
    tasks = _tasks(rows)
    empty = Level(level=level, n_tasks=len(tasks), n_runs=len(rows), r=None, c=None, r_low=None, r_high=None,
                  c_low=None, c_high=None, p_r=1.0, p_c=1.0, p_level=1.0, p_chain=1.0, claimed=False, why="")
    if not rows:
        return Level(**{**empty.__dict__, "why": "no task both arms ran at these levels"})
    if any(row["_usd"] is None for row in rows):
        return Level(**{**empty.__dict__, "why": "a run's cost is unknown, so C is unmeasurable"})

    def r_of(sample: Sequence[dict]) -> float:
        mine, theirs = _sums(sample, arm)
        if not mine[1] or not theirs[1] or not theirs[0]:
            return 0.0
        return (mine[0] / mine[1]) / (theirs[0] / theirs[1])

    def c_of(sample: Sequence[dict]) -> float:
        mine, theirs = _sums(sample, arm)
        if not mine[0] or not theirs[0]:
            return UNDEFINED_C
        return (mine[2] / mine[0]) / (theirs[2] / theirs[0])

    mine, theirs = _sums(rows, arm)
    if not theirs[0] or not mine[0]:
        side = "the reference" if not theirs[0] else arm
        return Level(**{**empty.__dict__, "why": f"{side} has no VS at these levels, so R or C is undefined"})
    r_result, c_result = (_interval(rows, statistic, b, seed, alpha) for statistic in (r_of, c_of))
    p_r, p_c = bootstrap.p_value(r_result, x, "greater"), bootstrap.p_value(c_result, y, "less")
    failed = []
    if r_result.low < x:
        failed.append(f"R's lower bound {r_result.low:.4f} < {x}")
    if c_result.high > y:
        failed.append(f"C's upper bound {c_result.high:.4f} > {y}")
    return Level(level=level, n_tasks=len(tasks), n_runs=len(rows), r=r_result.estimate, c=c_result.estimate,
                 r_low=r_result.low, r_high=r_result.high, c_low=c_result.low, c_high=c_result.high, p_r=p_r, p_c=p_c,
                 p_level=max(p_r, p_c), p_chain=max(p_r, p_c), claimed=False, why="; ".join(failed))


def _interval(rows: list[dict], statistic, b: int, seed: int, alpha: float) -> bootstrap.BootstrapResult:
    """BCa, or the percentile interval where BCa's jackknife cannot tell the tasks apart (module docstring)."""
    try:
        return bootstrap.paired_bootstrap(rows, statistic, b=b, seed=seed, alpha=alpha, method=METHOD)
    except bootstrap.BootstrapError:
        return bootstrap.paired_bootstrap(rows, statistic, b=b, seed=seed, alpha=alpha, method="percentile")


def _sums(rows: Sequence[dict], arm: str) -> tuple[list[float], list[float]]:
    """[VS, runs, cost] for the arm and for the reference, over `rows` (a replicate's or the data's)."""
    mine, theirs = [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]
    for row in rows:
        sums = mine if row["arm"] == arm else theirs
        sums[0] += row["_vs"]
        sums[1] += 1
        sums[2] += row["_usd"] or 0.0
    return mine, theirs


def _tasks(rows: Iterable[dict]) -> set[tuple]:
    return {metrics.task_key(row) for row in rows}


def _leading(levels: Sequence[Level]) -> Iterable[Level]:
    for level in levels:
        if not level.claimed:
            return
        yield level
