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
(replay_h6.py, task 3357). Every one of those cells is `evaluated: false` until 8117's evaluator has a CLI entry
point (replay_h6.py's own module docstring): this adapter does not recompute IAE itself, so X2 stays not
evaluated per kind until then. Even once iae_a3/iae_a4 are real, the ci95_excludes_0 rule itself needs A3 and
A4's paired bootstrap draws (common random numbers): replay_h6.py's `ArmReport` carries only each arm's own
`IaeSummary`, with no covariance between arms, so that rule is reported separately as not evaluated too.

**X3** `median_delay_to_fg_breach(floor, boost)`, guard `audit_share <= 0.12`, rule ci95 excludes 0. Needs the
lottery replay's own per-position draw order, to find when a false-green run would first have been caught under
the fixed (floor) and adaptive (boost) audit rate. replay_h5.py's `estimate()` returns only each cell's
aggregate coverage/bias/theta_ratio (module docstring there), not that per-position sequence, so this adapter
reports X3 not evaluated until a replay exposes it.

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
    X2_GUARD_RATIO, CLOSURE_4_LOOPS
    delta_brier_x1(found, rng, h5_doc, reps=BOOTSTRAP_REPS) -> dict
    delta_iae_x2(h6_doc) -> dict
    closure_4(repo=None) -> dict          # tests fake it by monkeypatching campaign.census_report (3359's own)
    estimate(matrix, rng, h5=None, h6=None) -> dict
"""

from __future__ import annotations

import json
import random
import sys
from collections.abc import Sequence
from pathlib import Path

_VB_ROOT = Path(__file__).resolve().parents[1]
for _extra in (_VB_ROOT, _VB_ROOT / "driver"):
    if str(_extra) not in sys.path:
        sys.path.insert(0, str(_extra))

import campaign  # noqa: E402 (driver/campaign.py: closure 4's census check)
import replay_h4  # noqa: E402 (brier_score)
import replay_h5  # noqa: E402 (units)
import replay_h6  # noqa: E402 (KINDS)
import replay_runner  # noqa: E402

BOOTSTRAP_REPS = 2000
X2_GUARD_RATIO = 1.10  # ub95(iae_A3mis) <= this x iae_A4
CLOSURE_4_LOOPS = ("L-M1", "L-audit", "L-route-trust")  # S09 §5's prereg sketch, H7's E0


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
            h6: str | None = None) -> dict:
    """X1-X4 and closure 4 over the matrix (module docstring)."""
    x1 = (delta_brier_x1(found, rng, _load_doc(h5)) if h5 else
         {"evaluated": False, "reason": "no --h5: run `vb replay --adapter h5 --out FILE` first"})
    x2 = (delta_iae_x2(_load_doc(h6)) if h6 else
         {"evaluated": False, "reason": "no --h6: run `vb replay --adapter h6 --out FILE` first"})
    x3 = {"evaluated": False,
         "reason": "replay_h5.py's cells carry only aggregate coverage/bias/theta_ratio, not the per-position "
                   "lottery draw order median_delay_to_fg_breach needs"}
    x4 = {"evaluated": False,
         "reason": "LOG1 has no block run under the no_gate/always_refine/always_escalate policies; scoring "
                   "them needs the gate's own refine/escalate decision model"}
    return {"x1": x1, "x2": x2, "x3": x3, "x4": x4, "closure_4": closure_4()}


replay_runner.register(replay_runner.Adapter("closure", "r-closure-1", estimate, params=("h5", "h6")))
