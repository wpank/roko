"""The audit lottery (S05 §4.2, DP1): which green units get a deep audit, by a draw anyone can re-check afterwards.

Every green unit i gets an inclusion probability π_i and one keyed draw u_i, and is selected when u_i < π_i: Poisson
sampling, one independent draw per unit. The draw is logged when the verdict settles; the audit itself runs later,
off the critical path, and never changes the verdict it audits.

- **π_i** = clip(ρ·[(1−λ) + λ·r_i^α/r̄], ε_floor, π_max) (`inclusion_probability`). r_i is M3's predicted
  P(false green) for the unit, and r̄ the window's mean of r^α, so that π averages ρ before the clip. Without a
  risk, π_i = ρ. The floor ε_floor = 5% is locked (S05 §4.9, decision 7102): it is a constant here, never a
  parameter, so every π_i is at least 0.05 and the Horvitz–Thompson estimate stays unbiased. ρ defaults to 0.10, the
  rate for real work (decision 7103); experiments set their own (H5's primary cell uses 0.15). The census strata,
  `unverified` and the reserved `forced_accept`, are drawn at π = 1 (`CENSUS_VERDICTS` in the package).
- **The tilt** λ = λ_max·max(0, 1 − ECE/ECE_ref), and 0 until M3 has `MIN_TILT_LABELS` audited labels (`tilt`). A
  miscalibrated M3 therefore falls back to uniform sampling, which is never worse than random.
- **The draw.** x_i is the first 8 bytes, read as a big-endian integer, of HMAC-SHA256(K_run, fields), where the
  fields are `DRAW_TAG`, run_id, task_id, attempt_id and accepted_commit (`draw`). u_i = x_i / 2^64, and the unit
  is selected when u_i < π_i, decided exactly as the integer comparison x_i < π_i·2^64 (`is_selected`), so π = 1
  always selects. The record keeps x_i as `prf_u`, "0x" and 16 hex digits.
- **The key and its commitment.** K_run = HMAC-SHA256(workspace secret, (`KEY_TAG`, run_id)) (`run_key`): one 32-byte
  key per run, from a secret held outside every sandbox. At run start the ledger records the commitment
  "sha256:" + hex(SHA-256(K_run ‖ run_id)) (`commitment`), and at run end K_run itself. Revealing one run's key
  says nothing about the secret or about any other run's key.
- **The reveal** (`verify_reveal`): with K_run revealed, anyone recomputes the commitment and every draw and
  selection of the run from the ledger. Any difference is a mismatch.

**The field encoding is shared with Rust (7114).** It is `common.hmac_seed`'s: each field is encoded as UTF-8 and
prefixed by its byte length as a 4-byte big-endian integer, and the pieces are concatenated in order. As there, the
first field is a domain tag, so a draw can never collide with another use of the same key.

API:
    EPS_FLOOR, DEFAULT_RHO, RHO_MAX, PI_MAX, ALPHA, LAMBDA_MAX, ECE_REF, MIN_TILT_LABELS, DRAW_TAG, KEY_TAG
    tilt(ece: float | None, n_labels: int, *, lambda_max=LAMBDA_MAX, ece_ref=ECE_REF) -> float
    inclusion_probability(rho=DEFAULT_RHO, *, risk=None, mean_risk=None, lam=0.0, alpha=ALPHA, pi_max=PI_MAX)
        -> float
    run_key(secret: bytes, run_id: str) -> bytes
    commitment(key: bytes, run_id: str) -> str
    draw(key: bytes, run_id: str, task_id: str, attempt_id: str, accepted_commit: str) -> int
    u_value(x: int) -> float; is_selected(x: int, pi: float) -> bool; prf_hex(x: int) -> str
    Selection(run_id, task_id, attempt_id, accepted_commit, pi, prf_u, selected); .record() -> dict
    select(key, run_id, task_id, attempt_id, accepted_commit, pi) -> Selection
    verify_reveal(key: bytes, run_id: str, committed: str, selections: Iterable[Mapping]) -> list[str]
"""

from __future__ import annotations

import hashlib
import hmac
import math
from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass

import audit  # noqa: F401 (puts families/ on sys.path for common)
from common.hmac_seed import _derive  # the one length-prefixed field encoding, shared on purpose

EPS_FLOOR = 0.05  # locked (S05 §4.9, decision 7102): the author changes it, never a config or a learning loop
DEFAULT_RHO = 0.10  # real work (decision 7103); S05's experiments default to 0.15 and set it in their manifest
RHO_MAX = 0.5
PI_MAX = 1.0
ALPHA = 1.0
LAMBDA_MAX = 0.8
ECE_REF = 0.10
MIN_TILT_LABELS = 50  # M3 tilts nothing until this many audited labels exist
DRAW_TAG = "roko.audit.draw/1"
KEY_TAG = "roko.audit.key/1"
KEY_BYTES = 32
TWO_64 = float(1 << 64)


def tilt(ece: float | None, n_labels: int, *, lambda_max: float = LAMBDA_MAX, ece_ref: float = ECE_REF) -> float:
    """λ for M3's risk tilt: λ_max·max(0, 1 − ECE/ECE_ref), and 0 without an ECE or below `MIN_TILT_LABELS` labels."""
    if not 0.0 <= lambda_max <= 1.0:
        raise ValueError(f"lambda_max is in [0, 1], not {lambda_max}")
    if not ece_ref > 0.0:
        raise ValueError(f"ece_ref must be above 0, not {ece_ref}")
    if ece is None or n_labels < MIN_TILT_LABELS:
        return 0.0
    if not 0.0 <= ece <= 1.0:
        raise ValueError(f"an ECE is in [0, 1], not {ece}")
    return lambda_max * max(0.0, 1.0 - ece / ece_ref)


def inclusion_probability(rho: float = DEFAULT_RHO, *, risk: float | None = None, mean_risk: float | None = None,
                          lam: float = 0.0, alpha: float = ALPHA, pi_max: float = PI_MAX) -> float:
    """π = clip(ρ·[(1−λ) + λ·r^α/r̄], EPS_FLOOR, π_max); uniform (π = ρ before the clip) without a risk or a tilt.

    `mean_risk` is r̄, the window's mean of r^α; a window whose mean is 0 carries no tilt either.
    """
    if not EPS_FLOOR <= rho <= RHO_MAX:
        raise ValueError(f"rho is in [{EPS_FLOOR}, {RHO_MAX}], not {rho}")
    if not 0.0 <= lam <= 1.0:
        raise ValueError(f"lambda is in [0, 1], not {lam}")
    if not alpha > 0.0:
        raise ValueError(f"alpha must be above 0, not {alpha}")
    if not EPS_FLOOR <= pi_max <= 1.0:
        raise ValueError(f"pi_max is in [{EPS_FLOOR}, 1], not {pi_max}")
    if risk is not None and not 0.0 <= risk <= 1.0:
        raise ValueError(f"a risk is a probability in [0, 1], not {risk}")
    if mean_risk is not None and not 0.0 <= mean_risk <= 1.0:
        raise ValueError(f"a mean risk is in [0, 1], not {mean_risk}")
    weight = 1.0
    if lam > 0.0 and risk is not None and mean_risk is not None and mean_risk > 0.0:
        weight = (1.0 - lam) + lam * risk ** alpha / mean_risk
    return min(pi_max, max(EPS_FLOOR, rho * weight))


def run_key(secret: bytes, run_id: str) -> bytes:
    """K_run: HMAC-SHA256(secret, (KEY_TAG, run_id)), 32 bytes."""
    if len(secret) < KEY_BYTES:
        raise ValueError(f"the audit secret must hold at least {KEY_BYTES} bytes")
    return _derive(secret, KEY_TAG, run_id)


def commitment(key: bytes, run_id: str) -> str:
    """The run-start commitment to K_run: "sha256:" + hex(SHA-256(K_run ‖ run_id as UTF-8))."""
    _check_key(key)
    return "sha256:" + hashlib.sha256(key + run_id.encode("utf-8")).hexdigest()


def draw(key: bytes, run_id: str, task_id: str, attempt_id: str, accepted_commit: str) -> int:
    """x_i: the first 8 bytes, big-endian, of HMAC-SHA256(K_run, (DRAW_TAG, run_id, task_id, attempt_id, commit))."""
    _check_key(key)
    fields = (run_id, task_id, attempt_id, accepted_commit)
    if not all(isinstance(field, str) for field in fields):
        raise TypeError(f"the draw's fields are strings, not {[type(field).__name__ for field in fields]}")
    return int.from_bytes(_derive(key, DRAW_TAG, *fields)[:8], "big")


def u_value(x: int) -> float:
    """u_i = x_i / 2^64, rounded to the nearest double (for display; `is_selected` compares exactly)."""
    _check_draw(x)
    return x / TWO_64


def is_selected(x: int, pi: float) -> bool:
    """S_i = 1[x_i / 2^64 < π_i], decided exactly as x_i < π_i·2^64.

    π_i·2^64 is exact in binary floating point, and Python compares an int with a float exactly. Rust gets the same
    answer comparing `x as u128` with `(pi * 2^64) as u128`, which is exact for every π ≥ 2^-11, so above the floor.
    """
    _check_draw(x)
    if not (math.isfinite(pi) and 0.0 < pi <= 1.0):
        raise ValueError(f"an inclusion probability is in (0, 1], not {pi}")
    return x < pi * TWO_64


def prf_hex(x: int) -> str:
    """x_i as the ledger records it (`prf_u`): "0x" and 16 lowercase hex digits."""
    _check_draw(x)
    return f"0x{x:016x}"


@dataclass(frozen=True)
class Selection:
    """One green unit's draw, as `audit.selection` records it (S05 §5)."""

    run_id: str
    task_id: str
    attempt_id: str
    accepted_commit: str
    pi: float
    prf_u: str
    selected: bool

    def record(self) -> dict:
        return asdict(self)


def select(key: bytes, run_id: str, task_id: str, attempt_id: str, accepted_commit: str, pi: float) -> Selection:
    """Draw unit i at probability `pi`."""
    if pi < EPS_FLOOR:
        raise ValueError(f"pi {pi} is below the {EPS_FLOOR} floor")
    x = draw(key, run_id, task_id, attempt_id, accepted_commit)
    return Selection(run_id, task_id, attempt_id, accepted_commit, pi, prf_hex(x), is_selected(x, pi))


def verify_reveal(key: bytes, run_id: str, committed: str, selections: Iterable[Mapping]) -> list[str]:
    """Every mismatch between a revealed key and a run's ledger: its commitment, and each selection's draw.

    `selections` are `audit.selection` records (or `Selection.record()`s) with run_id, task_id, attempt_id,
    accepted_commit, pi, prf_u and selected. An empty list means the key opens the commitment and reproduces every
    draw and every selection, and no π is below the floor.
    """
    try:
        opened = commitment(key, run_id)
    except ValueError as err:
        return [f"key: {err}"]
    mismatches = []
    if opened != committed:
        mismatches.append(f"commitment: the revealed key opens {opened}, but the run committed to {committed}")
    for index, row in enumerate(selections):
        where = f"selection {index} ({row.get('task_id')!r}, attempt {row.get('attempt_id')!r})"
        if row.get("run_id") != run_id:
            mismatches.append(f"{where}: run_id {row.get('run_id')!r} is not {run_id!r}")
            continue
        try:
            x = draw(key, run_id, row["task_id"], row["attempt_id"], row["accepted_commit"])
            selected = is_selected(x, row["pi"])
        except (KeyError, TypeError, ValueError) as err:
            mismatches.append(f"{where}: cannot recompute the draw: {type(err).__name__}: {err}")
            continue
        if row["pi"] < EPS_FLOOR:
            mismatches.append(f"{where}: pi {row['pi']} is below the {EPS_FLOOR} floor")
        if row.get("prf_u") != prf_hex(x):
            mismatches.append(f"{where}: prf_u {row.get('prf_u')!r}, but the key draws {prf_hex(x)}")
        if row.get("selected") is not selected:
            mismatches.append(f"{where}: selected {row.get('selected')!r}, but the draw gives {selected}")
    return mismatches


def _check_key(key: bytes) -> None:
    if not isinstance(key, bytes) or len(key) != KEY_BYTES:
        raise ValueError(f"a run key is {KEY_BYTES} bytes")


def _check_draw(x: int) -> None:
    if isinstance(x, bool) or not isinstance(x, int) or not 0 <= x < 1 << 64:
        raise ValueError(f"a draw is an integer in [0, 2^64), not {x!r}")
