"""Tests for the M3 economics report (S04 §4.9, task 6123), on synthetic run records and policy traces.

Every record is built from the schema's example and validates against `vb.run_record/1`; nothing calls a model.
Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_econ.py -q
"""

from __future__ import annotations

import json
from fractions import Fraction

import pytest

import bootstrap
import econ
import passk
import report
import validate  # schema/validate.py, on sys.path through report

EXAMPLE = json.loads((report.VB_ROOT / "schema" / "examples" / "run-record.json").read_text())
EXPERIMENT = "ECON-T"
# The golden arm's labels by task, one per seed from 1: c of n = 2 of 5, 5 of 5, 0 of 5 and 1 of 2.
GOLDEN = {"F1-l1-0001": [1, 0, 1, 0, 0], "F1-l1-0002": [1, 1, 1, 1, 1], "F2-l2-0003": [0, 0, 0, 0, 0],
          "F2-l2-0004": [1, 0]}


def run_record(instance: str, seed: int, arm: str, label: int, cost: float | None = 0.02, *,
               experiment: str = EXPERIMENT, audit: float | None = None) -> dict:
    """A valid run record of one (instance, seed) on `arm`, which ran gpt-oss-120b."""
    record = json.loads(json.dumps(EXAMPLE))
    family, level = instance.split("-")[0], int(instance.split("-")[1][1:])
    run_id = f"run-{arm}-{seed}"
    record.update(experiment_id=experiment, run_id=run_id, arm=arm, seed=seed, record_id=f"sha256:{run_id}/{instance}")
    record["task"].update(family=family, instance_id=instance, ladder=level, is_honeypot=False)
    record["execution"]["attempts"][0]["attempt_key"] = f"{run_id}/{instance}.s{seed}:1"
    record["visible"]["passed"] = bool(label)
    record["vs"] = {"label": label, "unknown": False,
                    "checks": {"completion": 1, "visible_clean": label, "hidden": label, "integrity": 1},
                    "truth_suite_version": "1.0.0", "failed": [] if label else ["hidden.cases"]}
    if audit is not None:
        record["vs"]["cost_usd"] = {"impl": cost, "escalation": 0.0, "spec_refine": None, "audit": audit}
    if cost is None:
        record["costs"] = {"api_equiv_usd": None, "billed_usd": None, "without_cache_usd": None, "vendor_usd": None,
                           "source": "unknown", "meter_cross_check_usd": None}
    else:
        record["costs"] = {"api_equiv_usd": cost, "billed_usd": cost, "without_cache_usd": cost, "vendor_usd": None,
                           "source": "provider_usage", "meter_cross_check_usd": None}
    assert validate.validate("run-record", record) == []
    return record


def golden_records() -> list[dict]:
    """The golden arm, plus cheap_x: it fails everywhere and one of its runs has an unknown cost."""
    records = [run_record(instance, seed, "golden", label)
               for instance, labels in GOLDEN.items() for seed, label in enumerate(labels, 1)]
    records += [run_record(instance, seed, "cheap_x", 0, None if (instance, seed) == ("F1-l1-0001", 2) else 0.01)
                for instance in GOLDEN for seed in (1, 2, 3)]
    return records


def trace(policy: str, ordering: int, task: str, arms: list[str], passed: bool, cost: float) -> dict:
    return {"policy": policy, "ordering": ordering, "task": task, "family": task.split("-")[0], "arms": arms,
            "passed": passed, "cost_usd": cost}


def golden_traces() -> list[dict]:
    """Two orderings of a static policy on the golden arm, and two π* settings of lcb_aci that try cheap_x first."""
    lines = []
    for ordering in (0, 1):
        for task, labels in GOLDEN.items():
            lines.append(trace("H4-B0", ordering, task, ["golden"], bool(labels[0]), 0.02))
            lines.append(trace("lcb_aci@0.9", ordering, task, ["cheap_x", "golden"], bool(labels[0]), 0.03))
            lines.append(trace("lcb_aci@0.5", ordering, task, ["cheap_x"], False, 0.01))
    return lines


def spread_records(tasks: int) -> list[dict]:
    """`tasks` tasks in two strata, three seeds each, alternating between 2 of 3 and 1 of 3 verified."""
    records = []
    for index in range(tasks):
        instance = f"F1-l1-{index:04d}" if index % 2 else f"F2-l2-{index:04d}"
        records += [run_record(instance, seed, "spread", (index + seed) % 2, experiment=f"ECON-{tasks}")
                    for seed in (1, 2, 3)]
    return records


def test_policy_report_reuses_passk_and_cluster_bootstrap(monkeypatch, tmp_path):
    calls = {"pass_k": [], "paired_bootstrap": []}
    real_pass_k, real_bootstrap = passk.pass_k, bootstrap.paired_bootstrap

    def spy_pass_k(labels, k):
        calls["pass_k"].append(k)
        return real_pass_k(labels, k)

    def spy_bootstrap(rows, statistic, **kwargs):
        calls["paired_bootstrap"].append(kwargs)
        return real_bootstrap(rows, statistic, **kwargs)

    monkeypatch.setattr(passk, "pass_k", spy_pass_k)
    monkeypatch.setattr(bootstrap, "paired_bootstrap", spy_bootstrap)
    doc = econ.build(golden_records(), golden_traces())

    # pass^k and pass@k: the closed forms over independent seeds, the 2-seed task skipped for k = 3 and 5.
    golden = doc["arms"]["golden"]
    hat = {1: Fraction(19, 40), 3: Fraction(1, 3), 5: Fraction(1, 3)}
    at = {1: Fraction(19, 40), 3: Fraction(19, 30), 5: Fraction(2, 3)}
    for k in econ.KS:
        assert golden["pass_hat_k"][str(k)] == {"value": float(hat[k]), "tasks": 4 if k == 1 else 3,
                                                "skipped": 0 if k == 1 else 1}
        assert golden["pass_at_k"][str(k)]["value"] == float(at[k])
    assert sorted(set(calls["pass_k"])) == [1, 3, 5]
    # Every interval comes from the stratified paired bootstrap with S04's 2,000 resamples.
    assert calls["paired_bootstrap"] and all(call["b"] == 2_000 for call in calls["paired_bootstrap"])
    assert doc["b"] == 2_000 and doc["strata"] == ["family", "ladder"]
    assert golden["ci"]["vs_rate"]["low"] <= golden["vs_rate"] <= golden["ci"]["vs_rate"]["high"]
    assert golden["vs_rate"] == float(Fraction(19, 40))
    assert golden["cpr_usd"] == pytest.approx(0.02 * 17 / 8)

    # Arms with an unknown cost leave CPR, the oracle and the Pareto set, and the report names them.
    cheap = doc["arms"]["cheap_x"]
    assert cheap["cpr_usd"] is None and cheap["cpr_excl_audit_usd"] is None and cheap["ci"]["cpr_usd"] is None
    assert "unknown cost" in cheap["cpr_note"]
    assert doc["cost_coverage"]["excluded_arms"] == [{"arm": "cheap_x", "runs": 12, "unknown_cost_runs": 1}]
    names = [f"{point['kind']}:{point['name']}" for point in doc["pareto"]["points"]]
    assert "arm:cheap_x" not in names and "arm:golden" in names

    # The policy view: rates and CPR from the traces, no pass^k (orderings are not seeds), the π* sweep.
    static = doc["policies"]["H4-B0"]
    assert static["vs_rate"] == 0.75 and static["cpr_usd"] == pytest.approx(0.16 / 6)
    assert static["pass_hat_k"] is None and static["pass_k_note"] == econ.NOT_INDEPENDENT
    assert [point["setting"] for point in doc["sweeps"]["lcb_aci"]] == [0.5, 0.9]
    assert doc["ceilings"]["beta"]["beta"] == 0.25
    assert doc["ceilings"]["cross_seed_oracle"]["vs_rate"] == float(Fraction(19, 40))
    assert doc["pareto"]["frontier"] == ["policy:lcb_aci@0.5", "policy:H4-B0"]

    # The interval narrows as the number of tasks grows.
    widths = []
    for tasks in (6, 48):
        spread = econ.build(spread_records(tasks))["arms"]["spread"]["ci"]["vs_rate"]
        widths.append(spread["high"] - spread["low"])
    assert widths[1] < widths[0]

    # Two runs on one fixture write identical bytes, with sorted keys.
    first = econ.write(econ.build(golden_records(), golden_traces()), tmp_path / "a" / "econ-report.json")
    second = econ.write(econ.build(golden_records(), golden_traces()), tmp_path / "b" / "econ-report.json")
    assert first.read_bytes() == second.read_bytes()
    text = first.read_text(encoding="utf-8")
    assert text == json.dumps(json.loads(text), sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def test_variance_audit_spend_and_excluded_runs():
    golden = econ.build(golden_records(), b=200)["arms"]["golden"]
    # Within-task: (3/10 + 0 + 0 + 1/2) / 4; ICC(1) from the one-way ANOVA of the same four tasks.
    assert golden["variance"]["within_task"] == pytest.approx(0.2)
    assert golden["variance"]["icc1"] == pytest.approx(0.570190, abs=1e-6)

    # Audit spend from S05's cost split: in CPR, out of cpr_excl_audit_usd.
    records = [run_record("F1-l1-0001", seed, "audited", 1, 0.02, audit=0.01) for seed in (1, 2)]
    audited = econ.build(records, b=200)["arms"]["audited"]
    assert audited["cpr_usd"] == pytest.approx(0.03) and audited["cpr_excl_audit_usd"] == pytest.approx(0.02)

    # An infra_error run is excluded and counted; a trace line that used it is left out.
    broken = golden_records()
    broken[0]["execution"]["status"] = "infra_error"
    doc = econ.build(broken, golden_traces(), b=200)
    assert doc["runs"]["infra_error"] == 1 and doc["arms"]["golden"]["runs"] == len(broken) - 12 - 1
    assert doc["traces"]["touched_excluded"] == 4  # F1-l1-0001's line in two orderings of two golden policies


def test_cc_k_metric_is_produced():
    # gap-889682: cc_<k> = pass^k / pass@k and per-task cost_cv had no producer. Eight identical tasks (c = 3 of 5
    # seeds each) make pass^k/pass@k closed forms, and keep every bootstrap replicate identical too -- no task is
    # ever skipped or all-zero, so the interval is a clean, reproducible point: pass^1 == pass@1 == 0.6 (CC_1 = 1,
    # the identity that always holds at k = 1, any data); pass^3 = C(3,3)/C(5,3) = 0.1, pass@3 = 1 - C(2,3)/C(5,3)
    # = 1 (CC_3 = 0.1); pass^5 = C(3,5)/C(5,5) = 0, pass@5 = 1 - C(2,5)/C(5,5) = 1 (CC_5 = 0).
    flat = [run_record(f"F1-l1-{index:04d}", seed, "flat", int(seed <= 3), cost=0.03)
           for index in range(8) for seed in (1, 2, 3, 4, 5)]
    produced = econ.cc_metrics(flat, EXPERIMENT, b=50, seed=0, alpha=0.05)
    cc = {metric.metric: metric for metric in produced if metric.metric.startswith("cc_")}
    assert set(cc) == {"cc_1", "cc_3", "cc_5"}
    assert cc["cc_1"].value == pytest.approx(1.0) and cc["cc_1"].ci == pytest.approx((1.0, 1.0))
    assert cc["cc_3"].value == pytest.approx(0.1) and cc["cc_3"].ci == pytest.approx((0.1, 0.1))
    assert cc["cc_5"].value == pytest.approx(0.0) and cc["cc_5"].ci == pytest.approx((0.0, 0.0))
    assert {metric.n for metric in cc.values()} == {40}  # 8 tasks x 5 seeds, all independent

    cv = [metric for metric in produced if metric.metric == "cost_cv"]
    assert len(cv) == 8 and {metric.value for metric in cv} == {0.0}  # cost 0.03 on every run: no spread

    for metric in produced:
        row = report.metric_record(metric, experiment_id=EXPERIMENT, snapshot="prices-2026-09-28",
                                   analysis_commit="abc", computed_at="now")
        assert validate.validate("metric-record", row) == [], row
        if metric.metric == "cc_3":
            assert row["ci_method"] == "task_bootstrap_percentile" and row["cost_basis"] is None
        if metric.metric == "cost_cv":
            assert row["cost_basis"] == "api_equiv_usd" and "ci" not in row
            assert 'task.instance_id == "' in row["record_filter"]

    # cheap_x (golden_records()'s always-failing arm) gets no cc_<k> at all: its pass@k is 0 at every k.
    assert not [metric for metric in econ.cc_metrics(golden_records(), EXPERIMENT, b=50)
               if metric.metric.startswith("cc_") and 'arm == "cheap_x"' in metric.cut.filter]


def test_cli_writes_the_report(tmp_path, capsys):
    records = tmp_path / "records.jsonl"
    records.write_text("".join(json.dumps(record) + "\n" for record in golden_records()), encoding="utf-8")
    traces = tmp_path / "traces.jsonl"
    traces.write_text("".join(json.dumps(line) + "\n" for line in golden_traces()), encoding="utf-8")
    out = tmp_path / "econ-report.json"
    assert econ.main(["--records", str(records), "--traces", str(traces), "--out", str(out), "--b", "200"]) == 0
    assert json.loads(out.read_text())["schema_version"] == econ.SCHEMA
    assert "excluded from CPR (unknown cost): cheap_x (1 of 12 runs)" in capsys.readouterr().out
    traces.write_text(json.dumps({"policy": "x"}) + "\n", encoding="utf-8")
    assert econ.main(["--records", str(records), "--traces", str(traces), "--out", str(out)]) == 1
