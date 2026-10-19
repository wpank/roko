"""PILOT-A-RECAL's manifest (`pilot_a_recal.toml`, gap-dd670b): Pilot A's cheap_direct block on the re-tuned F1 and F4,
as one unit under the coordinator's $2 cap. Offline; the dry run bills nothing. `test_pilot_a.py`'s rehearsal runs
the same stream on the same arm through the stub provider.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_a_recal.py -q
"""

from __future__ import annotations

import json

import campaign
import layout
import ledger
import vb
from common import hmac_seed

RECAL = layout.VB_ROOT / "experiments" / "pilot_a_recal.toml"


def test_pilot_a_recal_is_one_unit_of_60_runs_under_2_dollars(tmp_path, capsys):
    secret = hmac_seed.write_secret_file(tmp_path / "config" / "secret")
    assert vb.main(["campaign", "--manifest", str(RECAL), "--dry-run", "--max-cost-usd", "2", "--results",
                    str(tmp_path / "results"), "--secret-file", str(secret)]) == 0
    shown = json.loads(capsys.readouterr().out)
    assert shown["ok"] and shown["experiment"] == "PILOT-A-RECAL" and shown["runs"] == 60
    [block] = shown["blocks"]
    assert (block["block"], block["runs"], block["network"], block["billed"]) == ("cheap-direct", 60, True, True)
    # One unit, so the last task's worst case is held against the whole $2, not against a third of it.
    assert [unit["unit"] for unit in shown["units"]] == ["cheap-direct"]
    assert [unit.key for unit in campaign.units(campaign.load(RECAL))] == ["cheap-direct"]
    assert shown["budget"]["line BL0"]["planned_usd"] == 1.20
    assert shown["budget"]["experiment cap pilot"]["planned_usd"] == 1.20 <= 15
    assert ledger.load_budget().experiment("PILOT-A-RECAL").id == "pilot"
