#!/usr/bin/env python3
"""Run one command with a deadline and preserve a small, run-scoped evidence bundle.

This deliberately does not build anything, clean anything, or probe endpoints.  It is
the generic capture layer used by ``./dev.sh fast`` and may also be used directly via
``./dev.sh run-evidence``.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import platform
import re
import shlex
import shutil
import signal
import subprocess
import sys
import threading
import time
from typing import Any, BinaryIO, Iterable, NamedTuple, Sequence


SCHEMA_VERSION = 1
DEFAULT_DEADLINE_SECONDS = 300
MAX_DIFF_BYTES = 16 * 1024 * 1024
MAX_GIT_METADATA_BYTES = 4 * 1024 * 1024
MAX_OUTPUT_BYTES = 16 * 1024 * 1024
MAX_METADATA_ERROR_BYTES = 64 * 1024
OUTPUT_TRUNCATION_MARKER = b"\n[run-evidence output truncated at 16 MiB]\n"
SAFE_ENV_VALUE_NAMES = {
    "CARGO_INCREMENTAL",
    "CARGO_TARGET_DIR",
    "CI",
    "NO_COLOR",
    "ROKO_EVIDENCE_BUNDLE",
    "ROKO_EVIDENCE_RUN_ID",
    "ROKO_AGENT_SHARED_TARGET",
    "ROKO_FAST_MODE",
    "ROKO_FAST_PLAN_DEADLINE_SECS",
    "ROKO_SKIP_PREFLIGHT",
    "ROKO_TASK_VERIFY_ONLY",
    "RUSTC_WRAPPER",
    "RUST_LOG",
    "SKIP_FRONTEND_BUILD",
}
SECRET_NAME_RE = re.compile(
    r"(?:api[_-]?key|auth|bearer|credential|password|private[_-]?key|secret|token)",
    re.IGNORECASE,
)
SECRET_FLAG_RE = re.compile(
    r"^--?(?:api[_-]?key|auth|bearer|credential|password|private[_-]?key|secret|token)$",
    re.IGNORECASE,
)
SECRET_ASSIGNMENT_RE = re.compile(
    r"^(?P<name>[^=]*(?:api[_-]?key|auth|bearer|credential|password|private[_-]?key|secret|token)[^=]*)=(?P<value>.*)$",
    re.IGNORECASE,
)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace(
        "+00:00", "Z"
    )


def write_json(path: pathlib.Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
    os.replace(temporary, path)
    os.chmod(path, 0o600)


def append_jsonl(path: pathlib.Path, value: Any) -> None:
    with path.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(value, sort_keys=True, separators=(",", ":")))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.chmod(path, 0o600)


def write_text(path: pathlib.Path, value: str) -> None:
    path.write_text(value, encoding="utf-8")
    os.chmod(path, 0o600)


def sha256_file(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_metadata_command(
    argv: Sequence[str], cwd: pathlib.Path, timeout: float = 2.0
) -> tuple[int | None, bytes, bytes]:
    try:
        completed = subprocess.run(
            argv,
            cwd=str(cwd),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        return completed.returncode, completed.stdout, completed.stderr
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, b"", str(error).encode("utf-8", errors="replace")


class BoundedCommandResult(NamedTuple):
    returncode: int | None
    stdout: bytes
    stderr: bytes
    stdout_truncated: bool
    stderr_truncated: bool
    timed_out: bool
    spawn_error: str | None


def _read_bounded_pipe(
    source: BinaryIO,
    limit: int,
    destination: bytearray,
    state: dict[str, Any],
    overflow: threading.Event,
) -> None:
    observed = 0
    try:
        while True:
            chunk = source.read(64 * 1024)
            if not chunk:
                break
            observed += len(chunk)
            remaining = max(0, limit + 1 - len(destination))
            if remaining:
                destination.extend(chunk[:remaining])
            if observed > limit:
                overflow.set()
    except (OSError, ValueError) as error:
        state["read_error"] = f"{type(error).__name__}: {error}"
    finally:
        state["bytes_observed"] = observed
        try:
            source.close()
        except OSError:
            pass


def run_bounded_command(
    argv: Sequence[str],
    cwd: pathlib.Path,
    *,
    timeout: float,
    stdout_limit: int,
    stderr_limit: int = MAX_METADATA_ERROR_BYTES,
) -> BoundedCommandResult:
    """Capture a helper command without allowing output or descendants to escape bounds."""

    try:
        process = subprocess.Popen(
            argv,
            cwd=str(cwd),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as error:
        return BoundedCommandResult(
            None,
            b"",
            b"",
            False,
            False,
            False,
            f"{type(error).__name__}: {error}",
        )

    assert process.stdout is not None
    assert process.stderr is not None
    stdout = bytearray()
    stderr = bytearray()
    stdout_state: dict[str, Any] = {}
    stderr_state: dict[str, Any] = {}
    overflow = threading.Event()
    readers = [
        threading.Thread(
            target=_read_bounded_pipe,
            args=(process.stdout, stdout_limit, stdout, stdout_state, overflow),
            daemon=True,
            name="evidence-metadata-stdout",
        ),
        threading.Thread(
            target=_read_bounded_pipe,
            args=(process.stderr, stderr_limit, stderr, stderr_state, overflow),
            daemon=True,
            name="evidence-metadata-stderr",
        ),
    ]
    for reader in readers:
        reader.start()

    deadline = time.monotonic() + timeout
    timed_out = False
    while process.poll() is None:
        if overflow.is_set():
            stop_process_group(process, 0.1)
            break
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            timed_out = True
            stop_process_group(process, 0.1)
            break
        try:
            process.wait(timeout=min(0.05, remaining))
        except subprocess.TimeoutExpired:
            pass

    # A leader may exit while an inherited pipe remains open in a descendant. If a
    # reader has observed overflow, clean up the whole process group regardless of
    # the leader's state.
    if overflow.is_set():
        stop_process_group(process, 0.1)
    else:
        try:
            process.wait(timeout=0.5)
        except subprocess.TimeoutExpired:
            stop_process_group(process, 0.1)
        if process_group_exists(process.pid, process):
            stop_process_group(process, 0.1)

    for reader in readers:
        reader.join(timeout=1.0)
    if any(reader.is_alive() for reader in readers):
        stop_process_group(process, 0)
        for source in (process.stdout, process.stderr):
            if not source.closed:
                source.close()
        for reader in readers:
            reader.join(timeout=0.5)

    stdout_observed = int(stdout_state.get("bytes_observed", len(stdout)))
    stderr_observed = int(stderr_state.get("bytes_observed", len(stderr)))
    read_errors = [
        str(state["read_error"])
        for state in (stdout_state, stderr_state)
        if state.get("read_error")
    ]
    return BoundedCommandResult(
        process.poll(),
        bytes(stdout[:stdout_limit]),
        bytes(stderr[:stderr_limit]),
        stdout_observed > stdout_limit,
        stderr_observed > stderr_limit,
        timed_out,
        "; ".join(read_errors) or None,
    )


def bounded_capture_metadata(result: BoundedCommandResult) -> dict[str, Any]:
    return {
        "command_exit_code": result.returncode,
        "spawn_or_read_error": result.spawn_error,
        "stderr_truncated": result.stderr_truncated,
        "stdout_truncated": result.stdout_truncated,
        "timed_out": result.timed_out,
    }


def run_git(
    cwd: pathlib.Path,
    *args: str,
    timeout: float = 2.0,
    stdout_limit: int = MAX_GIT_METADATA_BYTES,
) -> BoundedCommandResult:
    return run_bounded_command(
        ["git", *args],
        cwd,
        timeout=timeout,
        stdout_limit=stdout_limit,
    )


def git_text(cwd: pathlib.Path, *args: str, timeout: float = 2.0) -> str | None:
    result = run_git(cwd, *args, timeout=timeout)
    if (
        result.returncode != 0
        or result.stdout_truncated
        or result.stderr_truncated
        or result.timed_out
        or result.spawn_error is not None
    ):
        return None
    return result.stdout.decode("utf-8", errors="replace").strip()


def git_snapshot(cwd: pathlib.Path) -> dict[str, Any]:
    status_result = run_git(
        cwd,
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        timeout=3.0,
    )
    status = status_result.stdout.decode("utf-8", errors="replace").strip()
    head = git_text(cwd, "rev-parse", "HEAD")
    branch = git_text(cwd, "branch", "--show-current")
    top = git_text(cwd, "rev-parse", "--show-toplevel")
    upstream = git_text(cwd, "rev-parse", "--abbrev-ref", "@{upstream}")
    dirty_entries = status.splitlines() if status else []
    return {
        "available": head is not None,
        "branch": branch or None,
        "dirty": bool(dirty_entries),
        "dirty_entries": dirty_entries,
        "head": head,
        "repo_root": top,
        "status_capture": bounded_capture_metadata(status_result),
        "upstream": upstream,
    }


def _sysctl(name: str) -> str | None:
    code, stdout, _ = run_metadata_command(["sysctl", "-n", name], pathlib.Path.cwd())
    if code != 0:
        return None
    return stdout.decode("utf-8", errors="replace").strip()


def machine_snapshot(cwd: pathlib.Path) -> dict[str, Any]:
    disk = shutil.disk_usage(cwd)
    snapshot: dict[str, Any] = {
        "cpu_count": os.cpu_count(),
        "disk": {
            "free_bytes": disk.free,
            "total_bytes": disk.total,
            "used_bytes": disk.used,
        },
        "machine": platform.machine(),
        "os": platform.system(),
        "os_release": platform.release(),
        "python": platform.python_version(),
    }
    try:
        snapshot["load_average"] = list(os.getloadavg())
    except (AttributeError, OSError):
        snapshot["load_average"] = None

    if platform.system() == "Darwin":
        memory = _sysctl("hw.memsize")
        snapshot["memory_total_bytes"] = int(memory) if memory and memory.isdigit() else None
        snapshot["swap"] = _sysctl("vm.swapusage")
    elif pathlib.Path("/proc/meminfo").is_file():
        values: dict[str, int] = {}
        try:
            for line in pathlib.Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
                name, raw = line.split(":", 1)
                number = raw.strip().split()[0]
                if number.isdigit():
                    values[name] = int(number) * 1024
        except (OSError, ValueError):
            values = {}
        snapshot["memory_total_bytes"] = values.get("MemTotal")
        snapshot["memory_available_bytes"] = values.get("MemAvailable")
        snapshot["swap_total_bytes"] = values.get("SwapTotal")
        snapshot["swap_free_bytes"] = values.get("SwapFree")
    return snapshot


def cache_snapshot(cwd: pathlib.Path, env: dict[str, str]) -> dict[str, Any]:
    configured_target = env.get("CARGO_TARGET_DIR")
    target = pathlib.Path(configured_target) if configured_target else cwd / "target"
    if not target.is_absolute():
        target = cwd / target
    target = target.resolve(strict=False)
    target_stat: dict[str, Any] = {
        "exists": target.exists(),
        "path": str(target),
        # Recursively measuring a 100+ GB target directory is itself a material
        # fast-loop cost, so only constant-time metadata is captured here.
        "recursive_size_bytes": None,
        "recursive_size_omitted_reason": "latency_budget",
    }
    try:
        stat = target.stat()
        target_stat["modified_utc"] = dt.datetime.fromtimestamp(
            stat.st_mtime, tz=dt.timezone.utc
        ).isoformat().replace("+00:00", "Z")
    except OSError:
        target_stat["modified_utc"] = None

    lock_path = cwd / "Cargo.lock"
    toolchain_path = cwd / "rust-toolchain.toml"
    return {
        "cargo_incremental": env.get("CARGO_INCREMENTAL"),
        "cargo_lock_sha256": sha256_file(lock_path),
        "rustc_wrapper": env.get("RUSTC_WRAPPER"),
        "rust_toolchain_sha256": sha256_file(toolchain_path),
        "sccache_available": shutil.which("sccache") is not None,
        "target": target_stat,
    }


def redact_argument(argument: str, secret_values: Iterable[str]) -> str:
    assignment = SECRET_ASSIGNMENT_RE.match(argument)
    if assignment:
        return f"{assignment.group('name')}=<redacted>"

    if "=" in argument:
        name, _ = argument.split("=", 1)
        if SECRET_FLAG_RE.match(name):
            return f"{name}=<redacted>"

    redacted = argument
    for secret in secret_values:
        if len(secret) >= 4 and secret in redacted:
            redacted = redacted.replace(secret, "<redacted>")
    # Strip the password from conventional user:password@ URLs even when the
    # credential was not sourced from an environment variable.
    redacted = re.sub(r"(://[^/@:\s]+:)[^/@\s]+(@)", r"\1<redacted>\2", redacted)
    return redacted


def redact_argv(argv: Sequence[str], env: dict[str, str]) -> list[str]:
    secrets = [value for name, value in env.items() if SECRET_NAME_RE.search(name) and value]
    result: list[str] = []
    hide_next = False
    for argument in argv:
        if hide_next:
            result.append("<redacted>")
            hide_next = False
            continue
        result.append(redact_argument(argument, secrets))
        if SECRET_FLAG_RE.match(argument):
            hide_next = True
    return result


def environment_snapshot(env: dict[str, str]) -> dict[str, Any]:
    safe_values = {
        name: value for name, value in env.items() if name in SAFE_ENV_VALUE_NAMES
    }
    credential_names = sorted(name for name in env if SECRET_NAME_RE.search(name))
    return {
        "credential_variable_names_present": credential_names,
        "credential_values_recorded": False,
        "full_environment_recorded": False,
        "safe_values": safe_values,
    }


def capture_git_diff(cwd: pathlib.Path, destination: pathlib.Path) -> dict[str, Any]:
    result = run_git(
        cwd,
        "diff",
        "--binary",
        "--no-ext-diff",
        "HEAD",
        "--",
        ".",
        timeout=10.0,
        stdout_limit=MAX_DIFF_BYTES,
    )
    payload = result.stdout
    if result.stdout_truncated:
        marker = b"\n# [run-evidence diff truncated at 16 MiB]\n"
        payload = payload[: MAX_DIFF_BYTES - len(marker)] + marker
    destination.write_bytes(payload)
    os.chmod(destination, 0o600)
    error_parts = []
    if result.timed_out:
        error_parts.append("git diff timed out")
    if result.stdout_truncated:
        error_parts.append("git diff exceeded the 16 MiB capture limit")
    if result.stderr_truncated:
        error_parts.append("git diff stderr exceeded its capture limit")
    if result.spawn_error:
        error_parts.append(result.spawn_error)
    if result.returncode != 0 and result.stderr:
        error_parts.append(result.stderr.decode("utf-8", errors="replace")[:2000])
    return {
        "bytes_captured": len(payload),
        "command_exit_code": result.returncode,
        "error": "; ".join(error_parts) or None,
        "note": "Tracked working-tree changes versus HEAD; untracked paths are listed separately.",
        "stderr_truncated": result.stderr_truncated,
        "timed_out": result.timed_out,
        "truncated": result.stdout_truncated,
    }


def write_untracked_paths(
    cwd: pathlib.Path, destination: pathlib.Path
) -> tuple[list[str], dict[str, Any]]:
    result = run_git(
        cwd,
        "ls-files",
        "--others",
        "--exclude-standard",
        timeout=3.0,
    )
    text = result.stdout.decode("utf-8", errors="replace")
    paths = text.splitlines() if text else []
    payload = result.stdout
    if result.stdout_truncated:
        marker = b"\n# [run-evidence untracked path list truncated]\n"
        payload = payload[: MAX_GIT_METADATA_BYTES - len(marker)] + marker
    destination.write_bytes(payload)
    os.chmod(destination, 0o600)
    return paths, bounded_capture_metadata(result)


def validate_events_jsonl(path: pathlib.Path) -> dict[str, Any]:
    """Validate JSONL and detect the current and common legacy run lifecycle names.

    Detection is deliberately advisory: generic run-evidence commands need not emit
    events, and older Roko binaries used several spellings. The result is recorded but
    never changes the wrapped command's exit status.
    """

    result: dict[str, Any] = {
        "path": path.name,
        "present": path.is_file(),
        "bytes": path.stat().st_size if path.is_file() else 0,
        "nonblank_lines": 0,
        "valid_jsonl": None,
        "parse_error": None,
        "event_type_field_missing": 0,
        "run_start_count": 0,
        "run_terminal_count": 0,
        "exactly_one_terminal": None,
        "lifecycle_detection": "best_effort_known_type_names",
    }
    if not path.is_file():
        return result

    start_names = {
        "run.start",
        "run.started",
        "run_start",
        "run_started",
        "runstart",
        "runstarted",
    }
    terminal_names = {
        "run.cancelled",
        "run.canceled",
        "run.complete",
        "run.completed",
        "run.failed",
        "run.terminal",
        "run_cancelled",
        "run_canceled",
        "run_complete",
        "run_completed",
        "run_failed",
        "runcancelled",
        "runcanceled",
        "runcomplete",
        "runcompleted",
        "runfailed",
    }
    lifecycle_detected = False
    result["valid_jsonl"] = True
    try:
        with path.open("r", encoding="utf-8") as stream:
            for line_number, line in enumerate(stream, start=1):
                if not line.strip():
                    continue
                result["nonblank_lines"] += 1
                try:
                    event = json.loads(line)
                except (json.JSONDecodeError, UnicodeDecodeError) as error:
                    result["valid_jsonl"] = False
                    result["parse_error"] = {
                        "line": line_number,
                        "column": getattr(error, "colno", None),
                        "message": str(error)[:500],
                    }
                    break

                event_type: Any = None
                if isinstance(event, dict):
                    event_type = event.get("type") or event.get("event_type")
                    nested = event.get("event")
                    if event_type is None and isinstance(nested, dict):
                        event_type = nested.get("type") or nested.get("event_type")
                if not isinstance(event_type, str):
                    result["event_type_field_missing"] += 1
                    continue
                normalized = event_type.strip().lower().replace("-", ".")
                if normalized in start_names:
                    result["run_start_count"] += 1
                    lifecycle_detected = True
                if normalized in terminal_names:
                    result["run_terminal_count"] += 1
                    lifecycle_detected = True
    except (OSError, UnicodeDecodeError) as error:
        result["valid_jsonl"] = False
        result["parse_error"] = {"line": None, "column": None, "message": str(error)[:500]}

    if lifecycle_detected:
        result["exactly_one_terminal"] = result["run_terminal_count"] == 1
    return result


def pump_output(
    source: BinaryIO,
    artifact: BinaryIO,
    live: Any,
    stats: dict[str, Any],
    limit: int = MAX_OUTPUT_BYTES,
) -> None:
    observed = 0
    captured = 0
    truncated = False
    payload_limit = max(0, limit - len(OUTPUT_TRUNCATION_MARKER))
    try:
        while True:
            chunk = source.read(64 * 1024)
            if not chunk:
                break
            observed += len(chunk)
            remaining = max(0, payload_limit - captured)
            captured_chunk = chunk[:remaining]
            if captured_chunk:
                artifact.write(captured_chunk)
                captured += len(captured_chunk)
                try:
                    live_buffer = getattr(live, "buffer", live)
                    live_buffer.write(captured_chunk)
                    live_buffer.flush()
                except (BrokenPipeError, OSError, TypeError):
                    # A closed caller pipe must not prevent preservation in the bundle.
                    pass
            if len(captured_chunk) < len(chunk) and not truncated:
                truncated = True
                artifact.write(OUTPUT_TRUNCATION_MARKER)
                captured += len(OUTPUT_TRUNCATION_MARKER)
                try:
                    live_buffer = getattr(live, "buffer", live)
                    live_buffer.write(OUTPUT_TRUNCATION_MARKER)
                    live_buffer.flush()
                except (BrokenPipeError, OSError, TypeError):
                    pass
            artifact.flush()
    finally:
        stats.update(
            {
                "bytes_captured": captured,
                "bytes_observed": observed,
                "limit_bytes": limit,
                "truncated": truncated,
            }
        )
        try:
            source.close()
        except OSError:
            pass


def process_group_exists(pgid: int, process: subprocess.Popen[bytes]) -> bool:
    if hasattr(os, "killpg"):
        try:
            os.killpg(pgid, 0)
            return True
        except ProcessLookupError:
            return False
        except PermissionError:
            # It exists even if a future signal would be denied.
            return True
    return process.poll() is None


def signal_process_group(
    process: subprocess.Popen[bytes], pgid: int, signum: int
) -> None:
    if hasattr(os, "killpg"):
        try:
            os.killpg(pgid, signum)
        except ProcessLookupError:
            pass
        return
    if process.poll() is not None:
        return
    try:
        if signum == signal.SIGKILL:
            process.kill()
        elif signum == signal.SIGTERM:
            process.terminate()
        else:
            process.send_signal(signum)
    except ProcessLookupError:
        pass


def stop_process_group(process: subprocess.Popen[bytes], grace_seconds: float) -> None:
    pgid = process.pid
    if process_group_exists(pgid, process):
        signal_process_group(process, pgid, signal.SIGTERM)
    grace_deadline = time.monotonic() + max(0.0, grace_seconds)
    while process_group_exists(pgid, process) and time.monotonic() < grace_deadline:
        # poll() reaps the group leader while group existence tracks descendants.
        process.poll()
        time.sleep(0.02)
    if process_group_exists(pgid, process):
        signal_process_group(process, pgid, signal.SIGKILL)
    kill_deadline = time.monotonic() + 1.0
    while process_group_exists(pgid, process) and time.monotonic() < kill_deadline:
        process.poll()
        time.sleep(0.02)
    try:
        process.wait(timeout=1.0)
    except subprocess.TimeoutExpired:
        # Last-resort cross-platform leader cleanup; POSIX descendants were already
        # addressed through their process group above.
        try:
            process.kill()
            process.wait(timeout=1.0)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            pass


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a command with a deadline and write a private evidence bundle.",
        epilog=(
            "The command deadline excludes a few seconds of final Git/artifact capture. "
            "No endpoint probes, builds, tests, cleanup, or environment dump are performed."
        ),
    )
    parser.add_argument(
        "--deadline",
        type=int,
        default=DEFAULT_DEADLINE_SECONDS,
        metavar="SECONDS",
        help="hard command deadline (default: 300)",
    )
    parser.add_argument(
        "--grace",
        type=float,
        default=3.0,
        metavar="SECONDS",
        help="SIGTERM grace before SIGKILL (default: 3)",
    )
    parser.add_argument(
        "--label", default="command", help="short label included in the run ID"
    )
    parser.add_argument(
        "--bundle-root",
        default=".roko/runs",
        metavar="DIR",
        help="bundle parent directory, relative to --cwd (default: .roko/runs)",
    )
    parser.add_argument(
        "--cwd",
        default=".",
        metavar="DIR",
        help="command and Git working directory (default: current directory)",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if args.command and args.command[0] == "--":
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")
    if args.deadline <= 0:
        parser.error("--deadline must be greater than zero")
    if args.grace < 0:
        parser.error("--grace cannot be negative")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    # Prevent a permissive caller umask from briefly exposing newly-created
    # artifacts before their explicit modes are applied.
    os.umask(0o077)
    cwd = pathlib.Path(args.cwd).expanduser().resolve(strict=False)
    if not cwd.is_dir():
        print(f"run-evidence: working directory does not exist: {cwd}", file=sys.stderr)
        return 2

    label = re.sub(r"[^A-Za-z0-9._-]+", "-", args.label).strip("-.")[:48] or "command"
    root = pathlib.Path(args.bundle_root).expanduser()
    if not root.is_absolute():
        root = cwd / root
    root.mkdir(parents=True, exist_ok=True)
    if root.is_symlink() or not root.is_dir():
        print(f"run-evidence: bundle root must be a real directory: {root}", file=sys.stderr)
        return 2

    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_id_base = f"{stamp}-{label}-{os.getpid()}"
    bundle = root / run_id_base
    suffix = 1
    while bundle.exists():
        bundle = root / f"{run_id_base}-{suffix}"
        suffix += 1
    bundle.mkdir(mode=0o700)
    run_id = bundle.name

    env = dict(os.environ)
    env["ROKO_EVIDENCE_RUN_ID"] = run_id
    env["ROKO_EVIDENCE_BUNDLE"] = str(bundle)
    execution_argv = [argument.replace("{bundle}", str(bundle)) for argument in args.command]
    redacted_argv = redact_argv(execution_argv, env)

    git_before = git_snapshot(cwd)
    machine = machine_snapshot(cwd)
    cache = cache_snapshot(cwd, env)
    started_utc = utc_now()
    overall_started = time.monotonic()

    write_json(bundle / "git-before.json", git_before)
    write_json(bundle / "machine.json", machine)
    write_json(bundle / "cache.json", cache)
    write_text(bundle / "command.txt", " ".join(shlex.quote(arg) for arg in redacted_argv) + "\n")
    capture_git_diff(cwd, bundle / "diff-before.patch")
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "started_utc": started_utc,
        "cwd": str(cwd),
        "bundle": str(bundle),
        "deadline_seconds": args.deadline,
        "command": redacted_argv,
        "command_redacted": redacted_argv != execution_argv,
        "environment": environment_snapshot(env),
        "git_before": git_before,
        "machine": machine,
        "cache": cache,
        "artifact_security": {
            "directory_mode": "0700",
            "file_mode": "0600",
            "content_warning": "Command output and Git diffs are captured verbatim and may contain sensitive data.",
        },
    }
    write_json(bundle / "manifest.json", manifest)
    append_jsonl(
        bundle / "status.jsonl",
        {
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "state": "started",
            "terminal": False,
            "timestamp_utc": started_utc,
        },
    )

    print(f"[evidence] run_id={run_id}", file=sys.stderr)
    print(f"[evidence] bundle={bundle}", file=sys.stderr)
    print(f"[evidence] deadline={args.deadline}s", file=sys.stderr)

    cancelled_signal: list[int] = []

    def request_cancel(signum: int, _frame: Any) -> None:
        if not cancelled_signal:
            cancelled_signal.append(signum)

    previous_handlers: dict[int, Any] = {}
    for signum in (signal.SIGINT, signal.SIGTERM):
        previous_handlers[signum] = signal.getsignal(signum)
        signal.signal(signum, request_cancel)

    command_started_utc = utc_now()
    command_started = time.monotonic()
    process: subprocess.Popen[bytes] | None = None
    timed_out = False
    descendants_terminated_after_leader_exit = False
    spawn_error: str | None = None
    return_code: int | None = None
    stdout_path = bundle / "stdout.log"
    stderr_path = bundle / "stderr.log"
    stdout_path.touch(mode=0o600)
    stderr_path.touch(mode=0o600)
    threads: list[threading.Thread] = []
    stdout_capture: dict[str, Any] = {
        "bytes_captured": 0,
        "bytes_observed": 0,
        "limit_bytes": MAX_OUTPUT_BYTES,
        "truncated": False,
    }
    stderr_capture: dict[str, Any] = dict(stdout_capture)

    try:
        with stdout_path.open("wb") as stdout_artifact, stderr_path.open("wb") as stderr_artifact:
            try:
                process = subprocess.Popen(
                    execution_argv,
                    cwd=str(cwd),
                    env=env,
                    stdin=None,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    start_new_session=True,
                )
            except OSError as error:
                spawn_error = f"{type(error).__name__}: {error}"
                stderr_artifact.write((spawn_error + "\n").encode("utf-8", errors="replace"))
                stderr_artifact.flush()
                print(f"run-evidence: {spawn_error}", file=sys.stderr)

            if process is not None:
                assert process.stdout is not None
                assert process.stderr is not None
                threads = [
                    threading.Thread(
                        target=pump_output,
                        args=(process.stdout, stdout_artifact, sys.stdout, stdout_capture),
                        daemon=True,
                        name="evidence-stdout",
                    ),
                    threading.Thread(
                        target=pump_output,
                        args=(process.stderr, stderr_artifact, sys.stderr, stderr_capture),
                        daemon=True,
                        name="evidence-stderr",
                    ),
                ]
                for thread in threads:
                    thread.start()

                command_deadline = command_started + args.deadline
                while process.poll() is None:
                    if cancelled_signal:
                        stop_process_group(process, args.grace)
                        break
                    remaining = command_deadline - time.monotonic()
                    if remaining <= 0:
                        timed_out = True
                        print(
                            f"[evidence] deadline reached after {args.deadline}s; terminating process group",
                            file=sys.stderr,
                        )
                        stop_process_group(process, args.grace)
                        break
                    try:
                        process.wait(timeout=min(0.25, remaining))
                    except subprocess.TimeoutExpired:
                        pass
                # The group leader can exit successfully after leaving children
                # behind. Treat the process group as the execution scope and clean
                # those descendants before finalizing a successful-looking run.
                if process.poll() is not None and process_group_exists(
                    process.pid, process
                ):
                    descendants_terminated_after_leader_exit = True
                    stop_process_group(process, args.grace)
                return_code = process.poll()
                if return_code is None:
                    stop_process_group(process, 0)
                    return_code = process.poll()

                for thread in threads:
                    thread.join(timeout=2.0)
                for source in (process.stdout, process.stderr):
                    if source and not source.closed:
                        source.close()
    except Exception as error:  # Preserve evidence for wrapper failures, too.
        spawn_error = f"{type(error).__name__}: {error}"
        if process is not None:
            stop_process_group(process, args.grace)
            return_code = process.poll()
        try:
            with stderr_path.open("ab") as stream:
                stream.write((spawn_error + "\n").encode("utf-8", errors="replace"))
        except OSError:
            pass
        print(f"run-evidence: internal capture error: {spawn_error}", file=sys.stderr)
    finally:
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)

    command_finished_utc = utc_now()
    command_duration = max(0.0, time.monotonic() - command_started)
    if cancelled_signal:
        terminal_state = "cancelled"
        shell_exit_code = 128 + cancelled_signal[0]
    elif timed_out:
        terminal_state = "timeout"
        shell_exit_code = 124
    elif spawn_error is not None:
        terminal_state = "spawn_error"
        shell_exit_code = 127
    elif return_code == 0:
        terminal_state = "succeeded"
        shell_exit_code = 0
    elif return_code is not None and return_code < 0:
        terminal_state = "signalled"
        shell_exit_code = 128 + (-return_code)
    else:
        terminal_state = "failed"
        shell_exit_code = return_code if return_code is not None else 1

    git_after = git_snapshot(cwd)
    write_json(bundle / "git-after.json", git_after)
    diff_meta = capture_git_diff(cwd, bundle / "diff.patch")
    untracked, untracked_capture = write_untracked_paths(cwd, bundle / "untracked.txt")
    diff_stat_result = run_git(
        cwd,
        "diff",
        "--stat",
        "HEAD",
        "--",
        ".",
        timeout=3.0,
    )
    diff_stat_payload = diff_stat_result.stdout
    if diff_stat_result.stdout_truncated:
        marker = b"\n# [run-evidence diff stat truncated]\n"
        diff_stat_payload = (
            diff_stat_payload[: MAX_GIT_METADATA_BYTES - len(marker)] + marker
        )
    (bundle / "diff-stat.txt").write_bytes(diff_stat_payload)
    os.chmod(bundle / "diff-stat.txt", 0o600)
    events_validation = validate_events_jsonl(bundle / "events.jsonl")
    write_json(bundle / "events-validation.json", events_validation)
    attempt_evidence_dir = bundle / "attempt-evidence"
    attempt_evidence = (
        sorted(
            str(path.relative_to(bundle))
            for path in attempt_evidence_dir.iterdir()
            if path.is_file()
        )
        if attempt_evidence_dir.is_dir()
        else []
    )

    timings = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "started_utc": started_utc,
        "command_started_utc": command_started_utc,
        "command_finished_utc": command_finished_utc,
        "command_duration_ms": round(command_duration * 1000),
        "deadline_seconds": args.deadline,
        "deadline_exceeded": timed_out,
        "bundle_finalize_duration_is_in_total_only": True,
        "total_duration_ms": round((time.monotonic() - overall_started) * 1000),
    }
    write_json(bundle / "timings.json", timings)

    command_record = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "command_id": "command-1",
        "argv": redacted_argv,
        "argv_redacted": redacted_argv != execution_argv,
        "cwd": str(cwd),
        "pid": process.pid if process is not None else None,
        "started_utc": command_started_utc,
        "finished_utc": command_finished_utc,
        "duration_ms": round(command_duration * 1000),
        "deadline_seconds": args.deadline,
        "exit_code": return_code,
        "shell_exit_code": shell_exit_code,
        "state": terminal_state,
        "timed_out": timed_out,
        "descendants_terminated_after_leader_exit": descendants_terminated_after_leader_exit,
        "signal": cancelled_signal[0] if cancelled_signal else (-return_code if return_code and return_code < 0 else None),
        "stdout": "stdout.log",
        "stderr": "stderr.log",
        "stdout_capture": stdout_capture,
        "stderr_capture": stderr_capture,
    }
    append_jsonl(bundle / "commands.jsonl", command_record)

    summary = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "terminal": True,
        "state": terminal_state,
        "exit_code": shell_exit_code,
        "process_exit_code": return_code,
        "timed_out": timed_out,
        "descendants_terminated_after_leader_exit": descendants_terminated_after_leader_exit,
        "cancelled_signal": cancelled_signal[0] if cancelled_signal else None,
        "spawn_error": spawn_error,
        "duration_ms": timings["command_duration_ms"],
        "git": {
            "head_before": git_before.get("head"),
            "head_after": git_after.get("head"),
            "initial_dirty_entries": git_before.get("dirty_entries", []),
            "final_dirty_entries": git_after.get("dirty_entries", []),
            "untracked_paths": untracked,
            "untracked_capture": untracked_capture,
            "diff": diff_meta,
            "diff_stat_capture": bounded_capture_metadata(diff_stat_result),
        },
        "events": events_validation,
        "artifacts": {
            "command": "command.txt",
            "events": "events.jsonl" if events_validation["present"] else None,
            "events_validation": "events-validation.json",
            "attempt_evidence": attempt_evidence,
            "stderr": "stderr.log",
            "stdout": "stdout.log",
            "timings": "timings.json",
            "tracked_diff": "diff.patch",
            "untracked_path_list": "untracked.txt",
        },
        "stdout_bytes": stdout_path.stat().st_size if stdout_path.exists() else 0,
        "stderr_bytes": stderr_path.stat().st_size if stderr_path.exists() else 0,
        "stdout_capture": stdout_capture,
        "stderr_capture": stderr_capture,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }
    write_json(bundle / "summary.json", summary)
    append_jsonl(
        bundle / "status.jsonl",
        {
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "state": terminal_state,
            "terminal": True,
            "exit_code": shell_exit_code,
            "timestamp_utc": utc_now(),
        },
    )

    print(
        f"[evidence] terminal={terminal_state} exit={shell_exit_code} duration={command_duration:.1f}s",
        file=sys.stderr,
    )
    print(f"[evidence] summary={bundle / 'summary.json'}", file=sys.stderr)
    return shell_exit_code


if __name__ == "__main__":
    raise SystemExit(main())
