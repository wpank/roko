"""Offline tests that the benchmark secret stays driver-only (gap-a8a160, S08 SC4): adversarial fake agents on a local
stub server, the toy family, no model calls.

The fake agents try to read the secret through their environment, by searching the files around their workdir, and
through `ps -E` or `/proc` on the driver and on `hidden.py`, whose child the agent's code becomes during the census.
Each attempt must come back empty, or the census must mark the run `leak_suspected`. Every probe also has a positive
control, a value it must find, so an empty result means the probe worked and found nothing.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -q
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

import agent_env
import census
import layout
import secret
import vb
from common import canary, hmac_seed
from stub_provider import StubServer, bash, scripted

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
FAKE_KEY = "sk-test-not-a-real-key-5e1a"
MARKER = "vb-probe-marker-3c9d"  # in the driver's environment: what a working `ps -E` or /proc probe must show
CLAMP = "def clamp(value, low, high):\n    return max(low, min(value, high))\n"
CORRECT = f"cat > calc/ops.py <<'EOF'\n{CLAMP}EOF"
ENV_PROBE = ("printf 'VB_SECRET=%s\\n' \"${VB_SECRET-unset}\"; echo \"HOME=$HOME\"; "
             "cat ~/.config/viabilitybench/secret ~/.roko/.env 2>&1; true")
PS_PROBE = ("ps -E -ww -o command= -p $PPID 2>/dev/null; tr '\\0' '\\n' 2>/dev/null < /proc/$PPID/environ; "
            "tr '\\0' ' ' 2>/dev/null < /proc/$PPID/cmdline; true")
SEARCH = ("grep -rIl -e vb-canary- -e VB_SECRET . .. ../.. 2>/dev/null; "
          "find . .. ../.. -iname '*secret*' 2>/dev/null; true")
# calc/ops.py that, whenever it is imported, records its own environment and its parent's argv and environment. In
# the census its parent is hidden.py, which holds the secret file's path in its argv.
CENSUS_PROBE = r"""cat > calc/ops.py <<'EOF'
import os
import subprocess


def _probe():
    parent = os.getppid()
    seen = [repr(sorted(os.environ.items()))]
    try:
        seen.append(subprocess.run(["ps", "-E", "-ww", "-o", "command=", "-p", str(parent)], capture_output=True,
                                   text=True, timeout=10).stdout)
    except (OSError, subprocess.SubprocessError):
        pass
    for name in ("environ", "cmdline"):
        try:
            with open(f"/proc/{parent}/{name}", "rb") as handle:
                seen.append(handle.read().decode("utf-8", "replace").replace("\0", " "))
        except OSError:
            pass
    with open("probe.txt", "a", encoding="utf-8") as handle:
        handle.write("\n".join(seen) + "\n")


_probe()
""" + CLAMP + "EOF"
# calc/ops.py that, when its parent's argv names a secret file (hidden.py's, in the census), copies that file here.
CENSUS_STASH = r"""cat > calc/ops.py <<'EOF'
import os
import subprocess


def _stash():
    parent = os.getppid()
    try:
        with open(f"/proc/{parent}/cmdline", "rb") as handle:
            argv = handle.read().decode("utf-8", "replace").split("\0")
    except OSError:
        argv = subprocess.run(["ps", "-ww", "-o", "command=", "-p", str(parent)], capture_output=True,
                              text=True).stdout.split()
    if "--secret-file" in argv:
        with open(argv[argv.index("--secret-file") + 1], encoding="utf-8") as source:
            with open("stash.txt", "w", encoding="utf-8") as stash:
                stash.write(source.read())


_stash()
""" + CLAMP + "EOF"


@pytest.fixture(autouse=True)
def fresh_registry(monkeypatch):
    """Each test starts with no secret registered in agent_env; `vb run` registers its own."""
    monkeypatch.setattr(agent_env, "_forbidden", ())


@pytest.fixture
def operator(tmp_path: Path, monkeypatch) -> dict[str, Path]:
    """An operator's home with the secret at its default path and a roko dotenv holding only a provider key."""
    home = tmp_path / "home"
    monkeypatch.setenv("HOME", str(home))
    monkeypatch.delenv("VB_SECRET_FILE", raising=False)
    monkeypatch.delenv("VB_SECRET", raising=False)
    path = secret.create(secret.resolve())
    (home / ".roko").mkdir(mode=0o700)
    (home / ".roko" / ".env").write_text(f"CEREBRAS_API_KEY={FAKE_KEY}\n", encoding="utf-8")
    workspace = tmp_path / "ws"
    workspace.mkdir()
    monkeypatch.chdir(workspace)
    return {"home": home, "secret": path, "workspace": workspace, "results": tmp_path / "results",
            "work": tmp_path / "work"}


def run_args(places: dict[str, Path], url: str, *extra: str) -> list[str]:
    return ["run", "--experiment", "TEST-SECRET", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
            "cheap_direct", "--model", "gpt-oss-120b", "--seeds", "1", "--provider-url", url, "--results",
            str(places["results"]), "--work", str(places["work"]), *extra]


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def needle_files(root: Path, loaded: secret.DriverSecret) -> list[str]:
    """Every regular file under `root` that holds the secret or its canary, links never followed."""
    hits = []
    for directory, _, filenames in os.walk(root):
        for name in filenames:
            path = Path(directory) / name
            if path.is_file() and not path.is_symlink() and (
                    loaded.find_in_file(path) or loaded.canary.encode() in path.read_bytes()):
                hits.append(str(path.relative_to(root)))
    return hits


def observations(transcript: list[dict]) -> dict[str, str]:
    """command -> the output the agent saw for it."""
    return {entry["command"]: entry["content"] for entry in transcript if entry.get("role") == "user"
            and "command" in entry}


def test_secret_never_reaches_an_agent_env(operator, tmp_path):
    loaded = secret.load(operator["secret"])
    value = next(line for line in operator["secret"].read_text().splitlines() if not line.startswith("#"))
    assert loaded.canary and canary.CANARY_RE.fullmatch(loaded.canary) and loaded.needles == (value, loaded.canary)

    # The one environment builder refuses the secret under any name, once `vb run` has registered it.
    secret.preflight(operator["secret"], work_root=operator["work"], results_root=operator["results"])
    home = operator["work"] / "_home" / "probe"
    for extra in ({"NOTES": f"x{value}y"}, {"LOG_TAG": loaded.canary}, {"VB_SECRET_FILE": str(loaded.path)}):
        with pytest.raises(agent_env.AgentEnvError):
            agent_env.build(home=home, extra=extra)
    with pytest.raises(agent_env.AgentEnvError):  # the operator's HOME holds ~/.roko/.env and the secret's directory
        agent_env.build(home=home, extra={"HOME": str(operator["home"])})

    # End to end: the driver runs as its own process, so `ps -E` and /proc show the agent its real environment.
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash("env | sort", "What is in my environment?"), bash(ENV_PROBE, "Try the obvious names."),
                       bash(PS_PROBE, "Read my parent's argv and environment."),
                       bash(CENSUS_PROBE, "Implement it, and watch whoever imports it."), bash("echo VB_SUBMIT")],
               second: [bash(SEARCH, "Search every file around my workdir."), bash(CORRECT, "Implement it."),
                        bash("echo VB_SUBMIT")]}
    driver_env = {"PATH": os.environ["PATH"], "HOME": str(operator["home"]), "VB_SECRET_FILE": str(loaded.path),
                  "VB_PROBE": MARKER, "CEREBRAS_API_KEY": FAKE_KEY}
    with StubServer(scripted(scripts)) as stub:
        done = subprocess.run([sys.executable, str(layout.DRIVER_DIR / "vb.py"),
                               *run_args(operator, stub.url, "--transcripts", "--keep-workdirs")],
                              cwd=operator["workspace"], env=driver_env, capture_output=True, text=True, timeout=300)
    assert done.returncode == 0, done.stderr
    out = operator["results"] / "TEST-SECRET" / "run-1"
    rows = {record["task"]["instance_id"]: record for record in read_jsonl(out / "records.jsonl")}
    for record in rows.values():
        assert record["execution"]["status"] == "completed" and record["vs"]["label"] == 1
        assert record["provenance"]["canary_hits"] == 0 and record["provenance"]["canary_places"] == []

    seen = observations(json.loads((out / "transcripts" / f"{first}.s1.json").read_text()))
    names = {line.split("=", 1)[0] for line in seen["env | sort"].splitlines() if "=" in line}
    assert "PATH" in names and not [name for name in names if agent_env.FORBIDDEN_NAME.search(name)]
    assert f"HOME={operator['work'] / 'run-1' / '_home' / f'{first}.s1'}" in seen["env | sort"]
    assert "VB_SECRET=unset" in seen[ENV_PROBE] and "No such file" in seen[ENV_PROBE]
    # Control: the probe did read the driver's environment and argv, which name the secret file but never hold it.
    assert MARKER in seen[PS_PROBE] and f"VB_SECRET_FILE={loaded.path}" in seen[PS_PROBE]
    # Control: in the census, the agent's code saw hidden.py's argv with the path, and an agent environment.
    probe = (out / "private" / f"{first}.s1" / "census" / "hidden" / "probe.txt").read_text()
    assert f"--secret-file {loaded.path}" in probe and f"{first}.s1.census" in probe and MARKER not in probe
    listing = observations(json.loads((out / "transcripts" / f"{second}.s1.json").read_text()))[SEARCH]
    assert str(loaded.path) not in listing

    for root in (out, operator["work"]):  # records, ledger, manifest, transcripts, archives, exports, workdirs, homes
        assert needle_files(root, loaded) == [], root


def test_run_refused_when_secret_is_in_roko_dotenv(operator, capsys, monkeypatch):
    loaded = secret.load(operator["secret"])
    value = next(line for line in operator["secret"].read_text().splitlines() if not line.startswith("#"))
    user_dotenv = operator["home"] / ".roko" / ".env"
    workspace_dotenv = operator["workspace"] / ".roko" / ".env"
    clean = user_dotenv.read_text()
    cases = [
        (user_dotenv, clean + f"VB_SECRET={value}\n", "defines VB_SECRET"),  # S08 decision 8's old default
        (user_dotenv, clean + 'export VB_SECRET="not-even-the-real-one"\n', "defines VB_SECRET"),
        (user_dotenv, clean + f"BENCH_KEY={value}\n", "holds the secret"),
        (user_dotenv, clean + f"# {loaded.canary}\n", "holds the secret"),
        (workspace_dotenv, f"VB_SECRET={value}\n", "defines VB_SECRET"),
    ]
    with StubServer(lambda body: bash("echo VB_SUBMIT")) as stub:
        for dotenv, text, reason in cases:
            dotenv.parent.mkdir(exist_ok=True)
            dotenv.write_text(text, encoding="utf-8")
            assert vb.main(run_args(operator, stub.url, "--limit", "1")) == 2, dotenv
            err = capsys.readouterr().err
            assert str(dotenv) in err and reason in err and value not in err and loaded.canary not in err
            dotenv.write_text(clean if dotenv == user_dotenv else "", encoding="utf-8")
        monkeypatch.setenv("VB_SECRET", value)
        assert vb.main(run_args(operator, stub.url, "--limit", "1")) == 2
        assert "VB_SECRET is set in the driver's environment" in capsys.readouterr().err
        monkeypatch.setenv("VB_SECRET", "anything")
        assert vb.main(run_args(operator, stub.url, "--limit", "1")) == 2
        monkeypatch.delenv("VB_SECRET")
        monkeypatch.setenv("VB_NOTES", f"keep {value} handy")
        assert vb.main(run_args(operator, stub.url, "--limit", "1")) == 2
        assert "VB_NOTES holds the secret" in capsys.readouterr().err
        monkeypatch.delenv("VB_NOTES")
        assert stub.requests == []
        assert not operator["results"].exists() and not operator["work"].exists()

        # Control: the same run starts once no dotenv or environment variable exposes the secret.
        assert vb.main(run_args(operator, stub.url, "--limit", "1")) == 0
    [record] = read_jsonl(operator["results"] / "TEST-SECRET" / "run-1" / "records.jsonl")
    assert record["execution"]["status"] == "completed" and record["provenance"]["canary_hits"] == 0


def test_deliberate_reads_are_flagged_and_kept_out_of_records(operator):
    """Same-uid agents can read the file by path (0600 stops other users only); the census must catch it."""
    loaded = secret.load(operator["secret"])
    path = loaded.path
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash(f"cat '{path}'", "Print it."), bash(f"touch \"$(grep -v '^#' '{path}')\" && {CORRECT}",
                                                                  "Keep a copy as a file name, silently."),
                       bash("echo VB_SUBMIT")],
               second: [bash(CENSUS_STASH, "Copy the secret when the truth suite runs me."), bash("echo VB_SUBMIT")]}
    with StubServer(scripted(scripts)) as stub:
        assert vb.main(run_args(operator, stub.url, "--transcripts")) == 0
    out = operator["results"] / "TEST-SECRET" / "run-1"
    rows = {record["task"]["instance_id"]: record for record in read_jsonl(out / "records.jsonl")}
    reader, stasher = rows[first], rows[second]
    assert reader["execution"]["status"] == "leak_suspected"
    assert {"transcript", "argv", "tree:<vb-secret>"} <= set(reader["provenance"]["canary_places"])
    # The stasher never names the file and never prints it: only the census's own sweep sees the copy.
    assert stasher["execution"]["status"] == "leak_suspected"
    assert stasher["provenance"]["canary_places"] == ["census:hidden/stash.txt"]
    assert "transcripts/" + f"{first}.s1.json" in needle_files(out, loaded)  # the evidence stays in the transcript
    for name in ("records.jsonl", "ledger.jsonl", "manifest.json"):  # but the census redacts what records carry
        assert not loaded.find_in_file(out / name) and loaded.canary not in (out / name).read_text()


def test_census_refuses_to_run_agent_code_with_the_secret_in_reach(operator, tmp_path):
    loaded = secret.load(operator["secret"])
    value = next(line for line in operator["secret"].read_text().splitlines() if not line.startswith("#"))
    env = agent_env.build(home=tmp_path / "census-home")
    for leaky in ({**env, "NOTES": value}, {**env, "HOME": str(operator["home"])}, {k: v for k, v in env.items()
                                                                                       if k != "HOME"}):
        result = census.run_census(final=None, manifest={}, manifest_path=tmp_path / "task.json", pristine=None,
                                   family_dir=tmp_path, secret_file=loaded.path, completed=True, transcript_text="[]",
                                   diff_text="", scratch=tmp_path / "scratch", env=leaky)
        assert result.infra_error.startswith("refusing to run the census") and value not in result.infra_error
        assert result.unknown and not (tmp_path / "scratch").exists()


def test_the_secret_file_is_private_and_kept_apart(operator, tmp_path, capsys, monkeypatch):
    path = operator["secret"]
    assert path == operator["home"] / ".config" / "viabilitybench" / "secret"
    assert path.stat().st_mode & 0o777 == 0o600 and path.parent.stat().st_mode & 0o777 == 0o700
    loaded = secret.load(path)
    assert hmac_seed.read_secret_file(path).fingerprint == loaded.fingerprint and "<redacted>" in repr(loaded)
    with pytest.raises(secret.SecretError, match="already exists"):
        secret.create(path)
    assert secret.main(["check"]) == 0 and "canary=yes" in capsys.readouterr().out
    assert secret.main(["init", "--secret-file", str(tmp_path / "second" / "secret")]) == 0
    assert secret.load(tmp_path / "second" / "secret").canary != loaded.canary

    os.chmod(path.parent, 0o750)
    with pytest.raises(secret.SecretError, match="directory"):
        secret.load(path)
    os.chmod(path.parent, 0o700)
    os.chmod(path, 0o640)
    with pytest.raises(secret.SecretError, match="group or others"):
        secret.load(path)
    os.chmod(path, 0o600)
    link = tmp_path / "private-link" / "secret"
    link.parent.mkdir(mode=0o700)
    link.symlink_to(path)
    with pytest.raises(secret.SecretError, match="regular file"):
        secret.load(link)
    with pytest.raises(secret.SecretError, match="repository"):  # refused before anything is written
        secret.create(layout.REPO_ROOT / "tmp" / "vb-secret-test" / "secret")
    assert not (layout.REPO_ROOT / "tmp" / "vb-secret-test").exists()
    with pytest.raises(secret.SecretError, match=r"~/\.roko"):
        secret.create(operator["home"] / ".roko" / "secret")
    with monkeypatch.context() as scoped:  # an existing file that lies inside the repository is refused too
        scoped.setattr(layout, "REPO_ROOT", path.parent.parent)
        with pytest.raises(secret.SecretError, match="repository"):
            secret.load(path)

    work, results = tmp_path / "work", tmp_path / "results"
    assert secret.preflight(path, work_root=work, results_root=results).fingerprint == loaded.fingerprint
    for kwargs in ({"work_root": path.parent / "work", "results_root": results},
                   {"work_root": work, "results_root": path.parent.parent}):
        with pytest.raises(secret.SecretError, match="must"):
            secret.preflight(path, **kwargs)
    inside_work = secret.create(work / "keys" / "secret")
    with pytest.raises(secret.SecretError, match="--work"):
        secret.preflight(inside_work, work_root=work, results_root=results)
