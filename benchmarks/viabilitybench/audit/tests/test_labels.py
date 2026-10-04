"""Tests for audit/labels.py's `_hidden`: the real truth-suite check count (gap-9f9c03), read from a run record's
`vs.hidden_checks` when present. audit/tests/test_battery.py exercises `label_row`/`_hidden` end to end through real
run records (and pins the pre-fix, n_known=False shape for a record without `hidden_checks`); this file is the
focused unit test for the n/n_known logic itself.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_labels.py
"""

from __future__ import annotations

import sys
from pathlib import Path

VB_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(VB_ROOT))
import audit  # noqa: E402
from audit import labels  # noqa: E402

TASK = {"family": "F4"}


def test_hidden_n_known_true_when_record_has_check_count():
    # gap-9f9c03: driver/records.py now persists vs.hidden_checks, the length of hidden.py's own "checks" list
    # (every family's contract). A record that carries it gets the real n and n_known True.
    vs = {"truth_suite_version": "1.0.0", "hidden_checks": 5}
    assert labels._hidden(TASK, vs, 1, []) == {"suite": "truth:F4@1.0.0", "n": 5, "failed": 0, "n_known": True}
    assert labels._hidden(TASK, vs, 0, ["hidden.rounding", "hidden.negative"]) == {
        "suite": "truth:F4@1.0.0", "n": 5, "failed": 2, "n_known": True}
    # Known and failed, but nothing named in `failed` as hidden.* (shouldn't happen structurally, per census.py's own
    # invariant, but exercise it anyway): trust the exact zero rather than falling back to the old "at least 1" guess.
    assert labels._hidden(TASK, {**vs, "hidden_checks": 3}, 0, [])["failed"] == 0

    # A record from before the fix (no hidden_checks at all): the historical lower-bound fallback, unchanged.
    old = {"truth_suite_version": "1.0.0"}
    assert labels._hidden(TASK, old, 1, []) == {"suite": "truth:F4@1.0.0", "n": 0, "failed": 0, "n_known": False}
    assert labels._hidden(TASK, old, 0, []) == {"suite": "truth:F4@1.0.0", "n": 1, "failed": 1, "n_known": False}
    assert labels._hidden(TASK, old, 0, ["hidden.rounding"]) == {
        "suite": "truth:F4@1.0.0", "n": 1, "failed": 1, "n_known": False}

    # A malformed hidden_checks (negative, not an int, missing, or a bool masquerading as one) is unknown too.
    for bad in (-1, "5", None, True, False):
        assert labels._hidden(TASK, {**vs, "hidden_checks": bad}, 1, [])["n_known"] is False

    assert labels._hidden(TASK, vs, None, []) is None  # the suite could not run
