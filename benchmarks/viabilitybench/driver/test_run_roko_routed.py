"""Offline tests of the Roko arm's routed model check (3312): `run_roko.settle` accepts a ladder's escalations, in
rung order only, and still rejects a failover, a downward step and a model outside the arm's rungs. A routed arm runs
Roko with no `--model` pin, and Roko's plan-start rung probes are no attempt's traffic (bug-0b7695).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_routed.py -q
"""

from __future__ import annotations

import datetime as dt
import json
import tomllib
from pathlib import Path

import caps
import layout
import ledger
import planemit
import provider
import run_roko
from test_run_roko import fake_roko

# The snapshot rows 3313's roko_ladder arm uses: cheap first, then mid and top, each on its own provider.
CHEAP, MID, TOP = "gpt-oss-120b", "glm-4.7", "gpt-5.4-mini"
OUTSIDE = "kimi-k2.6"  # a real model, but not one of this arm's rungs
PROVIDER_OF = {CHEAP: "cerebras", MID: "zai", TOP: "openai"}
RUNGS = tuple(planemit.Rung(name=m, model=m, provider=PROVIDER_OF[m], base_url=f"http://127.0.0.1:9/{PROVIDER_OF[m]}",
                            api_key_env=f"{PROVIDER_OF[m].upper()}_API_KEY") for m in (CHEAP, MID, TOP))
USAGE = {"tokens_in": 1000, "tokens_out": 50, "tokens_cache_read": 0}
LADDER_ARM = layout.ARMS_DIR / "roko_ladder.toml"


def verdict(model: str, *, failover_chain: tuple[str, ...] = ()) -> dict:
    provider = PROVIDER_OF[model]
    return {"executed": {"provider": provider, "model_requested": model, "model_dispatched": model,
                         "model_reported": model, "models_reported": [model], "model_mismatch": False,
                         "failover_chain": list(failover_chain), "turns": 1}, "usage": USAGE}


def evidence(models: list[str], verdicts: list[dict] | None = None) -> run_roko.Evidence:
    episodes = [{"task_id": "T01", "model": model, "backend": PROVIDER_OF.get(model, "cerebras"), "success": False,
                 "turns": 1, "completed_at": f"2026-09-29T15:00:0{number}Z", "extra": {"plan_id": "vb-x"}}
                for number, model in enumerate(models)]
    numbered = [{**row, "attempt": number} for number, row in enumerate(verdicts or [], 1)]
    return run_roko.Evidence(episodes=episodes, cost_rows=[], efficiency=[], verdicts=numbered, checkpoint=None,
                             proxy_rows=None)


def started(found: dict, at: str) -> dict:
    """The verdict `found`, with S01's attempt start at `at`, a time of day on 2026-09-29 (UTC)."""
    moment = dt.datetime.fromisoformat(f"2026-09-29T{at}+00:00")
    return {**found, "timing": {"attempt_started_at": int(moment.timestamp() * 1000)}}


def call(ordinal: int, at: str, model: str, status: int = 200) -> dict:
    """One proxy row: a request for `model` at `at` (a time of day on 2026-09-29, UTC), billed unless it failed."""
    billed = status == 200
    return {"ordinal": ordinal, "ts": f"2026-09-29T{at}Z", "model_requested": model, "status": status,
            "model_reported": model if billed else None, "usage_source": "reported" if billed else "none",
            "usage": USAGE if billed else None}


def settle(found: run_roko.Evidence) -> tuple[list[run_roko.RokoAttempt], list[str]]:
    return run_roko.settle(found, chain_key="run-1/F1-l1-0001.s1", model=CHEAP, provider=PROVIDER_OF[CHEAP],
                           snapshot=ledger.load_snapshot(), reserved_usd=0.1, max_attempts=3, rungs=RUNGS)


def test_routed_attempts_accept_escalation_and_reject_failover():
    # An escalation from the cheap rung to the mid rung is accepted, classed and priced on each attempt's own model.
    attempts, problems = settle(evidence([CHEAP, MID], [verdict(CHEAP), verdict(MID)]))
    assert problems == [] and [a.checks for a in attempts] == [[], []]
    assert [a.cost_class for a in attempts] == ["execute", "escalate"]
    assert [a.model_reported for a in attempts] == [CHEAP, MID]
    assert all(a.cost is not None and a.cost.api_equiv_usd and a.cost.api_equiv_usd > 0 for a in attempts)
    assert attempts[0].cost.api_equiv_usd != attempts[1].cost.api_equiv_usd  # the mid rung's own, higher rate

    # Climbing straight to the top rung, skipping the middle one, is still an escalation, never a mismatch.
    attempts, problems = settle(evidence([CHEAP, TOP], [verdict(CHEAP), verdict(TOP)]))
    assert problems == [] and [a.checks for a in attempts] == [[], []]
    assert attempts[1].cost.api_equiv_usd > attempts[0].cost.api_equiv_usd  # the top rung costs the most

    # A failover is a mismatch even onto another of the arm's own rungs: Roko's failover_chain gives it away.
    attempts, problems = settle(evidence([CHEAP, MID], [verdict(CHEAP), verdict(MID, failover_chain=(CHEAP,))]))
    assert problems and all(problem.startswith("model_mismatch") for problem in problems)
    assert attempts[1].checks == ["model_mismatch"]

    # A downward step is refused: the ladder only climbs.
    attempts, problems = settle(evidence([MID, CHEAP], [verdict(MID), verdict(CHEAP)]))
    assert problems and attempts[1].checks == ["model_mismatch"]
    assert attempts[0].checks == []  # the climb itself, mid then nothing lower yet, is fine

    # A model outside the arm's rungs is refused, exactly as a foreign model is for a pinned arm.
    attempts, problems = settle(evidence([CHEAP, OUTSIDE], [verdict(CHEAP)]))
    assert problems and attempts[1].checks == ["model_mismatch"]
    assert any(OUTSIDE in problem for problem in problems)


def test_a_routed_arm_runs_roko_without_a_model_pin(tmp_path):
    # bug-0b7695: `--model` would pin the start rung past the ladder (planemit's ladder mode), so Roko would never
    # escalate or fail over. A routed arm's preflight runs `plan validate` with no model on its command line, as its
    # tasks run `plan validate` and `plan run`; a pinned arm's command line still names its pin.
    binary, log = fake_roko(tmp_path, [CHEAP])
    text = LADDER_ARM.read_text()
    assert 'binary = "target/debug/roko"' in text
    arm = tomllib.loads(text.replace('binary = "target/debug/roko"', f"binary = {json.dumps(str(binary))}"))
    endpoint = provider.Endpoint(provider="cerebras", base_url="http://127.0.0.1:9/v1", api_key_env="CEREBRAS_API_KEY")
    run_roko.preflight(arm, CHEAP, endpoint, caps.Caps.from_table(arm["caps"]), ledger.load_snapshot())
    [preflight] = [json.loads(line)["argv"] for line in log.read_text().splitlines() if line.strip()]
    assert preflight[preflight.index("plan"):][:4] == ["plan", "validate", "--strict", "--dag"]
    assert "--model" not in preflight and "--no-serve" in preflight
    assert run_roko._head(Path("roko"), Path("ws"), CHEAP)[3:5] == ["--model", CHEAP]


def test_plan_start_probe_traffic_is_not_charged_to_the_first_attempt():
    # bug-0b7695: Roko probes each rung model once at plan start (backlog 1121), before its first attempt starts. Those
    # calls are not the first attempt's own: a probe need only name one of the arm's rungs, and each attempt keeps its
    # own calls and served model. The first attempt is still metered for them, each priced by its own model, so the
    # task's cost holds every billed call.
    probes = [call(1, "14:59:58.100000", CHEAP, status=500), call(2, "14:59:58.200000", MID),
              call(3, "14:59:58.300000", TOP)]
    own = [call(4, "14:59:59.900000", MID), call(5, "15:00:01.000000", TOP)]
    found = evidence([MID, TOP], [started(verdict(MID), "14:59:59.500"), started(verdict(TOP), "15:00:00.500")])
    found.proxy_rows = [*probes, *own]
    attempts, problems = settle(found)
    assert problems == [] and [a.checks for a in attempts] == [[], []]
    assert [(a.calls, a.model_reported) for a in attempts] == [(1, MID), (1, TOP)]
    usd = {served: ledger.price(USAGE, ledger.load_snapshot().row(served)).api_equiv_usd for served in (MID, TOP)}
    assert abs(attempts[0].cost.api_equiv_usd - (usd[MID] + usd[MID] + usd[TOP])) < 1e-12
    assert abs(attempts[1].cost.api_equiv_usd - usd[TOP]) < 1e-12
    assert attempts[0].as_record()["plan_start_calls"] == 3 and "plan_start_calls" not in attempts[1].as_record()

    # A plan-start call for a model outside the arm is still a mismatch, though no attempt's.
    found.proxy_rows = [call(0, "14:59:58.000000", OUTSIDE), *probes, *own]
    attempts, problems = settle(found)
    assert any("plan-start" in problem and OUTSIDE in problem for problem in problems), problems
    assert [a.checks for a in attempts] == [[], []]

    # Without S01's timing (an older Roko) nothing tells the probes apart: the first attempt owns them, as before.
    found = evidence([MID, TOP], [verdict(MID), verdict(TOP)])
    found.proxy_rows = [*probes, *own]
    attempts, problems = settle(found)
    assert attempts[0].checks == ["model_mismatch"] and attempts[1].checks == []
