#!/usr/bin/env python3
"""Tests for `common/toolchain` and the environment a sandboxed check gets from it (gap-46fd19, task 3324).

Offline, and no real cargo runs: each test builds a fake rustup installation (a `rustup` script that answers
`show home` and `which cargo`, its `cargo` proxy beside it, and a toolchain directory holding the real binaries).

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/common/test_toolchain.py
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common import sandbox, toolchain  # noqa: E402

SYSTEM_PATH = ["/usr/bin", "/bin"]
RUSTUP_SCRIPT = """#!/bin/sh
echo "$PWD $*" >> "{log}"
case "$1 $2" in
  "show home") echo "{rustup_home}" ;;
  "which cargo") echo "{bin_dir}/cargo" ;;
  *) exit 1 ;;
esac
"""


def executable(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(0o755)
    return path


@pytest.fixture
def rustup_install(tmp_path: Path) -> dict:
    """A rustup installation as `rustup-init` lays it out, under made-up homes in `tmp_path`."""
    root = tmp_path.resolve()
    places = {"cargo_home": root / "operator" / ".cargo", "rustup_home": root / "operator" / ".rustup",
              "log": root / "rustup.log"}
    places["bin_dir"] = places["rustup_home"] / "toolchains" / "stable-test" / "bin"
    for tool in ("cargo", "rustc"):
        executable(places["bin_dir"] / tool, f"#!/bin/sh\necho {tool} from the toolchain\n")
    (places["rustup_home"] / "settings.toml").write_text('default_toolchain = "stable-test"\n')
    rustup = executable(places["cargo_home"] / "bin" / "rustup", RUSTUP_SCRIPT.format(**places))
    (places["cargo_home"] / "bin" / "cargo").symlink_to(rustup)  # rustup's proxies are links to rustup itself
    places["environ"] = {"PATH": os.pathsep.join([str(places["cargo_home"] / "bin"), *SYSTEM_PATH]),
                         "HOME": str(root / "operator")}
    return places


def test_find_resolves_the_real_toolchain_through_rustup(rustup_install):
    found = toolchain.find(rustup_install["environ"])
    assert found == toolchain.Toolchain(bin_dir=rustup_install["bin_dir"], rustup_home=rustup_install["rustup_home"],
                                        cargo_home=rustup_install["cargo_home"])
    # `rustup which` ran from `/`, where no rust-toolchain file or directory override applies.
    assert {line.split(" ", 1)[0] for line in rustup_install["log"].read_text().splitlines()} == {"/"}
    assert toolchain.find(rustup_install["environ"]) is found  # resolved once per environment


def test_a_run_gets_the_real_rustup_home_and_a_cargo_home_of_its_own(rustup_install, tmp_path):
    found = toolchain.find(rustup_install["environ"])
    home = tmp_path / "run-home"
    assert found.env(home) == {"RUSTUP_HOME": str(rustup_install["rustup_home"]), "CARGO_HOME": str(home / ".cargo")}
    # A process that only has the run's environment still finds the toolchain, from PATH and RUSTUP_HOME, though
    # not the operator's CARGO_HOME: `read_only` then falls back to the default homes in the real home directory.
    agent = {"PATH": os.pathsep.join([str(found.bin_dir), *SYSTEM_PATH]), "HOME": str(home), **found.env(home)}
    assert toolchain.find(agent) == toolchain.Toolchain(bin_dir=found.bin_dir, rustup_home=found.rustup_home,
                                                        cargo_home=None)
    assert found.rustup_home in toolchain.read_only(agent)
    assert {found.rustup_home, found.cargo_home} <= set(toolchain.read_only(rustup_install["environ"]))


def test_without_rustup_the_toolchain_is_where_cargo_resolves(tmp_path):
    # A packaged toolchain: a shared bin directory holds a link to cargo in the toolchain's own directory.
    root = tmp_path.resolve()
    cargo = executable(root / "opt" / "rust-1.90" / "bin" / "cargo", "#!/bin/sh\n")
    (root / "opt" / "bin").mkdir()
    (root / "opt" / "bin" / "cargo").symlink_to(cargo)
    found = toolchain.find({"PATH": os.pathsep.join([str(root / "opt" / "bin"), *SYSTEM_PATH])})
    assert found == toolchain.Toolchain(bin_dir=cargo.parent, rustup_home=None, cargo_home=None)
    assert found.env(root / "home") == {"CARGO_HOME": str(root / "home" / ".cargo")}
    assert toolchain.find({"PATH": str(root / "nothing-here")}) is None


def test_the_sandbox_keeps_the_toolchain_read_only_by_default(rustup_install, tmp_path):
    found = toolchain.find(rustup_install["environ"])
    kept = toolchain.read_only(rustup_install["environ"])
    profile = sandbox.profile([tmp_path / "denied"], None, (), kept)
    for path in (found.rustup_home, found.cargo_home):
        assert f'(deny file-write* (subpath "{os.path.realpath(path)}"))' in profile
    assert sandbox.profile([tmp_path / "denied"]) == '(version 1) (allow default) (deny file* (subpath "{}"))'.format(
        os.path.realpath(tmp_path / "denied"))
    argv = ["true"]
    if sandbox.KIND == "sandbox-exec":  # by default `command` keeps this host's toolchain read-only
        assert sandbox.command(argv, deny=[tmp_path / "denied"]) == [
            sandbox.SANDBOX_EXEC, "-p", sandbox.profile([tmp_path / "denied"], None, (), toolchain.read_only()), "true"]
    assert sandbox.command(argv, deny=()) == argv  # read-only paths alone apply no sandbox


@pytest.mark.skipif(sandbox.KIND != "sandbox-exec", reason="no file sandbox on this host (common/sandbox)")
def test_sandboxed_code_runs_the_toolchain_but_cannot_change_it(rustup_install, tmp_path):
    """The environment a sandboxed check gets: the toolchain's own cargo first on PATH, the real RUSTUP_HOME it may
    read and not write, the operator's CARGO_HOME it may not write, and a CARGO_HOME of its own that it may."""
    found = toolchain.find(rustup_install["environ"])
    home = tmp_path / "run-home"
    home.mkdir()
    env = {"PATH": os.pathsep.join([str(found.bin_dir), *SYSTEM_PATH]), "HOME": str(home), **found.env(home)}
    probe = ('command -v cargo; cat "$RUSTUP_HOME/settings.toml"; '
             'touch "$RUSTUP_HOME/settings.toml" 2>/dev/null || echo "rustup home: refused"; '
             f'touch "{found.cargo_home}/config.toml" 2>/dev/null || echo "operator cargo home: refused"; '
             'mkdir -p "$CARGO_HOME/registry" && touch "$CARGO_HOME/registry/index" && echo "own cargo home: written"')
    argv = sandbox.command(["sh", "-c", probe], deny=[tmp_path / "denied"],
                           read_only=toolchain.read_only(rustup_install["environ"]))
    run = subprocess.run(argv, env=env, capture_output=True, text=True, check=False)
    assert run.stdout.splitlines() == [str(found.bin_dir / "cargo"), 'default_toolchain = "stable-test"',
                                       "rustup home: refused", "operator cargo home: refused",
                                       "own cargo home: written"]
    assert not (found.cargo_home / "config.toml").exists() and (home / ".cargo" / "registry" / "index").is_file()
    # Control: unconfined, the same writes go through, so the refusals above are the sandbox's.
    assert "refused" not in subprocess.run(["sh", "-c", probe], env=env, capture_output=True, text=True).stdout
