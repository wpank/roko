"""R-H6: S06's controller replay over LOG1 (S09 §4.3 "Replays" and H6; S06.T7; task 3357).

    vb.py replay --experiment LOG1 --adapter h6 [--seed N] [--table FILE]

An adapter under `replay_runner` (3354), so it runs deterministically on arm-hashed data like every replay.

**R-H6**: IAE per (arm, disturbance kind) cell, with its 95% bootstrap interval, from S06's full-information
replay evaluator (S06.T7)'s own rows: `roko_learn::homeostasis::replay::ArmReport`, one per arm x kind, read from
`--table` as `{"arm", "disturbance", "iae": {"mean", "low", "high", "n"} | null, "changes", "holds", "note"}` JSON
lines (`ArmReport`'s own field names). This adapter never recomputes any of that itself: 8117 built the
evaluator as a Rust library inside roko-learn (`crates/roko-learn/src/homeostasis/replay.rs`), and
`roko learn homeostasis replay --evaluate --stream synthetic:all@<t> --out FILE` runs it over disturb.py's six
kinds and writes exactly this table (gap-1a8ee7), the way `roko learn self-model replay` writes R-H4's
`--traces`. Without `--table`, every (arm, kind) cell is `evaluated: False` with that reason, the same as R-H4's
cells with no `--traces` file (analysis/replay_h4.py) and R-H5's with no `--risk` file.

**Arms and kinds.** `ARMS` is replay.rs's own `ReplayArm::ALL`, in its order (A0..A5, A3-gated, A3-mis; S06 §4.9).
`KINDS` is S08's six disturbance kinds this task names (`driver/disturb.py`'s `KINDS`: provider_fault, model_swap,
harder_mix, budget_cut, convention_flip, flaky_verify). A `--table` row naming an arm or kind outside these is
rejected: the adapter reports exactly this 8 x 6 grid, nothing more and nothing fewer. Closure test X2 (3358)
reads its A3, A4 and A3-mis cells back out of this same grid rather than running anything of its own.

API:
    ARMS, KINDS, NOT_WIRED
    load_table(path) -> dict[tuple[str, str], dict]      # (arm, kind) -> its ArmReport-shaped row
    estimate(matrix, rng, table=None) -> dict
"""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

if str(Path(__file__).resolve().parents[1]) not in sys.path:  # the benchmark directory, for replay_runner
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import replay_runner  # noqa: E402

ARMS = ("A0", "A1", "A2", "A3", "A4", "A5", "A3-gated", "A3-mis")  # roko-learn ReplayArm::ALL, in its order
KINDS = ("provider_fault", "model_swap", "harder_mix", "budget_cut", "convention_flip", "flaky_verify")  # disturb.py
NOT_WIRED = ("no --table: S06.T7's replay evaluator (task 8117) has not been run; write one with `roko learn "
            "homeostasis replay --evaluate --stream synthetic:all@<t> --out FILE` and pass it as --table")


def load_table(path: Path) -> dict[tuple[str, str], dict]:
    """`ArmReport`-shaped JSON lines from `path`, keyed by (arm, disturbance); S06.T7's own field names."""
    path = Path(path)
    table: dict[tuple[str, str], dict] = {}
    for number, text in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not text.strip():
            continue
        row = json.loads(text)
        if not (isinstance(row, dict) and row.get("arm") in ARMS and row.get("disturbance") in KINDS):
            raise ValueError(f"{path}:{number}: not an ArmReport row naming one of {ARMS} and one of {KINDS}")
        key = (row["arm"], row["disturbance"])
        if key in table:
            raise ValueError(f"{path}:{number}: {key} repeats")
        table[key] = row
    return table


def _cell(row: dict | None, *, has_table: bool) -> dict:
    if not has_table:
        return {"evaluated": False, "reason": NOT_WIRED}
    if row is None:
        return {"evaluated": False, "reason": "no row in --table for this (arm, kind)"}
    if row.get("note"):
        return {"evaluated": False, "reason": row["note"]}
    iae = row.get("iae")
    if not iae:
        return {"evaluated": False, "reason": "the table's row has no iae and no note"}
    return {"evaluated": True, "iae_mean": iae["mean"], "iae_ci95": [iae["low"], iae["high"]], "n": iae["n"],
            "changes": row.get("changes"), "holds": row.get("holds")}


def estimate(found: replay_runner.OutcomeMatrix, rng: random.Random, table: str | None = None) -> dict:
    """R-H6's (arm, kind) grid (module docstring). `found` and `rng` are unused: the evaluator's own common
    random numbers (replay.rs) are what built `--table`, not this replay's matrix or seed."""
    del found, rng
    loaded = load_table(Path(table)) if table else {}
    cells = [{"arm": arm, "kind": kind, **_cell(loaded.get((arm, kind)), has_table=bool(table))}
            for arm in ARMS for kind in KINDS]
    return {"arms": list(ARMS), "kinds": list(KINDS), "cells": cells,
            "n_evaluated": sum(cell["evaluated"] for cell in cells), "table": str(table) if table else None}


replay_runner.register(replay_runner.Adapter("h6", "r-h6-1", estimate, params=("table",)))
