"""Offline tests that the benchmark secret and the provider keys stay driver-only (gap-a8a160, gap-308373, bug-979a06,
S08 SC4): adversarial fake agents on a local stub server, the toy family, no model calls.

The fake agents try to read the secret through their environment, by searching the files around their workdir, and
through `ps -E` or `/proc` on the driver and on `hidden.py`, whose child the agent's code becomes during the census.
They also open the secret file and the key file by path, silently (the tripwire), and read the driver's environment for
a provider key. Each attempt must come back empty, or the census must mark the run `leak_suspected`. Every probe also
has a positive control, a value it must find, so an empty result means the probe worked and found nothing.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -q
"""

from __future__ import annotations

import json
import os
import re
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace

import pytest

import agent_env
import census
import layout
import provider
import run_roko
import secret
import vb
from common import canary, hmac_seed, sandbox
from stub_provider import StubServer, bash, scripted

TOY_STREAM = str(layout.DRIVER_DIR / "testdata" / "toy_stream.toml")
FAKE_KEY = "sk-test-not-a-real-key-5e1a"
MARKER = "vb-probe-marker-3c9d"  # in the operator's environment, which `vb run` sheds before any agent runs
HOLD_LOCK = ("import fcntl, os, sys, time; fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT, 0o600); "
             "fcntl.flock(fd, fcntl.LOCK_EX); print('locked', flush=True); time.sleep(60)")  # another run's lock
CLAMP = "def clamp(value, low, high):\n    return max(low, min(value, high))\n"
CORRECT = f"cat > calc/ops.py <<'EOF'\n{CLAMP}EOF"
ENV_PROBE = ("printf 'VB_SECRET=%s\\n' \"${VB_SECRET-unset}\"; echo \"HOME=$HOME\"; "
             "cat ~/.config/viabilitybench/secret ~/.roko/.env 2>&1; true")
# What `ps -E` shows of a process, read with sysctl(KERN_PROCARGS2) instead: inside the agents' macOS sandbox
# (gap-0bd49a) the setuid `ps` cannot start, and a parent's argv and environment stay readable this way.
PROCARGS = r"""import ctypes, ctypes.util, sys
libc = ctypes.CDLL(ctypes.util.find_library("c"))
size = ctypes.c_size_t(1 << 20)
buf = ctypes.create_string_buffer(size.value)
if libc.sysctl((ctypes.c_int * 3)(1, 49, int(sys.argv[1])), 3, buf, ctypes.byref(size), None, 0) == 0:
    print(" ".join(part.decode("utf-8", "replace") for part in buf.raw[4:size.value].split(b"\0") if part))"""
PS_PROBE = ("ps -E -ww -o command= -p $PPID 2>/dev/null || python3 -c '" + PROCARGS + "' $PPID 2>/dev/null; "
            "tr '\\0' '\\n' 2>/dev/null < /proc/$PPID/environ; tr '\\0' ' ' 2>/dev/null < /proc/$PPID/cmdline; true")
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
# calc/ops.py that copies the secret file here, or notes why it could not (the census's sandbox, gap-8c3752). The test
# fills in the file's directory, and the name is split, so no command names the file (census place `argv`).
CENSUS_STASH = r"""cat > calc/ops.py <<'EOF'
import os


def _stash():
    try:
        with open(os.path.join(__SECRET_DIR__, "sec" + "ret"), encoding="utf-8") as source:
            text = source.read()
    except OSError as err:
        text = f"denied: {err.strerror}"
    with open("stash.txt", "w", encoding="utf-8") as stash:
        stash.write(text)


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
                  "VB_PROBE": MARKER}  # no provider key: `vb run` refuses one in its environment (bug-979a06)
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
    # The driver runs scrubbed (bug-32eb77), so the operator's other variables, such as the marker, are gone.
    assert f"VB_SECRET_FILE={loaded.path}" in seen[PS_PROBE] and MARKER not in seen[PS_PROBE]
    # Control: in the census, the agent's code ran in an agent environment. Without a sandbox it also saw hidden.py's
    # argv, which names the secret file; the sandbox cannot run the setuid `ps` at all (gap-8c3752).
    probe = (out / "private" / f"{first}.s1" / "census" / "hidden" / "probe.txt").read_text()
    assert f"{first}.s1.census" in probe and MARKER not in probe
    assert (f"--secret-file {loaded.path}" in probe) is (sandbox.KIND == "none")
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


def readable_secret(monkeypatch) -> None:
    """Leave the files the tripwire holds out of the agent's sandbox, as on a host without one: on macOS the direct
    loop denies its agent those files (gap-0bd49a; test_sandbox_net.py), and the detection below needs a read to
    detect."""
    monkeypatch.setattr(vb, "_agent_deny", lambda run: (run.run_dir / "private",))


def test_deliberate_reads_are_flagged_and_kept_out_of_records(operator, monkeypatch):
    """Same-uid agents can read the file by path (0600 stops other users only, and the tripwire's mode 000 only until a
    chmod); the census must catch it."""
    readable_secret(monkeypatch)
    loaded = secret.load(operator["secret"])
    path = loaded.path
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash(f"chmod 600 '{path}' && cat '{path}'", "Print it."),
                       bash(f"touch \"$(grep -v '^#' '{path}')\" && {CORRECT}",
                            "Keep a copy as a file name, silently."),
                       bash("echo VB_SUBMIT")],
               second: [bash(CENSUS_STASH.replace("__SECRET_DIR__", repr(str(path.parent))),
                             "Copy the secret when the truth suite runs me."), bash("echo VB_SUBMIT")]}
    with StubServer(scripted(scripts)) as stub:
        assert vb.main(run_args(operator, stub.url, "--transcripts", "--keep-workdirs")) == 0
    out = operator["results"] / "TEST-SECRET" / "run-1"
    rows = {record["task"]["instance_id"]: record for record in read_jsonl(out / "records.jsonl")}
    reader, stasher = rows[first], rows[second]
    assert reader["execution"]["status"] == "leak_suspected"
    assert {"transcript", "argv", "tree:<vb-secret>", "secret-file:mode", "secret-file:ctime"} <= set(
        reader["provenance"]["canary_places"])
    # The stasher never names the file and never prints it. The truth suite's sandbox denies it the file
    # (gap-8c3752); on a host without one, only the census's own sweep sees the copy.
    stash = (out / "private" / f"{second}.s1" / "census" / "hidden" / "stash.txt").read_text()
    assert stasher["vs"]["sandbox"] == sandbox.KIND
    if sandbox.KIND == "none":
        assert stasher["execution"]["status"] == "leak_suspected"
        assert stasher["provenance"]["canary_places"] == ["census:hidden/stash.txt"]
    else:
        assert stash == "denied: Operation not permitted"
        assert stasher["execution"]["status"] == "completed" and stasher["provenance"]["canary_places"] == []
    assert "transcripts/" + f"{first}.s1.json" in needle_files(out, loaded)  # the evidence stays in the transcript
    for name in ("records.jsonl", "ledger.jsonl", "manifest.json"):  # but the census redacts what records carry
        assert not loaded.find_in_file(out / name) and loaded.canary not in (out / name).read_text()


def test_a_silent_read_of_the_secret_file_is_detected(operator, tmp_path, capsys, monkeypatch):
    """gap-308373: an agent under the driver's user can open the secret file by path, and a read that never shows the
    secret leaves the census nothing to find. While agents run, the tripwire keeps the file at mode 000, so a read
    needs a chmod, and the chmod changes the file's ctime, which the agent cannot set back."""
    readable_secret(monkeypatch)
    loaded = secret.load(operator["secret"])
    split = f"f='{loaded.path.parent}/sec''ret'"  # no command in the transcript names the file (census place `argv`)
    silent = f"{split}; chmod 600 \"$f\" && s=$(grep -v '^#' \"$f\") && chmod 000 \"$f\" && {CORRECT}"
    plain = f"{split}; cat \"$f\"; true"
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash(silent, "Read the secret quietly, put the mode back, and solve."), bash("echo VB_SUBMIT")],
               second: [bash(plain, "Just read it."), bash(CORRECT, "Solve."), bash("echo VB_SUBMIT")]}
    with StubServer(scripted(scripts)) as stub:
        assert vb.main(run_args(operator, stub.url, "--transcripts")) == 0
    out = operator["results"] / "TEST-SECRET" / "run-1"
    rows = {record["task"]["instance_id"]: record for record in read_jsonl(out / "records.jsonl")}
    reader, tried = rows[first], rows[second]

    # The silent reader left no secret, canary or path in its transcript, diff or tree: only the tripwire saw it.
    transcript = (out / "transcripts" / f"{first}.s1.json").read_text()
    assert not loaded.find(transcript) and loaded.canary not in transcript and str(loaded.path) not in transcript
    assert reader["execution"]["status"] == "leak_suspected"
    assert reader["provenance"]["canary_places"] == ["secret-file:ctime"]  # the mode was put back; the ctime was not
    assert reader["vs"]["checks"]["hidden"] is None and reader["vs"]["label"] == 0  # hidden.py never got the file
    # Without a chmod the read fails, and the run stands.
    tried_transcript = json.loads((out / "transcripts" / f"{second}.s1.json").read_text())
    assert "Permission denied" in observations(tried_transcript)[plain]
    assert tried["execution"]["status"] == "completed" and tried["vs"]["label"] == 1
    assert tried["provenance"]["canary_places"] == []
    # After the run the file is at rest again, and every record says nothing of the secret.
    assert stat.S_IMODE(loaded.path.stat().st_mode) == secret.REST_MODE and secret.tripwires() == ()
    assert not loaded.find_in_file(out / "records.jsonl")

    # A file left armed (a killed run) or open (a run using it) is refused, and says why.
    for mode, reason in ((secret.ARMED_MODE, "the tripwire of a vb run"), (secret.OPEN_MODE, "a vb run is using it")):
        os.chmod(loaded.path, mode)
        assert vb.main(run_args(operator, "http://127.0.0.1:9/v1", "--run-id", "run-2")) == 2
        assert reason in capsys.readouterr().err
    os.chmod(loaded.path, secret.REST_MODE)
    # One run per file, across processes: while another process holds the lock, the tripwire arms nothing.
    holder = subprocess.Popen([sys.executable, "-c", HOLD_LOCK, f"{loaded.path}.lock"], stdout=subprocess.PIPE,
                              text=True)
    try:
        assert holder.stdout.readline().strip() == "locked"
        with pytest.raises(secret.SecretError, match="another vb run holds"), secret.tripwire(loaded):
            pass
        assert stat.S_IMODE(loaded.path.stat().st_mode) == secret.REST_MODE and secret.tripwires() == ()
    finally:
        holder.kill()
        holder.wait()
    # A SIGTERM ends the run through its `finally` blocks, which put the file back at rest.
    before = signal.getsignal(signal.SIGTERM)
    with pytest.raises(SystemExit) as stopped, secret.tripwire(loaded):
        assert stat.S_IMODE(loaded.path.stat().st_mode) == secret.ARMED_MODE
        os.kill(os.getpid(), signal.SIGTERM)
        time.sleep(5)  # the handler raises before this ends
    assert stopped.value.code == 128 + signal.SIGTERM and signal.getsignal(signal.SIGTERM) is before
    assert stat.S_IMODE(loaded.path.stat().st_mode) == secret.REST_MODE and secret.tripwires() == ()
    # The stamp sees a replaced file, and the tripwire never chmods a file it did not stamp.
    wire = secret.Tripwire("secret-file", loaded.path, loaded)
    wire.arm()
    impostor = loaded.path.with_name("impostor")  # a secret the agent chose, which would predict the hidden cases
    impostor.write_text("# canary vb-canary-00000000-0000-4000-8000-000000000000\n" + "0" * 64 + "\n")
    os.chmod(impostor, 0o644)
    os.replace(impostor, loaded.path)
    assert wire.check() == ["replaced"] and not wire.set(secret.ARMED_MODE)
    assert stat.S_IMODE(loaded.path.stat().st_mode) == 0o644


def test_no_provider_key_in_the_driver_environment(operator, tmp_path, capsys, monkeypatch):
    """bug-979a06: every agent under the driver's user can read the environment the driver started with (`ps -E`,
    /proc/<ppid>/environ). So the driver reads its provider key from a driver-only key file into memory, never from its
    environment, and refuses to start with a key there; the tripwire holds the key file at mode 000 while agents run."""
    key = "csk-test-driver-only-8f3b2a71c9"
    key_file = secret.create_keys(operator["home"] / ".config" / "viabilitybench" / "keys", {"CEREBRAS_API_KEY": key})
    key_probe = f"cat '{key_file}'; true"
    silent = (f"{{ chmod 600 '{key_file}' && k=$(cat '{key_file}') && chmod 000 '{key_file}'; }} 2>/dev/null; "
              "echo VB_SUBMIT")
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash(PS_PROBE, "Read my parent's environment."), bash(key_probe, "Read the key file."),
                       bash(CORRECT, "Solve."), bash("echo VB_SUBMIT")],
               second: [bash(silent, "Take the key quietly, and stop.")]}
    with StubServer(scripted(scripts)) as stub:
        # cheap_direct with its provider on the stub: an endpoint that names its key, with no network. The proxy, the
        # only sender of a key, fronts it.
        arm_text, count = re.subn(r"(?m)^base_url = .*$", f'base_url = "{stub.url}"',
                                  (layout.ARMS_DIR / "cheap_direct.toml").read_text())
        assert count == 1
        (tmp_path / "keyed.toml").write_text(arm_text)
        args = ["run", "--experiment", "TEST-KEYS", "--run-id", "run-1", "--stream", TOY_STREAM, "--arm",
                str(tmp_path / "keyed.toml"), "--model", "gpt-oss-120b", "--seeds", "1", "--results",
                str(operator["results"]), "--work", str(operator["work"]), "--key-file", str(key_file), "--proxy"]

        # Refused before any request or directory: the arm's key variable, another arm's, or the key under any name.
        for name, value in (("CEREBRAS_API_KEY", key), ("OPENAI_API_KEY", "sk-another-arms-key-5d1c9e"),
                            ("NOTES", f"keep {key} handy")):
            monkeypatch.setenv(name, value)
            assert vb.main(args) == 2
            err = capsys.readouterr().err
            assert name in err and key not in err
            monkeypatch.delenv(name)
        assert stub.requests == [] and not operator["results"].exists() and not operator["work"].exists()

        # The driver as its own process, so the probe reads the environment it really started with.
        driver_env = {"PATH": os.environ["PATH"], "HOME": str(operator["home"]), "VB_PROBE": MARKER}
        done = subprocess.run([sys.executable, str(layout.DRIVER_DIR / "vb.py"), *args, "--transcripts"],
                              cwd=operator["workspace"], env=driver_env, capture_output=True, text=True, timeout=300)
        headers = list(stub.headers)
    assert done.returncode == 0, done.stderr
    assert headers and all(item.get("authorization") == f"Bearer {key}" for item in headers)  # the proxy sent it
    out = operator["results"] / "TEST-KEYS" / "run-1"
    rows = {record["task"]["instance_id"]: record for record in read_jsonl(out / "records.jsonl")}
    seen = observations(json.loads((out / "transcripts" / f"{first}.s1.json").read_text()))
    assert f"HOME={operator['home']}" in seen[PS_PROBE]  # control: the probe read the driver's environment
    assert key not in seen[PS_PROBE] and "CEREBRAS_API_KEY" not in seen[PS_PROBE]
    assert key not in seen[key_probe]
    assert rows[first]["execution"]["status"] == "completed" and rows[first]["vs"]["label"] == 1
    if sandbox.KIND == "none":  # the key file sat at mode 000, and a silent read trips the tripwire
        assert "Permission denied" in seen[key_probe]
        assert rows[second]["execution"]["status"] == "leak_suspected"
        assert rows[second]["provenance"]["canary_places"] == ["key-file:ctime"]
    else:  # the agent's sandbox denies it the key file (gap-0bd49a): even the chmod fails, so nothing changed
        assert "Operation not permitted" in seen[key_probe]
        assert rows[second]["execution"]["status"] == "completed" and rows[second]["provenance"]["canary_places"] == []
    assert stat.S_IMODE(key_file.stat().st_mode) == secret.REST_MODE
    for root in (out, operator["work"]):  # records, ledger, manifest, transcripts, archives, workdirs, homes
        assert [path for path in root.rglob("*") if path.is_file() and key.encode() in path.read_bytes()] == []
    # Roko, whose tools run the agent's commands, gets a placeholder on a loopback URL, and a network endpoint (one
    # that skipped the proxy) is refused before Roko starts.
    loopback = SimpleNamespace(endpoint=provider.Endpoint("cerebras", "http://127.0.0.1:9/cerebras"), agent_env={})
    assert run_roko._roko_env(loopback, "CEREBRAS_API_KEY", tmp_path / "roko.toml")["CEREBRAS_API_KEY"] == \
        run_roko.OFFLINE_KEY
    network = SimpleNamespace(endpoint=provider.Endpoint("cerebras", "https://api.cerebras.ai/v1", "CEREBRAS_API_KEY"),
                              agent_env={})
    with pytest.raises(run_roko.RunnerError, match="metering proxy"):
        run_roko._roko_env(network, "CEREBRAS_API_KEY", tmp_path / "roko.toml")


def test_the_driver_runs_with_a_scrubbed_environment(operator):
    """bug-32eb77: a credential the operator's shell exports would sit in the driver's start-up environment, which
    every agent can read (`ps -E`, /proc). Once its checks pass, `vb run` starts itself again with an allowlisted
    environment. The checks see the original, so a benchmark key in it is still refused rather than hidden."""
    credentials = {"ANTHROPIC_API_KEY": "sk-ant-operator-only-4f1e9c", "GITHUB_TOKEN": "ghp_operatorOnly7c2d18",
                   "AWS_SECRET_ACCESS_KEY": "aws-operator-only-93b0e2", "VB_PROBE": MARKER,
                   "DATABASE_URL": "postgres://bench:hunter2-operator-only@db.invalid/runs"}
    first, second = vb.load_stream(TOY_STREAM).order(1)
    scripts = {first: [bash(PS_PROBE, "Read my parent's environment."), bash("env | sort", "And my own."),
                       bash(CORRECT, "Solve."), bash("echo VB_SUBMIT")],
               second: [bash(CORRECT, "Solve."), bash("echo VB_SUBMIT")]}
    driver_env = {"PATH": os.environ["PATH"], "HOME": str(operator["home"]), "LANG": "en_US.UTF-8", **credentials}
    command = [sys.executable, str(layout.DRIVER_DIR / "vb.py")]
    with StubServer(scripted(scripts)) as stub:
        refused = subprocess.run([*command, *run_args(operator, stub.url)], cwd=operator["workspace"], text=True,
                                 env={**driver_env, "CEREBRAS_API_KEY": "csk-operator-only-0d8e2b17"},
                                 capture_output=True, timeout=120)
        assert refused.returncode == 2 and "CEREBRAS_API_KEY is set in the driver's environment" in refused.stderr
        assert stub.requests == [] and not operator["results"].exists()
        done = subprocess.run([*command, *run_args(operator, stub.url, "--transcripts")], cwd=operator["workspace"],
                              env=driver_env, capture_output=True, text=True, timeout=300)
    assert done.returncode == 0, done.stderr
    out = operator["results"] / "TEST-SECRET" / "run-1"
    for record in read_jsonl(out / "records.jsonl"):
        assert record["execution"]["status"] == "completed" and record["vs"]["label"] == 1
    seen = observations(json.loads((out / "transcripts" / f"{first}.s1.json").read_text()))
    parent = seen[PS_PROBE]
    # Control: the probe read the driver's start-up environment: the allowlisted names, and the restart's mark.
    assert f"HOME={operator['home']}" in parent and "LANG=en_US.UTF-8" in parent
    assert f"{agent_env.DRIVER_SCRUBBED}=scrubbed" in parent
    assert [name for name, value in credentials.items() if f"{name}=" in parent or value in parent] == []
    assert [value for value in credentials.values() if value in seen["env | sort"]] == []
    for root in (out, operator["work"]):  # records, ledger, manifest, transcripts, archives, workdirs, homes
        assert [path for path in root.rglob("*") if path.is_file()
                and any(value.encode() in path.read_bytes() for value in credentials.values())] == []
    assert agent_env.driver_env({"PATH": "/bin", "LC_ALL": "C", "VB_WORK": "/w", "VB_SECRET": "x" * 40,
                                 "GITHUB_TOKEN": "t", "SHLVL": "2"}) == {
        "PATH": "/bin", "LC_ALL": "C", "VB_WORK": "/w", agent_env.DRIVER_SCRUBBED: "scrubbed"}


def test_the_key_file_is_private_and_well_formed(operator, tmp_path, capsys, monkeypatch):
    key = "csk-test-driver-only-8f3b2a71c9"
    path = secret.create_keys(operator["home"] / ".config" / "viabilitybench" / "keys", {"CEREBRAS_API_KEY": key})
    assert path.stat().st_mode & 0o777 == 0o600
    keys = secret.load_keys(path, need=["CEREBRAS_API_KEY"])
    assert dict(keys) == {"CEREBRAS_API_KEY": key} and key not in repr(keys) and "CEREBRAS_API_KEY" in repr(keys)
    assert {"CEREBRAS_API_KEY", "OPENAI_API_KEY"} <= secret.arm_key_names()
    assert secret.main(["keys", "--key-file", str(path)]) == 0
    printed = capsys.readouterr()
    assert "keys=CEREBRAS_API_KEY mode=0600" in printed.out and key not in printed.out + printed.err
    with pytest.raises(secret.SecretError, match="has no OPENAI_API_KEY"):
        secret.load_keys(path, need=["OPENAI_API_KEY"])
    with pytest.raises(secret.SecretError, match="already exists"):
        secret.create_keys(path, {"CEREBRAS_API_KEY": key})

    os.chmod(path, 0o640)
    with pytest.raises(secret.SecretError, match="group or others"):
        secret.load_keys(path)
    os.chmod(path, 0o000)  # the tripwire's mode, which a killed run can leave behind
    with pytest.raises(secret.SecretError, match="tripwire"):
        secret.load_keys(path)
    os.chmod(path, 0o600)
    for text, reason in ((f"CEREBRAS_API_KEY={key}\nCEREBRAS_API_KEY={key}\n", "a second time"),
                         ("CEREBRAS_API_KEY=short\n", "line 1"), (f"not a key line {key}\n", "line 1"),
                         (f'# a comment\nexport CEREBRAS_API_KEY="{key}"\n', None)):
        path.write_text(text)
        if reason is None:  # comments, `export` and quotes are read as a shell would
            assert secret.load_keys(path)["CEREBRAS_API_KEY"] == key
            continue
        with pytest.raises(secret.SecretError, match=reason) as raised:
            secret.load_keys(path)
        assert key not in str(raised.value)
    link = tmp_path / "private-link" / "keys"
    link.parent.mkdir(mode=0o700)
    link.symlink_to(path)
    with pytest.raises(secret.SecretError, match="regular file"):
        secret.load_keys(link)
    with pytest.raises(secret.SecretError, match="repository"):
        secret.create_keys(layout.REPO_ROOT / "tmp" / "vb-keys-test" / "keys", {"CEREBRAS_API_KEY": key})
    assert not (layout.REPO_ROOT / "tmp" / "vb-keys-test").exists()
    with pytest.raises(secret.SecretError, match=r"~/\.roko"):
        secret.create_keys(operator["home"] / ".roko" / "keys", {"CEREBRAS_API_KEY": key})
    # The key file may not sit where agents work or records live, like the secret file.
    with pytest.raises(secret.SecretError, match="key file must live outside --work"):
        secret.preflight(operator["secret"], work_root=path.parent, results_root=tmp_path / "results",
                         keys=secret.load_keys(path))
    # `keys` reports a key the driver's environment exposes, and the tests' escape hatch silences that.
    monkeypatch.setenv("CEREBRAS_API_KEY", key)
    assert secret.main(["keys", "--key-file", str(path)]) == 1
    assert "CEREBRAS_API_KEY is set in the driver's environment" in capsys.readouterr().err
    monkeypatch.setattr(secret, "KEYS_IN_ENV_OK", True)
    assert secret.key_exposures(secret.load_keys(path)) == []


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
