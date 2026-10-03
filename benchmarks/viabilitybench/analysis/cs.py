"""Always-valid confidence sequences for bounded outcomes (S09 §4.1's toolkit; task 3335).

A confidence sequence (CS) is a sequence of intervals C_1, C_2, ... that covers the mean at every time at once:
P(μ ∈ C_t for every t) ≥ 1 − α (`howard2021timeuniform`). It may be read after every task, and a campaign may stop
whenever it shows something, without losing coverage; a fixed-n interval re-checked after every task does lose it.
S09 uses it for live monitoring and for the stopping rules of H1 and H5-H7.

**The construction** is Waudby-Smith and Ramdas's hedged capital process with predictable plug-in bets (2024, JRSS-B,
Theorem 3; `waudbysmith2024estimating`), the one `audit/estimate.py` already implements for S05's live sequence. This
module builds on that function (`hedged_capital`) rather than repeating it, so S05 and S09 share one implementation
and one reference fixture for the Rust port. For values x_t in [lo, hi], rescaled to [0, 1]:
- K_t(m), the hedged capital against "the mean is m", is a nonnegative martingale when the mean is m, so the chance
  that it ever reaches 1/α is at most α (Ville's inequality).
- The CS at time t is the running intersection {m : K_s(m) < 1/α for every s ≤ t}, on a grid of `grid` steps over
  [lo, hi]. Each interval is the hull of the grid points still inside, widened by one step on each side (clipped to
  [lo, hi]), so it contains the exact running intersection; None once no grid point is left.
- The anytime-valid p-value for H0: μ = m is p_t = min(1, 1 / max_{s ≤ t} K_s(m)), S09 H7's "1 / sup_t E_t". It may
  be read at any stopping time: P(p_τ ≤ α) ≤ α however τ is chosen.

**Contrasts.** A paired contrast is a CS on per-task differences: `paired_contrast` turns two arms' outcomes on the
same tasks, x_t and y_t, into d_t = x_t − w·y_t with its bounds, for example H1's stop "`roko_full`'s VS rate below
half of `fd_claude`'s" (w = 0.5: stop when the CS lies below 0).

The coverage holds for any stopping rule as long as the outcomes share the mean m given the past; a drifting mean is
covered on average over time (`waudbysmith2024estimating`, §5). `test_toolkit.py` checks anytime coverage under an
adversarial stopping rule.

API:
    GRID
    confidence_sequence(values, alpha=0.05, *, lo=0.0, hi=1.0, grid=GRID) -> list[(low, high) | None]
    anytime_p(values, null, *, lo=0.0, hi=1.0, alpha=0.05) -> list[float]     # p_t after each value
    covers(values, mean, alpha=0.05, *, lo=0.0, hi=1.0) -> bool               # the CS covers `mean` at every time
    paired_contrast(first, second, *, weight=1.0, lo=0.0, hi=1.0) -> (values, lo, hi)
"""

from __future__ import annotations

import sys
from collections.abc import Sequence
from pathlib import Path

VB_ROOT = Path(__file__).resolve().parents[1]
if str(VB_ROOT) not in sys.path:
    sys.path.insert(0, str(VB_ROOT))
from audit.estimate import hedged_capital  # noqa: E402  (S05's process: one implementation, one fixture)

GRID = 1000


def confidence_sequence(values: Sequence[float], alpha: float = 0.05, *, lo: float = 0.0, hi: float = 1.0,
                        grid: int = GRID) -> list[tuple[float, float] | None]:
    """The CS for the mean of `values` in [lo, hi] after each value (module docstring)."""
    xs = _scaled(values, lo, hi)
    if isinstance(grid, bool) or not isinstance(grid, int) or grid < 1:
        raise ValueError(f"grid is a positive integer, not {grid!r}")
    threshold = 1.0 / alpha
    left = [len(xs)] * (grid + 1)  # the index of the first value at which grid point j leaves the sequence
    for j in range(grid + 1):
        for t, capital in enumerate(hedged_capital(xs, j / grid, alpha)):
            if capital >= threshold:
                left[j] = t
                break
    out: list[tuple[float, float] | None] = []
    for t in range(len(xs)):
        kept = [j for j in range(grid + 1) if left[j] > t]
        if not kept:
            out.append(None)
            continue
        low, high = max(0, kept[0] - 1) / grid, min(grid, kept[-1] + 1) / grid
        out.append((lo + low * (hi - lo), lo + high * (hi - lo)))
    return out


def anytime_p(values: Sequence[float], null: float, *, lo: float = 0.0, hi: float = 1.0,
              alpha: float = 0.05) -> list[float]:
    """p_t = min(1, 1 / max_{s ≤ t} K_s(null)) after each value; `alpha` tunes the bets, any level may read p."""
    if not lo <= null <= hi:
        raise ValueError(f"the null mean {null} is outside [{lo}, {hi}]")
    out, peak = [], 1.0
    for capital in hedged_capital(_scaled(values, lo, hi), (null - lo) / (hi - lo), alpha):
        peak = max(peak, capital)
        out.append(min(1.0, 1.0 / peak))
    return out


def covers(values: Sequence[float], mean: float, alpha: float = 0.05, *, lo: float = 0.0, hi: float = 1.0) -> bool:
    """Whether the CS covers `mean` at every time: K_t(mean) stays below 1/α throughout (no grid involved)."""
    if not lo <= mean <= hi:
        raise ValueError(f"the mean {mean} is outside [{lo}, {hi}]")
    threshold = 1.0 / alpha
    return all(capital < threshold for capital in hedged_capital(_scaled(values, lo, hi), (mean - lo) / (hi - lo),
                                                                 alpha))


def paired_contrast(first: Sequence[float], second: Sequence[float], *, weight: float = 1.0, lo: float = 0.0,
                    hi: float = 1.0) -> tuple[list[float], float, float]:
    """d_t = first_t − weight·second_t for two arms' outcomes on the same tasks, in order, with the bounds of d
    when each outcome lies in [lo, hi] (weight ≥ 0)."""
    if len(first) != len(second):
        raise ValueError(f"a paired contrast needs one outcome per task from each arm, not {len(first)} and "
                         f"{len(second)}")
    if weight < 0:
        raise ValueError(f"weight is at least 0, not {weight}")
    return [x - weight * y for x, y in zip(first, second)], lo - weight * hi, hi - weight * lo


def _scaled(values: Sequence[float], lo: float, hi: float) -> list[float]:
    if not hi > lo:
        raise ValueError(f"the bounds need lo < hi, not [{lo}, {hi}]")
    out = []
    for index, value in enumerate(values):
        if not lo <= value <= hi:
            raise ValueError(f"value {index} ({value}) is outside [{lo}, {hi}]")
        out.append((value - lo) / (hi - lo))
    return out
