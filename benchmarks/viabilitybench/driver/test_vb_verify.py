"""Tests of the visible-verify wrapper (`vb_verify.py`, gap-4e8795) and of the `flaky_verify` disturbance it serves.

The driver tests run it inside `vb run` for each arm (test_driver, test_run_cli, test_run_roko); these check the
wrapper alone. Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_vb_verify.py -q
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

import disturb
import vb_verify

KEY = "F4-l1-0001.s1"
VISIBLE = "sh tests/visible/run.sh"
BASH = shutil.which("bash") or "/bin/bash"


def installed(tmp_path: Path, *, p: float, seed: int = 7, name: str = "task") -> tuple[Path, Path]:
    """A workdir whose visible check passes, and the wrapper for it; returns (workdir, wrapper)."""
    workdir, bin_dir = tmp_path / name / "work", tmp_path / name / ".vb-bin"
    (workdir / "tests/visible").mkdir(parents=True)
    (workdir / "tests/visible/run.sh").write_text("echo PASS\n")
    bin_dir.mkdir()
    return workdir, vb_verify.install(bin_dir, key=KEY, visible=[VISIBLE], p=p, seed=seed, shell=BASH)


def run(wrapper: Path, workdir: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([str(wrapper), *args], cwd=workdir, capture_output=True, text=True, timeout=30,
                          check=False)


def test_the_wrapper_runs_commands_as_bash_would(tmp_path):
    workdir, wrapper = installed(tmp_path, p=1.0)
    done = run(wrapper, workdir, 'echo "$0 $1" && echo oops >&2 && exit 3', "zero", "one")
    assert (done.returncode, done.stdout, done.stderr) == (3, "zero one\n", "oops\n")
    assert run(wrapper, workdir, "bash tests/visible/run.sh").stdout == "PASS\n", "not the check as the spec states it"
    assert vb_verify.read_log(wrapper) == [], "only visible check runs are logged"
    assert run(wrapper, workdir).returncode == 2  # no command: a usage error


def test_visible_check_runs_flake_by_a_seeded_draw(tmp_path):
    workdir, wrapper = installed(tmp_path, p=0.5)
    codes = [run(wrapper, workdir, f"{VISIBLE} && echo done").returncode for _ in range(8)]
    expected = [vb_verify.flaky(7, KEY, number, 0.5) for number in range(1, 9)]
    assert codes == [vb_verify.FLAKE_STATUS if flake else 0 for flake in expected]
    assert True in expected and False in expected, "p = 0.5 over eight runs draws both outcomes for this seed"
    log = vb_verify.read_log(wrapper)
    assert [(row["run"], row["flake"], row["check"]) for row in log] == [
        (number, flake, VISIBLE) for number, flake in enumerate(expected, 1)]
    # A replay of the same spec injects the same flakes; another seed draws others.
    again, other = installed(tmp_path, p=0.5, name="again"), installed(tmp_path, p=0.5, seed=8, name="other")
    assert [run(again[1], again[0], VISIBLE).returncode for _ in range(8)] == codes
    assert [vb_verify.flaky(8, KEY, number, 0.5) for number in range(1, 9)] != expected
    assert [run(other[1], other[0], VISIBLE).returncode for _ in range(8)] == [
        vb_verify.FLAKE_STATUS if vb_verify.flaky(8, KEY, number, 0.5) else 0 for number in range(1, 9)]


def test_a_flake_looks_like_a_killed_check_and_p_0_only_counts(tmp_path):
    workdir, wrapper = installed(tmp_path, p=1.0)
    flaked = run(wrapper, workdir, f"touch ran && {VISIBLE}")
    assert (flaked.returncode, flaked.stdout, flaked.stderr) == (137, "", "Killed\n")
    assert not (workdir / "ran").exists(), "a flaked command runs nothing"
    workdir, wrapper = installed(tmp_path, p=0.0, name="counted")
    assert [run(wrapper, workdir, VISIBLE).stdout for _ in range(3)] == ["PASS\n"] * 3
    assert [(row["run"], row["flake"]) for row in vb_verify.read_log(wrapper)] == [(1, False), (2, False), (3, False)]


@pytest.mark.parametrize("command, found", [
    ("sh tests/visible/run.sh", True), ("cd . && sh tests/visible/run.sh; echo $?", True),
    ("(sh tests/visible/run.sh)", True), ("eval 'sh tests/visible/run.sh' < /dev/null", True),
    ("sh tests/visible/run.sh>out.txt", True), ("bash tests/visible/run.sh", False),
    ("sh tests/visible/run.sh.bak", False), ("cat tests/visible/run.sh", False), ("", False)])
def test_a_visible_check_run_holds_the_command_as_whole_words(command, found):
    assert (vb_verify.visible_run(command, ["", VISIBLE]) == VISIBLE) is found


def test_the_wrapper_fails_open(tmp_path):
    workdir, wrapper = installed(tmp_path, p=1.0)
    config = wrapper.with_name(vb_verify.CONFIG)
    log = wrapper.with_name(vb_verify.LOG)
    log.unlink()
    log.mkdir()  # a log it cannot append to: the run is not recorded, so it cannot flake
    assert run(wrapper, workdir, VISIBLE).stdout == "PASS\n"
    config.write_text("not json")  # no config: it still runs the command
    assert run(wrapper, workdir, VISIBLE).stdout == "PASS\n"
    assert vb_verify.read_log(wrapper) is None


def test_the_flaky_verify_disturbance_names_p_and_its_seed(tmp_path):
    spec = tmp_path / "flaky.toml"
    spec.write_text('schema_version = "vb.disturbance/1"\n\n[[disturbance]]\nkind = "flaky_verify"\nstart_at = 3\n'
                    "seed = 5\n")
    [flaky] = disturb.load(spec)
    assert flaky.params == {"p": 0.25}, "S06 §4.9's default"
    assert [disturb.flake([flaky], position) for position in (2, 3, 9)] == [(0.0, 0), (0.25, 5), (0.25, 5)]
    assert disturb.kinds([flaky], 3) == ["flaky_verify"]
    for bad in ("params = { p = 1.5 }", "params = { p = true }", "params = { rate = 0.1 }"):
        spec.write_text(f'schema_version = "vb.disturbance/1"\n\n[[disturbance]]\nkind = "flaky_verify"\n{bad}\n')
        with pytest.raises(disturb.DisturbanceError):
            disturb.load(spec)
