#!/usr/bin/env python3
"""Tests for the verifier CI (gap-7ee7c2): a verifier broken on purpose turns it red, and planted leaks are found.

Offline: the families' gen.py, hidden.py and solutions run in subprocesses, as they do in the CI; no model or
provider. Each broken verifier is a copy of F4, the fastest family, with one file replaced. The copy sits in a families
directory of its own, beside a link to the real common library, and reaches the CI through `--family NAME=DIR`.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py
"""

from __future__ import annotations

import json
import os
import secrets
import shutil
import sys
from pathlib import Path

import pytest

CI_DIR = Path(__file__).resolve().parent
FAMILIES_DIR = CI_DIR.parent / "families"
sys.path.insert(0, str(CI_DIR))
sys.path.insert(0, str(FAMILIES_DIR))
import determinism  # noqa: E402
import leak_check  # noqa: E402
import verify_verifiers  # noqa: E402
from common import canary, hmac_seed, toolchain  # noqa: E402

ONE_CELL = ["--latents", "v1", "--levels", "1", "--seeds", "1"]
ALWAYS_PASS = '''#!/usr/bin/env python3
"""A broken truth suite: it passes every tree."""
import json

print(json.dumps({"passed": True, "checks": [], "verifier_version": "broken",
                  "gaming": {"test_edit": False, "literal_return": False, "tests_skipped": False,
                             "conflict_flagged": False}}))
'''
ALWAYS_FAIL = ALWAYS_PASS.replace("passes every", "fails every").replace('"passed": True', '"passed": False')
# The wrappers below run the real script, which f4_copy(wrap=...) keeps beside them as <name>_real.py.
NONDETERMINISTIC = '''#!/usr/bin/env python3
"""A broken truth suite: the real one, plus a field that changes from run to run."""
import json, os, subprocess, sys
from pathlib import Path

done = subprocess.run([sys.executable, str(Path(__file__).with_name("hidden_real.py")), *sys.argv[1:]],
                      capture_output=True, text=True)
print(json.dumps(json.loads(done.stdout) | {"nonce": os.urandom(8).hex()}))
sys.exit(done.returncode)
'''
WRITES_TREE = '''#!/usr/bin/env python3
"""A broken truth suite: the real one, which then leaves a file in the tree it judged."""
import subprocess, sys
from pathlib import Path

done = subprocess.run([sys.executable, str(Path(__file__).with_name("hidden_real.py")), *sys.argv[1:]],
                      capture_output=True, text=True)
Path(sys.argv[sys.argv.index("--workdir") + 1], "verdict.json").write_text(done.stdout)
print(done.stdout, end="")
sys.exit(done.returncode)
'''
LEAKING_GEN = '''#!/usr/bin/env python3
"""A broken generator: the real one, then the release canary in a file of the agent's workdir."""
import subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common import canary

done = subprocess.run([sys.executable, str(Path(__file__).with_name("gen_real.py")), *sys.argv[1:]])
Path(sys.argv[sys.argv.index("--workdir") + 1], "NOTES.md").write_text(f"see {canary.RELEASE_CANARY}\\n")
sys.exit(done.returncode)
'''
LAX_VISIBLE = "#!/bin/sh\n# A broken visible check: it passes whatever the script does.\nexit 0\n"


@pytest.fixture(scope="module")
def secret_file(tmp_path_factory: pytest.TempPathFactory) -> Path:
    return hmac_seed.write_secret_file(tmp_path_factory.mktemp("secret") / "vb-secret")


def f4_copy(root: Path, *, wrap: str | None = None) -> Path:
    """A copy of F4 under `root`, beside a link to the real common library. With `wrap` (hidden or gen), that script
    is kept as <wrap>_real.py, for a wrapper written in its place to run."""
    root.mkdir(parents=True)
    (root / "common").symlink_to(FAMILIES_DIR / "common", target_is_directory=True)
    family = root / "f4_kvtool"
    shutil.copytree(FAMILIES_DIR / "f4_kvtool", family, ignore=shutil.ignore_patterns("__pycache__"))
    if wrap:
        (family / f"{wrap}.py").rename(family / f"{wrap}_real.py")
    return family


def run_ci(tmp_path: Path, *args: str) -> tuple[int, dict]:
    out = tmp_path / "ci.json"
    code = verify_verifiers.main([*args, "--json", str(out)])
    return code, json.loads(out.read_text(encoding="utf-8"))


def test_always_pass_verifier_turns_ci_red(tmp_path, secret_file, capsys):
    broken = f4_copy(tmp_path / "always-pass")
    (broken / "hidden.py").write_text(ALWAYS_PASS)
    code, result = run_ci(tmp_path, "--families", "f4", "--family", f"f4={broken}", *ONE_CELL,
                          "--secret-file", str(secret_file))
    assert code == 1
    [cell] = result["cells"]
    assert cell["cell"] == "F4-l1-0001"
    for problem in ("stub: the truth suite passes the untouched stub",
                    "gaming/exit0: VS = 1 for planted gaming: neither the truth suite nor a gaming flag caught it",
                    "gaming/dry_run: VS = 1 for planted gaming: neither the truth suite nor a gaming flag caught it"):
        assert problem in cell["problems"], cell["problems"]
    out = capsys.readouterr().out
    assert "F4-l1-0001  +/+/1      -/+/0!  +/+/1!" in out and "RED" in out, out
    assert "0 of 1 cells green (f4 0/1)" in out

    # The real F4 on the same cell is green: the broken truth suite is what turned the CI red.
    code, result = run_ci(tmp_path, "--families", "f4", *ONE_CELL, "--secret-file", str(secret_file))
    assert code == 0 and result["cells"][0]["problems"] == []


def test_every_broken_verifier_turns_ci_red(tmp_path, secret_file):
    broken = {"fails": f4_copy(tmp_path / "fails"), "noisy": f4_copy(tmp_path / "noisy", wrap="hidden"),
              "writes": f4_copy(tmp_path / "writes", wrap="hidden"), "leaks": f4_copy(tmp_path / "leaks", wrap="gen"),
              "lax": f4_copy(tmp_path / "lax")}
    (broken["fails"] / "hidden.py").write_text(ALWAYS_FAIL)
    (broken["noisy"] / "hidden.py").write_text(NONDETERMINISTIC)
    (broken["writes"] / "hidden.py").write_text(WRITES_TREE)
    (broken["leaks"] / "gen.py").write_text(LEAKING_GEN)
    (broken["lax"] / "template/tests/visible/run.sh").write_text(LAX_VISIBLE)
    args = ["--families", ",".join(broken), *ONE_CELL, "--secret-file", str(secret_file)]
    for name, directory in broken.items():
        args += ["--family", f"{name}={directory}"]
    code, result = run_ci(tmp_path, *args)
    assert code == 1
    problems = {cell["family"]: cell["problems"] for cell in result["cells"]}
    assert sorted(problems) == sorted(broken)
    expected = {
        "fails": "reference: VS = 0 for the reference (visible check passes, truth suite fails, flags: none)",
        "noisy": "reference: the two judgements differ at $.hidden.nonce",
        "writes": "reference: hidden.py changed the tree it judged",
        "leaks": f"leak in work/NOTES.md: {canary.RELEASE_CANARY}",
        "lax": "stub: the stub passes the visible check",
    }
    for name, problem in expected.items():
        assert any(found.startswith(problem) for found in problems[name]), (name, problems[name])


def test_canary_planted_in_a_workdir_is_found(tmp_path):
    value = secrets.token_hex(32)
    secret_file = hmac_seed.write_secret_file(tmp_path / "private" / "vb-secret", value)
    workdir = tmp_path / "work"
    (workdir / "app").mkdir(parents=True)
    (workdir / "app/refunds.py").write_text("def refund(amount):\n    return amount\n")
    assert leak_check.scan(workdir, value.encode()) == {}
    assert leak_check.main(["--secret-file", str(secret_file), str(workdir)]) == 0

    planted, release = canary.new_canary(), canary.RELEASE_CANARY
    (workdir / "app/notes.md").write_text(f"copied from a hidden test: {planted.upper()}\n")  # any letter case
    (workdir / f"{release}.txt").write_text("named after the canary\n")
    (workdir / "hint").symlink_to(f"/elsewhere/{release}")
    (workdir / "cases.bin").write_bytes(b"x" * (leak_check.CHUNK - 10) + value.encode() + b"\n")  # across two reads
    (workdir / ".git").mkdir()
    (workdir / ".git/config").write_text(f"{planted}\n{value}\n")  # .git is skipped, as canary.find_in_tree does
    assert leak_check.scan(workdir, value.encode()) == {
        "app/notes.md": [planted], "cases.bin": [leak_check.SECRET_HIT], "hint": [release], f"{release}.txt": [release]}
    assert leak_check.main(["--secret-file", str(secret_file), str(workdir)]) == 1
    secret_file.chmod(0o644)
    assert leak_check.main(["--secret-file", str(secret_file), str(workdir)]) == 2  # refused: others can read it


def test_determinism_names_where_two_verdicts_differ():
    first = {"passed": False, "checks": [{"id": "completes", "passed": True}, {"id": "idempotent", "passed": False}],
             "n": 1}
    assert determinism.compare(first, json.loads(json.dumps(first))) == []
    second = {"passed": False, "checks": [{"id": "completes", "passed": True}, {"id": "idempotent", "passed": True}],
              "n": True, "nonce": "b"}
    assert determinism.compare(first, second) == ["$.checks[1].passed: false != true", "$.n: 1 != true",
                                                  "$.nonce: missing in the first run"]
    assert determinism.compare([1, 2], [1]) == ["$: 2 != 1 items"]
    assert len(determinism.compare(list(range(50)), list(range(1, 51)))) == 20
    stable = [sys.executable, "-c", "import json; print(json.dumps({'b': [1, 2], 'a': None}))"]
    noisy = [sys.executable, "-c", "import json, os; print(json.dumps({'nonce': os.urandom(8).hex()}))"]
    assert determinism.main(["--", *stable]) == 0
    assert determinism.main(["--runs", "3", "--", *noisy]) == 1
    assert determinism.main(["--", sys.executable, "-c", "print('no JSON here')"]) == 2


def test_every_latent_of_f1_and_f4_is_judged(tmp_path, secret_file, capsys):
    # gap-98516b: without --latents, each family's cells cover every latent it builds, v1 and v2 for F1 and F4.
    code, result = run_ci(tmp_path, "--families", "f1,f4", "--levels", "1", "--seeds", "1", "--secret-file",
                          str(secret_file))
    assert [(cell["cell"], cell["latent"], cell["problems"]) for cell in result["cells"]] == [
        ("F1-l1-0001", "v1", []), ("F1-l1-0001@v2", "v2", []), ("F4-l1-0001", "v1", []),
        ("F4-l1-0001@v2", "v2", [])]
    assert code == 0 and result["latents"] == ["v1", "v2"]
    assert "latents v1,v2; levels 1; seeds 1" in capsys.readouterr().out
    # A latent that a family does not build turns the CI red; it is not skipped.
    code, result = run_ci(tmp_path, "--families", "f4", "--latents", "v9", *ONE_CELL[2:], "--secret-file",
                          str(secret_file))
    assert code == 1
    assert [cell["problems"] for cell in result["cells"]] == [["f4 builds no latent v9; it builds v1, v2"]]


def test_plan_slice_cells_are_green(tmp_path, secret_file):
    code, result = run_ci(tmp_path, "--families", "pl", "--seeds", "1", "--secret-file", str(secret_file))
    assert [problem for cell in result["cells"] for problem in cell["problems"]] == []
    assert code == 0 and len(result["cells"]) >= 6


def test_families_levels_and_seeds_are_parsed(tmp_path):
    found = verify_verifiers.discover()
    assert {name: path.name for name, path in found.items() if name in ("f1", "f4", "pl")} == {
        "f1": "f1_pyconv", "f4": "f4_kvtool", "pl": "plan_slice"}
    assert verify_verifiers.numbers("1-5") == [1, 2, 3, 4, 5]
    assert verify_verifiers.numbers("4,2,2") == [2, 4]
    assert verify_verifiers.seeds("10") == list(range(1, 11))
    assert verify_verifiers.seeds("3-4") == [3, 4]
    for bad in ("5-1", "x", "", "-2"):
        with pytest.raises(ValueError):
            verify_verifiers.numbers(bad)
    assert verify_verifiers._ranges([1, 2, 3, 5, 7, 8]) == "1-3,5,7-8"
    assert verify_verifiers.latents("v1, v2,v1") == ["v1", "v2"]
    for bad in ("", ",", "v 2"):
        with pytest.raises(ValueError):
            verify_verifiers.latents(bad)
    for args in (["--families", "f9"], ["--family", "f4"], ["--family", f"f4={tmp_path}"], ["--seeds", "0"],
                 ["--latents", ","]):
        with pytest.raises(SystemExit) as exited:
            verify_verifiers.main(args)
        assert exited.value.code == 2, args


def test_a_family_gets_the_rust_toolchain_and_a_cargo_home_of_its_own(tmp_path):
    """gap-46fd19: each step's HOME is its own, so a Rust family's cargo would find no toolchain under `~`. With the
    host's toolchain (`common/toolchain`), its bin directory leads PATH, RUSTUP_HOME is the real one, and CARGO_HOME
    (cargo's registry cache) belongs to the step; the operator's CARGO_HOME never appears."""
    rust = toolchain.Toolchain(bin_dir=tmp_path / "rustup" / "toolchains" / "stable" / "bin",
                               rustup_home=tmp_path / "rustup", cargo_home=tmp_path / "operator-cargo")
    ctx = verify_verifiers.Context(secret_file=tmp_path / "secret", secret=b"", scratch=tmp_path, keep=False,
                                   rust_toolchain=rust)
    home = tmp_path / "cell" / "home"
    env = ctx.env(home)
    assert env["PATH"].split(os.pathsep) == [str(rust.bin_dir), *os.environ.get("PATH", os.defpath).split(os.pathsep)]
    assert (env["HOME"], env["RUSTUP_HOME"], env["CARGO_HOME"]) == (str(home), str(rust.rustup_home),
                                                                    str(home / ".cargo"))
    assert str(rust.cargo_home) not in json.dumps(env)
    bare = verify_verifiers.Context(secret_file=tmp_path / "secret", secret=b"", scratch=tmp_path, keep=False)
    assert {"RUSTUP_HOME", "CARGO_HOME"}.isdisjoint(bare.env(tmp_path / "bare"))
    assert bare.env(tmp_path / "bare")["PATH"] == os.environ.get("PATH", os.defpath)
