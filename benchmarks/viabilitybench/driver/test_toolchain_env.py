"""The Rust toolchain in a real run's agent environment (gap-46fd19, task 3324; Will's decision of 2026-10-02).

`agent_env.build` gives every agent process, and the census, a HOME of its own. Before this change it also left
the toolchain off PATH, so an F7 task's `cargo test` could not run at all. These tests pin what the environment now
carries, with a made-up toolchain, and check on this host that the environment finds the real cargo.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_toolchain_env.py
"""

from __future__ import annotations

import os
import shutil
from pathlib import Path

import pytest

import agent_env
from common import toolchain


def made_up(root: Path) -> toolchain.Toolchain:
    return toolchain.Toolchain(bin_dir=root / "rustup" / "toolchains" / "stable" / "bin", rustup_home=root / "rustup",
                               cargo_home=root / "operator-cargo")


def test_agent_env_carries_the_toolchain_and_a_cargo_home_of_its_own(tmp_path, monkeypatch):
    rust = made_up(tmp_path)
    monkeypatch.setattr(agent_env.toolchain, "find", lambda environ=None: rust)
    home = tmp_path / "home"
    env = agent_env.build(home=home)
    # The toolchain's own bin directory, not the operator's ~/.cargo/bin with whatever `cargo install` put there.
    assert env["PATH"].split(os.pathsep) == [str(home / ".vb-bin"), str(rust.bin_dir), *agent_env.SYSTEM_PATH]
    assert (env["RUSTUP_HOME"], env["CARGO_HOME"]) == (str(rust.rustup_home), str(home / ".cargo"))
    assert str(rust.cargo_home) not in "".join(env.values())  # the registry cache is the run's own
    agent_env.check(env)
    monkeypatch.setattr(agent_env.toolchain, "find", lambda environ=None: None)
    bare = agent_env.build(home=tmp_path / "bare")
    assert bare["PATH"].split(os.pathsep) == [str(tmp_path / "bare" / ".vb-bin"), *agent_env.SYSTEM_PATH]
    assert {"RUSTUP_HOME", "CARGO_HOME"}.isdisjoint(bare)


def test_the_driver_keeps_a_custom_toolchain_location_through_its_scrub():
    kept = agent_env.driver_env({"PATH": "/bin", "RUSTUP_HOME": "/opt/rustup", "CARGO_HOME": "/opt/cargo",
                                 "RUSTUP_TOKEN": "x" * 20})
    assert kept == {"PATH": "/bin", "RUSTUP_HOME": "/opt/rustup", "CARGO_HOME": "/opt/cargo",
                    agent_env.DRIVER_SCRUBBED: "scrubbed"}


def test_on_this_host_the_agent_env_finds_the_real_cargo(tmp_path):
    rust = toolchain.find()
    if rust is None:
        pytest.skip("no Rust toolchain on this host")
    env = agent_env.build(home=tmp_path / "home")
    cargo = shutil.which("cargo", path=env["PATH"])
    assert cargo is not None and Path(cargo).parent == rust.bin_dir
    if rust.rustup_home is not None:  # the real binary, inside RUSTUP_HOME, not a proxy that would need ~/.rustup
        assert Path(cargo).resolve().is_relative_to(rust.rustup_home.resolve())
