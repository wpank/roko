"""Tests for the real Holm decision reaching T3's footer, and for deriving `adverse` from an envelope ratio
instead of defaulting it (gap-04496d). `tab_t3_headline.py`'s own table-drawing is covered by `test_figures_p1.py`;
these tests are about `holm_decisions.py`, the first caller of `holm.decide`/`write_verdicts` outside holm.py's
own tests, and what it hands `tab_t3_headline.py` and `showcase/build_bundle.py` to read.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_tab_t3_headline.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import figlib
import holm
import holm_decisions
import tab_t3_headline
from test_figures_p1 import Fixture, write_fixture

# E-P1-live's roko_full-vs-fd_claude cell, H1's chain claimed (l1 only) and H2 also rejected.
P_SUPPORTED = {"H1:l1": 0.001, "H1:l2": 0.9, "H1:l3": 0.9, "H1:l4": 0.9, "H1:l5": 0.9,
              "H2": 0.001, "H3": 0.9, "H4": 0.9, "H5": 0.9, "H6": 0.9, "H7": 0.9}
# Nothing rejected: H1's own node p is far above alpha, whatever its envelope ratio says.
P_NOT_REJECTED = dict.fromkeys(("H1:l1", "H1:l2", "H1:l3", "H1:l4", "H1:l5", "H2", "H3", "H4", "H5", "H6", "H7"), 0.9)


def fixture_inputs(tmp_path: Path) -> tuple[figlib.Inputs, Path]:
    """`test_figures_p1.Fixture`'s records, minus its own hand-built `holm_reject` rows -- this module computes
    those from a real `HolmResult` instead, and a cut with two records of the same hypothesis is a figlib error."""
    records = [record for record in Fixture().build() if record["metric"] != "holm_reject"]
    path = write_fixture(tmp_path / "metrics.json", records)
    return figlib.load([path], dry_run=True, p1=True), path


def test_tab_t3_headline_shows_a_real_holm_decision(tmp_path):
    """gap-04496d: before holm_decisions.write ran, T3's footer said "no holm_reject record in the inputs" for
    every hypothesis, because nothing produced one; it now reads the MetricRecord holm_decisions.write projects
    from the same HolmResult verdicts.json carries, so the footer shows the real decision."""
    inputs, metrics_path = fixture_inputs(tmp_path)
    out = holm_decisions.write(inputs, "E-P1-live", P_SUPPORTED, tmp_path)
    holm_path = write_fixture(tmp_path / "holm_reject.json", [dict(row, simulated=True) for row in out["holm_reject"]])
    merged = figlib.load([metrics_path, holm_path], dry_run=True, p1=True)

    footer = tab_t3_headline.build(merged).footer
    assert "H1: Holm rejects the null." in footer
    assert "H2: Holm rejects the null." in footer
    assert not any("no holm_reject record in the inputs" in line for line in footer)


def test_adverse_from_ratio_reads_an_existing_sign_not_a_new_statistic():
    """R below 1 or C above 1 (module docstring) is adverse; neither ratio read means no answer, not a guess."""
    assert holm_decisions.adverse_from_ratio(0.8, None) is True  # R below 1: roko resolves less than the reference
    assert holm_decisions.adverse_from_ratio(None, 1.3) is True  # C above 1: roko costs more
    assert holm_decisions.adverse_from_ratio(1.1, 0.4) is False  # both favourable
    assert holm_decisions.adverse_from_ratio(None, None) is None


def test_write_passes_adverse_from_the_envelope_ratio_into_the_verdict(tmp_path):
    """gap-04496d: no production caller passed `adverse` before, so a real H1 point estimate pointing the wrong
    way could only ever read as INCONCLUSIVE. holm_decisions.write reads it off envelope_ratio_r/_c (this
    fixture's are adverse to H1 -- test_find_adverse_matches_the_fixtures_own_ratio below pins the sign) and the
    verdict reflects it even when the null is not rejected, not the conservative default."""
    inputs, _ = fixture_inputs(tmp_path)
    assert holm_decisions.find_adverse(inputs, "E-P1-live") == {"H1": True}

    out = holm_decisions.write(inputs, "E-P1-live", P_NOT_REJECTED, tmp_path)
    verdicts = json.loads(out["verdicts"].read_text(encoding="utf-8"))["records"]
    h1 = next(record for record in verdicts if record["hypothesis"] == "H1")
    assert (h1["rejected"], h1["claim_state"]) == (False, "NOT_SUPPORTED")
    # Every other primary keeps verdict_records' own conservative default: no ratio wired, so INCONCLUSIVE.
    assert {record["hypothesis"]: record["claim_state"] for record in verdicts if record["hypothesis"] != "H1"} \
        == {primary: "INCONCLUSIVE" for primary in holm.PRIMARIES if primary != "H1"}


def test_holm_reject_records_copy_their_provenance_from_the_template(tmp_path):
    """holm_reject_records computes nothing about an experiment's runs itself: every provenance field is the
    template record's own, and only the decision fields (metric, value, estimator, the extra clause) are new."""
    inputs, _ = fixture_inputs(tmp_path)
    template = inputs.one("envelope_level", experiment="E-P1-live", arm="roko_full", against=(figlib.REFERENCE_ARM,))
    result = holm.decide(P_SUPPORTED)
    [h1] = holm_decisions.holm_reject_records(result, template.doc, hypotheses=("H1",))
    for field in ("experiment_id", "arms", "run_ids", "seeds", "commits", "config_hashes", "analysis_commit",
                 "computed_at", "preregistered", "prereg_id", "blinded", "label_source", "cost_basis",
                 "price_snapshot_id", "n"):
        assert h1[field] == template.doc[field], field
    assert (h1["metric"], h1["value"]) == ("holm_reject", 1)
    assert h1["record_filter"] == template.doc["record_filter"] + ' and hypothesis == "H1"'
    assert "ci" not in h1 and "ladder" not in h1
