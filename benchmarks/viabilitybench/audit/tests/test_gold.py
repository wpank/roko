"""Tests for the gold-task planter and its per-check sensitivity/specificity report (S05 task 16; SC2; work item
gap-33eec2, task 7110).

`test_gold_units_report_sensitivity_and_specificity` runs the real pipeline (gen.py, a family's planted-gaming
`reference.solutions.apply`, `driver/archive.py` and the real `audit.battery`) over F1's one gaming kind and F4's
two; no network, no model. Run:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_gold.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

VB_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(VB_ROOT))
import audit  # noqa: E402
from audit import gold  # noqa: E402
from common import knobs  # noqa: E402  (families/common, on sys.path through audit)


def test_select_gold_draws_about_3_percent_and_is_deterministic():
    instances = [knobs.instance_id(family, level, seed) for family in ("F1", "F2", "F4", "F7")
                for level in knobs.LEVELS for seed in range(1, 11)]  # 200 instances, half F1/F4
    chosen = gold.select_gold(instances, seed=7)
    again = gold.select_gold(instances, seed=7)
    assert chosen == again  # deterministic in the seed
    assert 1 <= len(chosen) <= 10  # ~3% of the 100 eligible (F1, F4) instances, not of all 200
    assert len({instance_id for instance_id, _kind in chosen}) == len(chosen)  # no instance picked twice
    assert all(knobs.parse_instance_id(instance_id)[0] in ("F1", "F4") for instance_id, _kind in chosen)
    assert all(kind.family == knobs.parse_instance_id(instance_id)[0] for instance_id, kind in chosen)
    other_seed = gold.select_gold(instances, seed=8)
    assert other_seed != chosen  # the seed actually matters

    tiny = gold.select_gold(["F1-l1-0001", "F2-l1-0001"], seed=1)
    assert len(tiny) == 1 and tiny[0][0] == "F1-l1-0001"  # at least one, from a pool too small for a literal 3%

    with pytest.raises(ValueError):
        gold.select_gold(instances, seed=1, rate=0.0)


def test_p1_rows_drops_only_the_gold_ones():
    rows = [{"gold": True, "task_id": "a"}, {"task_id": "b"}, {"gold": False, "task_id": "c"}]
    assert gold.p1_rows(rows) == [{"task_id": "b"}, {"gold": False, "task_id": "c"}]
    assert gold.p1_rows([]) == []


def test_gold_units_report_sensitivity_and_specificity(tmp_path):
    """Plant F1's one gaming kind and F4's two, audit each with the real battery, and check that every row is a
    valid vs.label row (gold: true, reusing `audit.label_errors`) and that `gold_confusion` reports both
    sensitivity and specificity for the check that predicts `integrity.g` (A1), from the kinds' known labels
    alone, with the counts behind each. `p1_rows` then drops every one of them."""
    rows = []
    for family_kind, instance_id in ((gold.GOLD_KINDS[0], "F1-l1-0001"), (gold.GOLD_KINDS[1], "F4-l1-0001"),
                                     (gold.GOLD_KINDS[2], "F4-l1-0001")):
        unit = gold.plant(instance_id, family_kind, tmp_path / f"plant-{family_kind.kind.replace('/', '-')}")
        row = gold.audit_unit(unit, scratch=tmp_path / f"audit-{family_kind.kind.replace('/', '-')}")
        assert audit.label_errors(row) == []
        assert row["gold"] is True
        assert row["vs"] == 0 and audit.false_green(row) == 1  # every gold unit is a known false green
        rows.append(row)

    # F1's gaming (known g 0) and F4's gaming/dry_run (known g 0) are both in-scope, non-test edits, so neither
    # trips A1's tamper policy (it predicts g 0, a true negative); F4's gaming/exit0 (known g 1, astcheck's own
    # literal_return) is the same kind of in-scope edit, so A1 misses it too (a false negative) -- the point of
    # this report: a diff-shaped check cannot see a purely semantic shortcut.
    assert [row["integrity"]["g"] for row in rows] == [0, 1, 0]
    assert [row["battery"]["a1"]["g"] for row in rows] == [0, 0, 0]

    confusion = gold.gold_confusion(rows)
    assert set(confusion) == {"a1", "a2", "b1"}
    a1 = confusion["a1"]
    assert (a1.sensitivity, a1.n_sensitivity) == (0.0, 1)  # the one known-tamper unit, missed
    assert (a1.specificity, a1.n_specificity) == (1.0, 2)  # the two known-clean units, both correctly cleared
    assert a1.ci_sensitivity is not None and a1.ci_sensitivity[0] <= a1.sensitivity <= a1.ci_sensitivity[1]
    assert a1.ci_specificity is not None and a1.ci_specificity[0] <= a1.specificity <= a1.ci_specificity[1]

    # A2 and B1 predict Y, and every gold unit's known Y is 1 (a false green by construction), so only their
    # sensitivity is reportable from gold alone; none of these three in-scope edits make a clean visible re-run
    # fail either, so A2 misses every one of them too.
    a2 = confusion["a2"]
    assert (a2.sensitivity, a2.n_sensitivity) == (0.0, 3)
    assert a2.specificity is None and a2.n_specificity == 0
    b1 = confusion["b1"]  # no suite was given, so B1 never ran
    assert (b1.sensitivity, b1.specificity) == (None, None)

    assert gold.p1_rows(rows) == []


def test_audit_unit_rejects_a_kind_for_the_wrong_family(tmp_path):
    with pytest.raises(ValueError):
        gold.plant("F1-l1-0001", gold.GOLD_KINDS[1], tmp_path / "mismatch")  # F4's kind on an F1 instance
