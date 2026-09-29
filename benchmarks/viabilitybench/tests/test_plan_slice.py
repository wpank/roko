#!/usr/bin/env python3
"""Verifier CI for the plan-level slice (gap-89f393): every feature's reference solution passes its hidden
whole-feature suite and the visible tests, and its stub (the untouched base repo) fails both.

The slice lives in `families/plan_slice/`; its finer checks are in `families/plan_slice/test_slice.py`.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/tests/test_plan_slice.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "families" / "plan_slice"))
import slicekit  # noqa: E402


def test_plan_slice_reference_passes_and_stub_fails():
    features = slicekit.load_features()
    assert slicekit.FEATURES_MIN <= len(features) <= slicekit.FEATURES_MAX
    for feature in features:
        assert slicekit.shape_errors(feature) == [], feature.id
        row = slicekit.selftest(feature, seed=1, partials=False)
        assert row["reference"]["hidden"] and row["reference"]["visible"] and row["reference"]["verified"], row
        assert not row["stub"]["hidden"] and not row["stub"]["visible"], row
        assert row["ok"], row
