"""Offline tests of the Roko arm's routed model check (3312): `run_roko.settle` accepts a ladder's escalations, in
rung order only, and still rejects a failover, a downward step and a model outside the arm's rungs.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_routed.py -q
"""

from __future__ import annotations

import ledger
import planemit
import run_roko

# The snapshot rows 3313's roko_ladder arm uses: cheap first, then mid and top, each on its own provider.
CHEAP, MID, TOP = "gpt-oss-120b", "glm-4.7", "gpt-5.4-mini"
OUTSIDE = "kimi-k2.6"  # a real model, but not one of this arm's rungs
PROVIDER_OF = {CHEAP: "cerebras", MID: "zai", TOP: "openai"}
RUNGS = tuple(planemit.Rung(name=m, model=m, provider=PROVIDER_OF[m], base_url=f"http://127.0.0.1:9/{PROVIDER_OF[m]}",
                            api_key_env=f"{PROVIDER_OF[m].upper()}_API_KEY") for m in (CHEAP, MID, TOP))
USAGE = {"tokens_in": 1000, "tokens_out": 50, "tokens_cache_read": 0}


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
