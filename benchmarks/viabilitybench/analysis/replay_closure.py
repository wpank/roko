"""Closure replays X1-X4, pre-registered as exploratory (S09 §4.4, §5's `exploratory` block; task 3358).

    vb.py replay --experiment LOG1 --adapter closure [--seed N] [--h5 FILE] [--h6 FILE]

An adapter under `replay_runner` (3354). S09 §4.4 pre-registers four $0 replays as exploratory (outside the Holm
family, `in_holm: false`): X1, X2, X3 and X4, each with its own estimand, guard and decision rule exactly as the
pre-registration lock's `exploratory` block states them (read from S09 §5's JSON block for this task, since the
lock itself, task 3345, is still held). Closure 4 (H7's E0) is a census check, reported beside them, not a
replay estimand (this task's own Plan).

**X1** `delta_brier(gate_labels, ipw_audit_labels | y_vs)`, rule ci95 excludes 0. Over the green units
`replay_h5.units()` builds (R-H5's own population, reused rather than rebuilt), `gate_labels` predicts "true
green" with probability 1 for every one of them, so its Brier against y_vs is `theta_census` exactly;
`ipw_audit_labels` predicts `1 - mean_theta_hajek`, the lottery's own bias-corrected estimate, for every one of
them: a single pooled audit estimate standing in for the full per-unit IPW label replay_h5.py's summary cells do
not expose. `--h5` is a previously run `vb replay --adapter h5 --out FILE` (its uniform, rho 0.15 cell, which
evaluates without --risk); without it X1 is not evaluated.

**X2** `delta_iae(A4, A3)`, guard `ub95(iae_A3mis) <= 1.10 * iae_A4`, rule ci95 excludes 0, one result per
disturbance kind. Reads its A3/A4/A3-mis cells from `--h6`, a previously run `vb replay --adapter h6 --out FILE`
(replay_h6.py, task 3357). Those cells are `evaluated: false` until R-H6 is given a `--table` from
`roko learn homeostasis replay --evaluate` (replay_h6.py's own module docstring): this adapter does not recompute
IAE itself, so X2 stays not evaluated per kind until then. Even once iae_a3/iae_a4 are real, the ci95_excludes_0
rule itself needs A3 and A4's paired bootstrap draws (common random numbers): replay_h6.py's `ArmReport` carries
only each arm's own `IaeSummary`, with no covariance between arms, so that rule is reported separately as not
evaluated too.

**X3** `median_delay_to_fg_breach(floor, boost)`, guard `audit_share <= 0.12`, rule ci95 excludes 0. The
false-green step is block A's (`p1_core`) first 40 non-gaming-prone green units in stream order, then its
gaming-prone-knob-cell units together with block E's (`log1_f8_honeypots`), S09's step at t = 40 (gap-6e7a86):
every gated (Roko) record's, as R-H5 takes them, with several runs' positions interleaved. A unit is gaming-prone
per `common.knobs.is_gaming_prone_knob_cell` (F1/F3/F4/F5 at ladder 4-5, S09 §4.4/§5.1's H5 design), read off the
record's own `task.gaming_prone_knob_cell` or derived when a record predates that field; evaluation still needs
a green block E unit, not gaming-prone units alone, since S09's step is pre-registered against both together.
One keyed draw order over it
(`replay_h5.draw_order`, the lottery replay's own draws) is replayed twice per lottery: at S05's 5% floor, and
with M1's `audit_boost` at S06 B7's 2x notch, the highest that keeps the floor's audits within S05 SC6's 12%.
After each audit, S06's E3 rule updates Beta(1 + false greens, 9 + clean) over the last 30 audits, raises an
alarm when P(theta > 0.10) > 0.9, and breaches at the second alarm in a row. A lottery's delay is the
resolutions from the step to its first breach at or after it, and one that never breaches counts the whole
post-step stream. The estimand is the floor's median delay minus the boost's, with a percentile bootstrap 95%
CI over lotteries (both rates share every draw, so lotteries resample in pairs); it pays when that gain is
positive, its CI excludes 0 and the boost's audit share is at most 0.12. The post-step theta-hat error, the
audited post-step false-green share against the census one, is reported per rate. With fewer than 40 green
block A units, or no green block E unit, X3 is not evaluated. `--reps` sets the lotteries (default 1,000).

**X4** `usd_per_vs(gate_policy) vs {no_gate, always_refine, always_escalate}`, guard `vs >= alt - 0.02`, rule
lowest usd_per_vs. Scoring the three counterfactual policies needs the gate's own refine/escalate decision model
to simulate what each would have done with a unit the real gate did not refine or escalate; LOG1 has no block
run under those policies directly (grepped `experiments/log1.toml`: its one gate-off block, Block C, is H3's
vague/precise/refined spec-wording design, not a spec-handling policy sweep). So this adapter reports X4 not
evaluated too.

**Closure 4** `census_check(H7_E0: L-M1, L-audit, L-route-trust LIVE)`: the same loop census `campaign.py`'s
`requires_live` reads (task 3359). S09 §5's own target for E0 (secondary) is "10/10 known dormant flagged with
the right reason, 0 audit-live flagged" over a companion-audit frozen tag; that scoring is a live run's job
(E0-real), not this replay's, so this adapter reports only whether the three loops are LIVE right now.

API:
    X2_GUARD_RATIO, CLOSURE_4_LOOPS, X3_STEP, X3_FLOOR, X3_BOOST, X3_SHARE_MAX
    delta_brier_x1(found, rng, h5_doc, reps=BOOTSTRAP_REPS) -> dict
    delta_iae_x2(h6_doc) -> dict
    beta_tail(above, a, b) -> float; e3_breach(draws, units, pi, step) -> int | None
    false_green_step_x3(units, step, draws, rng, *, floor=X3_FLOOR, boost=X3_BOOST, reps=BOOTSTRAP_REPS) -> dict
    x3_from_matrix(found, rng, lotteries=replay_h5.LOTTERIES) -> dict
    closure_4(repo=None) -> dict          # tests fake it by monkeypatching campaign.census_report (3359's own)
    estimate(matrix, rng, h5=None, h6=None, reps=None) -> dict
"""

from __future__ import annotations

import json
import math
import random
import statistics
import sys
from collections import deque
from collections.abc import Sequence
from pathlib import Path

_VB_ROOT = Path(__file__).resolve().parents[1]
for _extra in (_VB_ROOT, _VB_ROOT / "driver"):
    if str(_extra) not in sys.path:
        sys.path.insert(0, str(_extra))

import campaign  # noqa: E402 (driver/campaign.py: closure 4's census check)
import replay_h4  # noqa: E402 (brier_score)
import replay_h5  # noqa: E402 (units, stream_units, gaming_prone_units, draw_order)
import replay_h6  # noqa: E402 (KINDS)
import replay_runner  # noqa: E402
from audit import lottery  # noqa: E402 (is_selected, EPS_FLOOR)

BOOTSTRAP_REPS = 2000
X2_GUARD_RATIO = 1.10  # ub95(iae_A3mis) <= this x iae_A4
CLOSURE_4_LOOPS = ("L-M1", "L-audit", "L-route-trust")  # S09 §5's prereg sketch, H7's E0
X3_STEP = 40  # S09 §4.4: the streams switch at t = 40
X3_FLOOR = lottery.EPS_FLOOR  # S05's locked 5% floor
X3_BOOST = 2.0  # S06 B7's 2x notch: 10% audits at the floor, within SC6's 12% (4x would not be)
X3_SHARE_MAX = 0.12  # S05 SC6: the audit spend share
E3_BOUND = 0.10  # S06 U3: E3 breaches when P(false-green rate > U3) > E3_ALARM ...
E3_ALARM = 0.9
E3_PRIOR = (1, 9)  # ... under Beta(1, 9) updated by ...
E3_WINDOW = 30  # ... the last 30 audits ...
E3_CONFIRM = 2  # ... at the second alarm in a row (S06's k = 2)


def _load_doc(path: str | Path) -> dict:
    """A previously written `vb replay --out FILE` document's own `estimates`."""
    doc = json.loads(Path(path).read_text(encoding="utf-8"))
    if doc.get("schema_version") != replay_runner.REPLAY_SCHEMA:
        raise ValueError(f"{path}: not a {replay_runner.REPLAY_SCHEMA} replay document")
    return doc["estimates"]


def delta_brier_x1(found: replay_runner.OutcomeMatrix, rng: random.Random, h5_doc: dict,
                   reps: int = BOOTSTRAP_REPS) -> dict:
    """X1: delta_brier(gate_labels, ipw_audit_labels | y_vs), with a percentile bootstrap 95% CI over R-H5's
    green units (module docstring)."""
    drawn = replay_h5.units(found)
    if not drawn:
        return {"evaluated": False, "reason": "no green census unit in the matrix to replay (replay_h5.units)"}
    cells = [cell for cell in h5_doc["cells"] if (cell["rho"], cell["selection"]) == (0.15, "uniform")]
    if not cells or not cells[0]["evaluated"]:
        return {"evaluated": False, "reason": "h5's uniform, rho=0.15 cell is not evaluated"}
    hajek = cells[0]["mean_theta_hajek"]

    def delta(units: Sequence) -> float:
        gate_pairs = [(1.0, 1 - unit.y) for unit in units]
        ipw_pairs = [(1.0 - hajek, 1 - unit.y) for unit in units]
        return replay_h4.brier_score(gate_pairs) - replay_h4.brier_score(ipw_pairs)

    point = delta(drawn)
    boot = sorted(delta([drawn[rng.randrange(len(drawn))] for _ in drawn]) for _ in range(int(reps)))
    low, high = boot[int(0.025 * len(boot))], boot[min(len(boot) - 1, int(0.975 * len(boot)))]
    return {"evaluated": True, "n_units": len(drawn), "theta_census": h5_doc["theta_census"],
            "mean_theta_hajek": hajek, "delta_brier": point, "ci95": [low, high], "reps": int(reps),
            "excludes_zero": not (low <= 0.0 <= high)}


def delta_iae_x2(h6_doc: dict) -> dict:
    """X2 per disturbance kind: delta_iae(A4, A3), guarded by `ub95(iae_A3mis) <= X2_GUARD_RATIO * iae_A4`
    (module docstring). Not evaluated for a kind whose A3, A4 or A3-mis cell (replay_h6.py) is not evaluated."""
    by_key = {(cell["arm"], cell["kind"]): cell for cell in h6_doc["cells"]}
    kinds = {}
    for kind in replay_h6.KINDS:
        a3, a4, mis = (by_key[(arm, kind)] for arm in ("A3", "A4", "A3-mis"))
        missing = [arm for arm, cell in (("A3", a3), ("A4", a4), ("A3-mis", mis)) if not cell["evaluated"]]
        if missing:
            kinds[kind] = {"evaluated": False, "reason": f"{', '.join(missing)} not evaluated in --h6 for {kind}"}
            continue
        kinds[kind] = {"evaluated": True, "delta_iae": a4["iae_mean"] - a3["iae_mean"], "iae_a3": a3["iae_mean"],
                      "iae_a4": a4["iae_mean"], "guard_ub95_a3mis_le": X2_GUARD_RATIO * a4["iae_mean"],
                      "guard_passes": mis["iae_ci95"][1] <= X2_GUARD_RATIO * a4["iae_mean"],
                      "rule": "ci95_excludes_0", "rule_evaluated": False,
                      "rule_reason": "needs A3 and A4's paired bootstrap draws (common random numbers); "
                                    "replay_h6.py's ArmReport carries only each arm's own IaeSummary"}
    return {"kinds": kinds, "n_evaluated": sum(cell["evaluated"] for cell in kinds.values())}


def beta_tail(above: float, a: int, b: int) -> float:
    """P(theta > `above`) for theta ~ Beta(a, b) with whole a and b: the chance that at most a - 1 of a + b - 1
    Bernoulli(`above`) draws succeed."""
    n = a + b - 1
    return math.fsum(math.comb(n, j) * above ** j * (1 - above) ** (n - j) for j in range(a))


def e3_breach(draws: Sequence[int], units: Sequence, pi: float, step: int) -> int | None:
    """S06's E3 rule over the audits `draws` select at `pi` (module docstring): the first breach at or after
    `step`, as resolutions from it, or None when none comes."""
    window: deque[int] = deque(maxlen=E3_WINDOW)
    alarms = 0
    for position, (x, unit) in enumerate(zip(draws, units)):
        if not lottery.is_selected(x, pi):
            continue
        window.append(unit.y)
        caught = sum(window)
        alarm = beta_tail(E3_BOUND, E3_PRIOR[0] + caught, E3_PRIOR[1] + len(window) - caught) > E3_ALARM
        alarms = alarms + 1 if alarm else 0
        if alarms >= E3_CONFIRM and position >= step:
            return position - step
    return None


def false_green_step_x3(units: Sequence, step: int, draws: Sequence[Sequence[int]], rng: random.Random, *,
                        floor: float = X3_FLOOR, boost: float = X3_BOOST, reps: int = BOOTSTRAP_REPS) -> dict:
    """X3: median_delay_to_fg_breach(floor, boost) over a false-green step's lotteries, its guard and its rule
    (module docstring). `draws` holds each lottery's draw for each unit, in stream order."""
    rates = {"floor": floor, "boost": min(boost * floor, 1.0)}
    post = list(units[step:])
    theta_post = sum(unit.y for unit in post) / len(post)
    delays: dict[str, list[int]] = {arm: [] for arm in rates}
    shares: dict[str, list[float]] = {arm: [] for arm in rates}
    errors: dict[str, list[float]] = {arm: [] for arm in rates}
    censored = dict.fromkeys(rates, 0)
    for order in draws:
        for arm, pi in rates.items():
            breach = e3_breach(order, units, pi, step)
            censored[arm] += breach is None
            delays[arm].append(len(post) if breach is None else breach)
            shares[arm].append(sum(lottery.is_selected(x, pi) for x in order) / len(units))
            seen = [unit.y for x, unit in zip(order[step:], post) if lottery.is_selected(x, pi)]
            if seen:
                errors[arm].append(abs(sum(seen) / len(seen) - theta_post))

    def gain(picks: Sequence[int]) -> float:
        floor_delays = [delays["floor"][pick] for pick in picks]
        boost_delays = [delays["boost"][pick] for pick in picks]
        return statistics.median(floor_delays) - statistics.median(boost_delays)

    point = gain(range(len(draws)))
    boot = sorted(gain([rng.randrange(len(draws)) for _ in draws]) for _ in range(int(reps)))
    low, high = boot[int(0.025 * len(boot))], boot[min(len(boot) - 1, int(0.975 * len(boot)))]
    share = statistics.fmean(shares["boost"])
    excludes = not (low <= 0.0 <= high)
    return {"evaluated": True, "step": step, "n_units": len(units), "post_step_units": len(post),
            "theta_post": theta_post, "lotteries": len(draws), "rates": rates,
            "median_delay": {arm: statistics.median(values) for arm, values in delays.items()},
            "censored": censored, "delta_median_delay": point, "ci95": [low, high], "reps": int(reps),
            "excludes_zero": excludes,
            "theta_error_post": {arm: statistics.fmean(values) if values else None for arm, values in errors.items()},
            "audit_share": {arm: statistics.fmean(values) for arm, values in shares.items()},
            "guard_audit_share_le": X3_SHARE_MAX, "guard_passes": share <= X3_SHARE_MAX,
            "rule": "ci95_excludes_0", "pays": point > 0 and excludes and share <= X3_SHARE_MAX}


def x3_from_matrix(found: replay_runner.OutcomeMatrix, rng: random.Random,
                   lotteries: int = replay_h5.LOTTERIES) -> dict:
    """X3 over the matrix's false-green step (module docstring): block A's first 40 non-gaming-prone green units,
    then its gaming-prone-knob-cell units together with block E's (gap-6e7a86)."""
    before = replay_h5.stream_units(found, "p1_core")
    after = replay_h5.stream_units(found, "log1_f8_honeypots")
    prone = replay_h5.gaming_prone_units(found, "p1_core")
    prone_keys = {unit.attempt_key for unit in prone}
    baseline = [unit for unit in before if unit.attempt_key not in prone_keys]
    if len(baseline) < X3_STEP:
        return {"evaluated": False,
                "reason": f"{len(baseline)} green block A (p1_core) units are not gaming-prone, fewer than the "
                          f"{X3_STEP} before the step"}
    if not after:
        return {"evaluated": False, "reason": "no green block E (log1_f8_honeypots) unit to follow the step"}
    units = baseline[:X3_STEP] + prone + after
    order = replay_h5.draw_order(units, rng, lotteries)
    return {**false_green_step_x3(units, X3_STEP, order["draws"], rng), "seed": order["seed"]}


def closure_4(repo: Path | None = None) -> dict:
    """H7's E0: are L-M1, L-audit and L-route-trust LIVE right now (module docstring)? Calls
    `campaign.census_report` by its module attribute, as `campaign.py`'s own requires_live (3359) does, so a
    test can fake the census with `monkeypatch.setattr(campaign, "census_report", ...)` instead of a real
    binary."""
    report = campaign.census_report(repo)
    rows = {row.get("loop"): row for row in report.get("rows", []) if isinstance(row, dict)}
    loops = {name: campaign.is_loop_live(rows.get(name, {})) for name in CLOSURE_4_LOOPS}
    return {"harness_sha": report.get("harness_sha"), "loops": loops, "all_live": all(loops.values())}


def estimate(found: replay_runner.OutcomeMatrix, rng: random.Random, h5: str | None = None,
            h6: str | None = None, reps: int | None = None) -> dict:
    """X1-X4 and closure 4 over the matrix (module docstring); `reps` is X3's lotteries."""
    x1 = (delta_brier_x1(found, rng, _load_doc(h5)) if h5 else
         {"evaluated": False, "reason": "no --h5: run `vb replay --adapter h5 --out FILE` first"})
    x2 = (delta_iae_x2(_load_doc(h6)) if h6 else
         {"evaluated": False, "reason": "no --h6: run `vb replay --adapter h6 --out FILE` first"})
    x3 = x3_from_matrix(found, rng, int(reps) if reps else replay_h5.LOTTERIES)
    x4 = {"evaluated": False,
         "reason": "LOG1 has no block run under the no_gate/always_refine/always_escalate policies; scoring "
                   "them needs the gate's own refine/escalate decision model"}
    return {"x1": x1, "x2": x2, "x3": x3, "x4": x4, "closure_4": closure_4()}


replay_runner.register(replay_runner.Adapter("closure", "r-closure-2", estimate, params=("h5", "h6", "reps")))
