"""Offline tests of vb.py's multi-provider support (3311): a routed arm (several models_allow) gets one metering
proxy upstream per provider, priced conservatively at its most expensive rung, and refuses to start without every
provider's key. A one-model arm is unchanged.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_vb_providers.py -q
"""

from __future__ import annotations

import argparse
from pathlib import Path

import pytest

import caps
import layout
import ledger
import secret
import vb

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
CHEAP, MID, TOP = "gpt-oss-120b", "glm-4.7", "gpt-5.4-mini"  # cerebras, zai, openai rows in the price snapshot
LADDER_ARM = """\
schema_version = "vb.arm/1"

[arm]
id = "ladder_test"
harness = "roko"
runner = "run_roko"
models_allow = ["gpt-oss-120b", "glm-4.7", "gpt-5.4-mini"]
line = "BL14"
billed = true
label = "ladder·test"

[caps]
usd_per_task = 0.60

[roko]
binary = "target/debug/roko"
provider_kind = "openai_compat"

[providers.cerebras]
base_url = "http://127.0.0.1:9/cerebras"
api_key_env = "CEREBRAS_API_KEY"

[providers.zai]
base_url = "http://127.0.0.1:9/zai"
api_key_env = "ZAI_API_KEY"

[providers.openai]
base_url = "http://127.0.0.1:9/openai"
api_key_env = "OPENAI_API_KEY"
"""


@pytest.fixture
def ladder_arm(tmp_path: Path) -> Path:
    path = tmp_path / "ladder_test.toml"
    path.write_text(LADDER_ARM)
    return path


def make_args(arm: Path | str, **extra: object) -> argparse.Namespace:
    fields = {"arm": str(arm), "stream": TOY_STREAM, "model": CHEAP, "seeds": "1", "limit": None,
              "price_snapshot": ledger.DEFAULT_SNAPSHOT, "provider_url": None}
    return argparse.Namespace(**{**fields, **extra})


def test_multi_provider_arm_gets_one_upstream_per_provider(tmp_path, ladder_arm):
    plan = vb.make_plan(make_args(ladder_arm))
    assert set(plan.endpoints) == {"cerebras", "zai", "openai"}
    assert plan.endpoints["cerebras"].base_url == "http://127.0.0.1:9/cerebras"
    assert plan.endpoints["zai"].api_key_env == "ZAI_API_KEY"
    assert plan.endpoints["openai"].api_key_env == "OPENAI_API_KEY"
    # Admission and reservations price at the most expensive rung (gpt-5.4-mini), not the cheap one --model picked.
    assert plan.worst_task_usd == pytest.approx(caps.worst_task_usd(plan.caps, plan.snapshot.row(TOP)))
    assert plan.worst_task_usd > caps.worst_task_usd(plan.caps, plan.snapshot.row(CHEAP))

    full = secret.create_keys(tmp_path / "keys", {"CEREBRAS_API_KEY": "sk-cerebras-0000000000",
                                                  "ZAI_API_KEY": "sk-zai-000000000000000",
                                                  "OPENAI_API_KEY": "sk-openai-00000000000000"})
    run_dir = tmp_path / "run"
    run_dir.mkdir()
    proxy = vb._start_proxy(plan, run_dir, keys=vb._provider_keys(full, plan.endpoints.values()))
    try:
        assert set(proxy.upstreams) == {"cerebras", "zai", "openai"}
        for name in ("cerebras", "zai", "openai"):
            assert proxy.base_url(name).startswith(proxy.url + "/")
    finally:
        proxy.close()

    # Without every provider's key, the key lookup (and so the run) refuses to start before anything else does.
    partial = secret.create_keys(tmp_path / "partial-keys", {"CEREBRAS_API_KEY": "sk-cerebras-0000000000",
                                                              "ZAI_API_KEY": "sk-zai-000000000000000"})
    with pytest.raises(vb.DriverError, match="OPENAI_API_KEY"):
        vb._provider_keys(partial, plan.endpoints.values())

    # A one-model arm behaves exactly as before: one endpoint, equal to the plan's own, priced the same way.
    single = vb.make_plan(make_args(layout.ARMS_DIR / "cheap_direct.toml", model=CHEAP))
    assert list(single.endpoints) == [single.endpoint.provider] and single.endpoints[single.endpoint.provider] == \
        single.endpoint
    assert single.worst_task_usd == caps.worst_task_usd(single.caps, single.snapshot.row(CHEAP))
