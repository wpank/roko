"""A window's false-green estimates from its audited units (S05 §4.5, at window close), and the live sequence.

For a window of N green units, s the audited (selected) ones, π_i their inclusion probabilities, w_i = 1/π_i and
Y_i ∈ {0, 1} their labels (1 = false green; γ̂ and ω̂ take the gaming and weak-oracle labels the same way):

    θ̂_HT  = (1/N) Σ_s Y_i/π_i                       Horvitz–Thompson: unbiased, since every π_i ≥ ε_floor > 0
    θ̂_H   = Σ_s w_i Y_i / N̂,   N̂ = Σ_s w_i          Hájek: the reported estimate
    v̂     = Σ_s (1 − π_i) w_i² (Y_i − θ̂_H)² / N̂²    its linearized variance under Poisson sampling
    n_eff = N̂² / Σ_s w_i²                           Kish's effective sample size
    CI    = Wald on v̂ when n_eff ≥ 30 with at least 5 events (and 5 non-events), else Wilson at n_eff

- **The interval** (`interval`). Wald is θ̂_H ± z·√v̂, Wilson the score interval for θ̂_H at the (fractional)
  n_eff; both are clipped to [0, 1], with z = Φ⁻¹(1 − α/2), `Z95` at α = 0.05. S05 names the event count; the
  non-event count is the same rule for the other tail, where θ̂_H = 1 would give Wald zero width. An empty sample
  has a Horvitz–Thompson estimate of 0, no Hájek estimate, and the vacuous interval [0, 1].
- **How far to trust Wald.** In 20,000 simulated lotteries per cell (60 to 400 units, uniform and M3-tilted
  selection; 2026-10-02), the Wald branch, where it applied (ρ = 0.30 at 200 units, ρ = 0.15 at 400), covered as
  little as 0.93 under a uniform lottery and 0.91 under a strong tilt, while Wilson at n_eff covered at least 0.957
  in every cell. The 60-unit first slice at every ρ, SC1's primary cell (ρ = 0.15 tilted: n_eff about 13 over 120
  units, 22 over 200) and 200-unit windows at ρ = 0.10 almost always stay below n_eff = 30, where the rule is Wilson.
- **Null labels** (a battery error or timeout): `null_bounds` gives the estimate with every missing label read as 0,
  and again as 1.
- **The live sequence** (`betting_cs`). For every green unit in stream order, audited or not,
  Z_i = ε_floor·S_i·Y_i/π_i ∈ [0, 1] (`audit_z`). When each unit is a false green with the same probability θ given
  the past, every Z_i has mean ε_floor·θ given the past, however π_i was chosen. The hedged capital process of
  Waudby-Smith and Ramdas (2024, JRSS-B, Theorem 3), with their predictable plug-in bets, c = 1/2 and equal hedge
  weights, then gives a confidence sequence for θ: with probability at least 1 − α it covers θ at every time at
  once, so at any stopping time, however often it is read (`hedged_capital`, `sequence_holds`). `z_max` is a known
  bound on every Z_i, 1 by default; when every π_i ≥ π_min it may be ε_floor/π_min, and the process runs on
  Z_i/z_max ∈ [0, 1]. The sequence is wide, since most Z_i are 0 and the floor scales the rest down, and it assumes
  that stationary stream; a window's estimate of its own census rate is the Hájek interval.

The arithmetic is the reference for roko-gate's `audit` module (7114), through `fixtures/estimators.json`: sums are
`math.fsum`, and the betting process uses only +, −, ×, ÷, √ and the logarithm in its bets.

API:
    Z95; z_value(alpha) -> float
    Estimate(n_green, n_audited, events, n_hat, theta_ht, theta_hajek, variance, n_eff, ci, ci_method, alpha)
        .record() -> dict
    estimate(audited: Iterable[(pi, y)], n_green: int, alpha=0.05) -> Estimate
    null_bounds(audited: Iterable[(pi, y | None)], n_green, alpha=0.05) -> (Estimate missing = 0, Estimate missing = 1)
    interval(theta, variance, n_eff, events, nonevents, z) -> ((low, high), "wald" | "wilson_eff")
    wilson(p, n, z=Z95) -> (low, high)
    audit_z(selected: bool, y: int | None, pi: float) -> float
    hedged_capital(xs: Sequence[float], m: float, alpha=0.05) -> list[float]      # K_t^±(m) after each x_t
    betting_cs(z: Sequence[float], alpha=0.05, *, z_max=1.0, grid=1000) -> list[(low, high) | None]
    sequence_holds(z: Sequence[float], theta: float, alpha=0.05, *, z_max=1.0) -> bool
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Sequence
from dataclasses import asdict, dataclass
from statistics import NormalDist

from audit.lottery import EPS_FLOOR

Z95 = 1.959963984540054  # Φ⁻¹(0.975), as scipy prints it; Rust hardcodes the same double
WALD_MIN_N_EFF = 30.0
WALD_MIN_EVENTS = 5
CS_C = 0.5  # the bet truncation c of Waudby-Smith and Ramdas: a factor never drops below 1 − c
CS_HEDGE = 0.5  # weight of the upward process; the downward one gets the rest
CS_GRID = 1000


def z_value(alpha: float) -> float:
    """z = Φ⁻¹(1 − α/2); exactly `Z95` at α = 0.05."""
    if not 0.0 < alpha < 1.0:
        raise ValueError(f"alpha is in (0, 1), not {alpha}")
    return Z95 if alpha == 0.05 else NormalDist().inv_cdf(1.0 - alpha / 2.0)


@dataclass(frozen=True)
class Estimate:
    """One window's estimate (S05 §5 `audit.estimate` names: theta_ht, theta_hajek, n_eff, ci, ci_method)."""

    n_green: int
    n_audited: int
    events: int
    n_hat: float
    theta_ht: float
    theta_hajek: float | None
    variance: float | None
    n_eff: float
    ci: tuple[float, float]
    ci_method: str
    alpha: float

    def record(self) -> dict:
        out = asdict(self)
        out["ci"] = list(self.ci)
        return out


def estimate(audited: Iterable[tuple[float, int]], n_green: int, alpha: float = 0.05) -> Estimate:
    """HT, Hájek, v̂, n_eff and the interval from the audited units' (π, Y), for a window of `n_green` green units."""
    units = [_unit(pi, y) for pi, y in audited]
    if isinstance(n_green, bool) or not isinstance(n_green, int) or n_green < max(1, len(units)):
        raise ValueError(f"a window of {n_green!r} green units cannot hold {len(units)} audited ones")
    z = z_value(alpha)
    events = sum(y for _, y in units)
    if not units:
        return Estimate(n_green, 0, 0, 0.0, 0.0, None, None, 0.0, (0.0, 1.0), "wilson_eff", alpha)
    n_hat = math.fsum(1.0 / pi for pi, _ in units)
    weighted = math.fsum(y / pi for pi, y in units)
    theta_hajek = weighted / n_hat
    variance = math.fsum((1.0 - pi) * (y - theta_hajek) ** 2 / pi ** 2 for pi, y in units) / n_hat ** 2
    n_eff = n_hat ** 2 / math.fsum(1.0 / pi ** 2 for pi, _ in units)
    ci, method = interval(theta_hajek, variance, n_eff, events, len(units) - events, z)
    return Estimate(n_green, len(units), events, n_hat, weighted / n_green, theta_hajek, variance, n_eff, ci, method,
                    alpha)


def null_bounds(audited: Iterable[tuple[float, int | None]], n_green: int,
                alpha: float = 0.05) -> tuple[Estimate, Estimate]:
    """The estimate with every null label read as 0, and again as 1 (S05 §4.5)."""
    units = list(audited)
    low = estimate([(pi, 0 if y is None else y) for pi, y in units], n_green, alpha)
    high = estimate([(pi, 1 if y is None else y) for pi, y in units], n_green, alpha)
    return low, high


def interval(theta: float, variance: float, n_eff: float, events: int, nonevents: int,
             z: float) -> tuple[tuple[float, float], str]:
    """Wald on the variance when n_eff ≥ 30 with at least 5 events and 5 non-events, else Wilson at n_eff."""
    if n_eff >= WALD_MIN_N_EFF and events >= WALD_MIN_EVENTS and nonevents >= WALD_MIN_EVENTS:
        half = z * math.sqrt(variance)
        return (max(0.0, theta - half), min(1.0, theta + half)), "wald"
    return wilson(theta, n_eff, z), "wilson_eff"


def wilson(p: float, n: float, z: float = Z95) -> tuple[float, float]:
    """The Wilson score interval for a proportion `p` at a (possibly fractional) sample size `n`; [0, 1] at n = 0."""
    if not 0.0 <= p <= 1.0:
        raise ValueError(f"a proportion is in [0, 1], not {p}")
    if n <= 0.0:
        return 0.0, 1.0
    z2 = z * z
    denom = 1.0 + z2 / n
    center = (p + z2 / (2.0 * n)) / denom
    half = z * math.sqrt(p * (1.0 - p) / n + z2 / (4.0 * n * n)) / denom
    low = 0.0 if p == 0.0 else max(0.0, center - half)  # exactly 0 and 1 at the ends, whatever the rounding
    high = 1.0 if p == 1.0 else min(1.0, center + half)
    return low, high


def audit_z(selected: bool, y: int | None, pi: float) -> float:
    """Z_i = ε_floor·S_i·Y_i/π_i for one green unit; Y may be None only for a unit the lottery did not select."""
    if not EPS_FLOOR <= pi <= 1.0:
        raise ValueError(f"pi is in [{EPS_FLOOR}, 1], not {pi}")
    if not selected:
        return 0.0
    if isinstance(y, bool) or y not in (0, 1):
        raise ValueError(f"an audited unit needs a label of 0 or 1, not {y!r}")
    return EPS_FLOOR * y / pi


def hedged_capital(xs: Sequence[float], m: float, alpha: float = 0.05) -> list[float]:
    """K_t^±(m) = max(½·K_t^+(m), ½·K_t^−(m)) after each x_t ∈ [0, 1]: a test of mean m (WSR 2024, Theorem 3).

    K^+ bets λ_t^+ = min(λ̇_t, c/m) that the mean is above m, K^− bets λ_t^− = min(λ̇_t, c/(1 − m)) that it is below,
    with the predictable plug-in λ̇_t = √(2·log(2/α) / (σ̂²_{t−1}·t·log(t + 1))), where σ̂²_{t−1}·t =
    1/4 + Σ_{i<t} (x_i − μ̂_i)² and μ̂_i = (1/2 + Σ_{j≤i} x_j)/(i + 1). Under the null, both are nonnegative
    martingales, so the chance that K^± ever reaches 1/α is at most α (Ville).
    """
    if not 0.0 <= m <= 1.0:
        raise ValueError(f"a mean of values in [0, 1] is in [0, 1], not {m}")
    process = _Capital(alpha)
    out = []
    up = down = 1.0
    for x in map(_unit_interval, xs):
        bet = process.bet()
        up *= 1.0 + _up_bet(bet, m) * (x - m)
        down *= 1.0 - _down_bet(bet, m) * (x - m)
        out.append(max(CS_HEDGE * up, (1.0 - CS_HEDGE) * down))
        process.observe(x)
    return out


def betting_cs(z: Sequence[float], alpha: float = 0.05, *, z_max: float = 1.0,
               grid: int = CS_GRID) -> list[tuple[float, float] | None]:
    """The confidence sequence for θ after each unit: the running intersection of {θ : K_t^±(ε_floor·θ/z_max) < 1/α}.

    θ runs over a grid of step 1/`grid` on [0, 1]; once a grid point's capital reaches 1/α it leaves the sequence for
    good. Each entry is the hull of the points still inside, widened by one step on each side (clipped to [0, 1]), so
    it contains the exact running intersection; None when no grid point is left.
    """
    xs = _scaled(z, z_max)
    if isinstance(grid, bool) or not isinstance(grid, int) or grid < 1:
        raise ValueError(f"grid is a positive integer, not {grid!r}")
    means = [EPS_FLOOR * (j / grid) / z_max for j in range(grid + 1)]
    up, down, inside = [1.0] * (grid + 1), [1.0] * (grid + 1), [True] * (grid + 1)
    threshold = 1.0 / alpha
    process = _Capital(alpha)
    out: list[tuple[float, float] | None] = []
    for x in xs:
        bet = process.bet()
        for j, m in enumerate(means):
            if not inside[j]:
                continue
            up[j] *= 1.0 + _up_bet(bet, m) * (x - m)
            down[j] *= 1.0 - _down_bet(bet, m) * (x - m)
            if max(CS_HEDGE * up[j], (1.0 - CS_HEDGE) * down[j]) >= threshold:
                inside[j] = False
        process.observe(x)
        kept = [j for j in range(grid + 1) if inside[j]]
        out.append((max(0, kept[0] - 1) / grid, min(grid, kept[-1] + 1) / grid) if kept else None)
    return out


def sequence_holds(z: Sequence[float], theta: float, alpha: float = 0.05, *, z_max: float = 1.0) -> bool:
    """Whether the sequence covers θ at every time: K_t^±(ε_floor·θ/z_max) stays below 1/α throughout."""
    if not 0.0 <= theta <= 1.0:
        raise ValueError(f"a rate is in [0, 1], not {theta}")
    threshold = 1.0 / alpha
    return all(capital < threshold for capital in hedged_capital(_scaled(z, z_max), EPS_FLOOR * theta / z_max, alpha))


class _Capital:
    """The bets' shared state: λ̇_t depends on the past values only, never on the mean under test."""

    __slots__ = ("_log_term", "_t", "_total", "_squares")

    def __init__(self, alpha: float) -> None:
        if not 0.0 < alpha < 1.0:
            raise ValueError(f"alpha is in (0, 1), not {alpha}")
        self._log_term = 2.0 * math.log(2.0 / alpha)
        self._t = 1
        self._total = 0.5  # 1/2 + Σ x_i, the regularized running mean's numerator
        self._squares = 0.25  # 1/4 + Σ (x_i − μ̂_i)², which is σ̂²_{t−1}·t

    def bet(self) -> float:
        return math.sqrt(self._log_term / (self._squares * math.log(self._t + 1.0)))

    def observe(self, x: float) -> None:
        self._total += x
        self._squares += (x - self._total / (self._t + 1)) ** 2
        self._t += 1


def _up_bet(bet: float, m: float) -> float:
    return bet if m == 0.0 else min(bet, CS_C / m)


def _down_bet(bet: float, m: float) -> float:
    return bet if m == 1.0 else min(bet, CS_C / (1.0 - m))


def _scaled(z: Sequence[float], z_max: float) -> list[float]:
    if not EPS_FLOOR <= z_max <= 1.0:
        raise ValueError(f"z_max is in [{EPS_FLOOR}, 1], not {z_max}")
    out = []
    for index, value in enumerate(z):
        if not 0.0 <= value <= z_max:
            raise ValueError(f"z[{index}] = {value} is outside [0, z_max = {z_max}]")
        out.append(value / z_max)
    return out


def _unit_interval(x: float) -> float:
    if not 0.0 <= x <= 1.0:
        raise ValueError(f"a value is in [0, 1], not {x}")
    return x


def _unit(pi: float, y: int) -> tuple[float, int]:
    if isinstance(pi, bool) or not isinstance(pi, (int, float)) or not 0.0 < pi <= 1.0:
        raise ValueError(f"an inclusion probability is in (0, 1], not {pi!r}")
    if isinstance(y, bool) or y not in (0, 1):
        raise ValueError(f"a label is 0 or 1, not {y!r}")
    return float(pi), int(y)
