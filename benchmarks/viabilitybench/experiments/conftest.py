"""Shared setup for the experiment manifests' tests: the driver's modules on `sys.path`, as `driver/`'s own tests have
them, and no provider key or key file inherited from the shell that runs pytest (`driver/conftest.py` has why)."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "driver"))

import secret  # noqa: E402 (driver/secret.py, on sys.path from the line above)


@pytest.fixture(autouse=True)
def no_inherited_provider_keys(monkeypatch, tmp_path_factory):
    for name in secret.arm_key_names():
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv(secret.KEYS_FILE_ENV, str(tmp_path_factory.getbasetemp() / "no-key-file"))
