"""Live manifests for H5, H6 and H7 (3361): each validates, names its budget line and cap (experiments/budget.toml)
and the loops `requires_live` needs LIVE at the harness sha (3359), through `vb campaign --dry-run`. The
pre-registration lock (3341; taking it is task 3345, held for the author as gap-394f28) and the real loop census
(needs a roko binary) are both prerequisites this file fakes to validate the manifests' own shape: neither is this
task's job.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_live.py -q
"""

from __future__ import annotations

import json
from pathlib import Path

import campaign
import layout
import ledger
import vb
from common import hmac_seed

# experiment id -> (manifest path, its one budget line, the loops requires_live names)
MANIFESTS: dict[str, tuple[Path, str, list[str]]] = {
    "E-H5-live": (layout.VB_ROOT / "experiments" / "h5_live.toml", "BL4",
                  ["L-audit", "L-gate-depth", "L-route-trust"]),
    "E-H6-live": (layout.VB_ROOT / "experiments" / "h6_live.toml", "BL3",
                  ["L-audit", "L-gate-depth", "L-route-trust"]),
    "E-H7-live": (layout.VB_ROOT / "experiments" / "h7_live.toml", "BL5",
                  ["L-M1", "L-audit", "L-route-trust"]),
}


def fake_census(loops: list[str], sha: str = "deadbeef1234"):
    """A monkeypatch replacement for `campaign.census_report`, matching its real signature, that reports every
    name in `loops` LIVE (campaign.py's requires_live, 3359's own test plan reused here for 3361)."""
    def fake(repo=None, roko_bin=None):
        return {"schema": campaign.LOOPS_SCHEMA, "harness_sha": sha,
                "rows": [{"loop": name, "state": "live"} for name in loops]}
    return fake


def test_live_manifests_name_lines_caps_and_required_loops(tmp_path, capsys, monkeypatch):
    """Each of h5_live.toml, h6_live.toml and h7_live.toml (3361) validates against vb.experiment/1, names the
    budget line and cap S09 §5 and experiments/budget.toml assign it, and names the loops requires_live needs
    LIVE. The dry run passes once the loop census is faked LIVE and the pre-registration lock, not this task's
    job, is faked clean (3341/3345)."""
    budget = ledger.load_budget()
    monkeypatch.setattr(campaign, "lock_refusal", lambda lock=None, spec=None: None)
    for experiment_id, (path, line, loops) in MANIFESTS.items():
        manifest = campaign.load(path)
        assert manifest.id == experiment_id
        assert manifest.requires_lock and campaign.requires_lock(experiment_id)  # E-prefixed: locked either way
        assert manifest.requires_live == tuple(loops)
        assert {block.line for block in manifest.blocks} == {line}, f"{experiment_id}: every block should be {line}"

        monkeypatch.setattr(campaign, "census_report", fake_census(loops))
        secret_file = hmac_seed.write_secret_file(tmp_path / experiment_id / "private-config" / "secret")
        code = vb.main(["campaign", "--manifest", str(path), "--dry-run", "--results",
                        str(tmp_path / experiment_id / "results"), "--secret-file", str(secret_file)])
        shown = json.loads(capsys.readouterr().out)
        assert code == 0 and shown["ok"], shown["problems"]
        assert shown["requires_live"] == loops and shown["harness_sha"] == "deadbeef1234" and shown["not_run"] == []

        scope = shown["budget"][f"line {line}"]
        assert scope["cap_usd"] == budget.lines[line].cap_usd
        assert scope["planned_usd"] <= scope["cap_usd"], f"{experiment_id}: plans past {line}'s own cap"
