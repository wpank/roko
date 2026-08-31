#!/usr/bin/env python3
"""Run one command with a deadline and preserve a small, run-scoped evidence bundle.

This deliberately does not build or clean anything. It is the generic capture layer
used by ``./dev.sh fast`` and may also be used directly via ``./dev.sh run-evidence``;
safe GET probes and explicit screenshot/CLI hooks are opt-in.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import ipaddress
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
import urllib.error
import urllib.parse
import urllib.request
import uuid
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from typing import Any, BinaryIO, Iterable, NamedTuple, Sequence


SCHEMA_VERSION = 2
DEFAULT_DEADLINE_SECONDS = 300
MAX_DIFF_BYTES = 16 * 1024 * 1024
MAX_GIT_METADATA_BYTES = 4 * 1024 * 1024
MAX_OUTPUT_BYTES = 16 * 1024 * 1024
MAX_METADATA_ERROR_BYTES = 64 * 1024
MAX_ENDPOINT_BYTES = 1024 * 1024
MAX_HOOK_OUTPUT_BYTES = 2 * 1024 * 1024
MAX_SCREENSHOT_BYTES = 8 * 1024 * 1024
MAX_FILTERED_LOG_BYTES = 8 * 1024 * 1024
MAX_JSON_ARTIFACT_BYTES = 4 * 1024 * 1024
MAX_BUNDLE_BYTES = 128 * 1024 * 1024
MAX_ENDPOINTS = 32
DEFAULT_MIN_FREE_BYTES = 5 * 1024 * 1024 * 1024
DEFAULT_MIN_FREE_PERCENT = 3.0
OUTPUT_TRUNCATION_MARKER = b"\n[run-evidence output truncated at 16 MiB]\n"
DEFAULT_APPEND_LOGS = (
    ".roko/events.jsonl",
    ".roko/state/run-ledger.jsonl",
    ".roko/run-ledger.jsonl",
)
DEFAULT_SAFE_GET_PATHS = (
    "/health",
    "/ready",
    "/metrics",
    "/api/health",
    "/api/openapi.json",
    "/api/status",
    "/api/statehub/snapshot",
    "/api/plans",
    "/api/gates/summary",
    "/api/metrics/summary",
    "/api/providers/health",
    "/api/projections/catalog",
    "/api/runs/{run_id}",
    "/api/runs/{run_id}/events",
    "/api/runs/{run_id}/gates",
    "/api/runs/{run_id}/logs",
    "/api/runs/{run_id}/metrics",
    "/api/runs/{run_id}/artifacts",
    "/api/runs/{run_id}/screenshots",
)
RUN_START_NAMES = {
    "run.start",
    "run.started",
    "run_start",
    "run_started",
    "runstart",
    "runstarted",
}
RUN_TERMINAL_NAMES = {
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
SAFE_ENV_VALUE_NAMES = {
    "CARGO_INCREMENTAL",
    "CARGO_TARGET_DIR",
    "CI",
    "NO_COLOR",
    "ROKO_EVIDENCE_BUNDLE",
    "ROKO_EVIDENCE_RUN_ID",
    "ROKO_EVIDENCE_ALLOW_LOW_DISK",
    "ROKO_AGENT_SHARED_TARGET",
    "ROKO_FAST_MODE",
    "ROKO_FAST_PLAN_DEADLINE_SECS",
    "ROKO_GATE_MODE",
    "ROKO_COMPILE_CONCURRENCY",
    "ROKO_FAST_SETTLEMENT_HEADROOM_SECS",
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
SECRET_VALUE_PATTERNS = (
    ("private_key", re.compile(rb"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----")),
    ("aws_access_key", re.compile(rb"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b")),
    ("provider_token", re.compile(rb"\b(?:sk|gh[pousr]|xox[baprs])-[A-Za-z0-9_-]{12,}\b")),
    ("bearer_token", re.compile(rb"(?i)\bBearer\s+(?!<redacted>)[A-Za-z0-9._~+/=-]{12,}")),
    (
        "named_secret",
        re.compile(
            rb"(?i)(?:api[_-]?key|password|private[_-]?key|secret|token)"
            rb"\s*[=:]\s*[\"']?(?!<redacted>|false\b|true\b|null\b)"
            rb"[A-Za-z0-9._~+/=-]{8,}"
        ),
    ),
    (
        "secret_flag_value",
        re.compile(
            rb"(?i)--(?:api[_-]?key|auth|bearer|credential|password|private[_-]?key|secret|token)"
            rb"\s+(?!<redacted>)[A-Za-z0-9._~+/=-]{8,}"
        ),
    ),
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


def write_jsonl_records(path: pathlib.Path, records: Iterable[dict[str, Any]]) -> None:
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    with temporary.open("w", encoding="utf-8") as stream:
        for record in records:
            stream.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
            stream.write("\n")
    os.replace(temporary, path)
    os.chmod(path, 0o600)


def bounded_read(path: pathlib.Path, limit: int) -> tuple[bytes, bool]:
    with path.open("rb") as stream:
        payload = stream.read(limit + 1)
    return payload[:limit], len(payload) > limit


def truncate_jsonl(path: pathlib.Path, limit: int) -> dict[str, Any]:
    try:
        original_bytes = path.stat().st_size
    except OSError:
        return {"path": str(path), "original_bytes": 0, "truncated": False}
    if original_bytes <= limit:
        return {"path": str(path), "original_bytes": original_bytes, "truncated": False}
    payload, _ = bounded_read(path, limit)
    last_newline = payload.rfind(b"\n")
    safe = payload[: last_newline + 1] if last_newline >= 0 else b""
    path.write_bytes(safe)
    os.chmod(path, 0o600)
    return {
        "path": str(path),
        "original_bytes": original_bytes,
        "captured_bytes": len(safe),
        "truncated": True,
        "reason": f"JSONL exceeded {limit} byte direct-write limit",
    }


def redact_text(value: str, env: dict[str, str]) -> str:
    redacted = redact_argument(value, (raw for name, raw in env.items() if SECRET_NAME_RE.search(name)))
    redacted = re.sub(
        r"(?i)((?:api[_-]?key|password|private[_-]?key|secret|token)\s*[=:]\s*)"
        r"([^\s,;]+)",
        r"\1<redacted>",
        redacted,
    )
    redacted = re.sub(r"(?i)(\bBearer\s+)[A-Za-z0-9._~+/=-]+", r"\1<redacted>", redacted)
    redacted = re.sub(
        r"(?i)(--(?:api[_-]?key|auth|bearer|credential|password|private[_-]?key|secret|token)\s+)\S+",
        r"\1<redacted>",
        redacted,
    )
    return redacted


def redact_json(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            str(key): "<redacted>" if SECRET_NAME_RE.search(str(key)) else redact_json(item)
            for key, item in value.items()
        }
    if isinstance(value, list):
        return [redact_json(item) for item in value]
    if isinstance(value, str):
        return re.sub(r"(://[^/@:\s]+:)[^/@\s]+(@)", r"\1<redacted>\2", value)
    return value


def normalized_event_type(event: Any) -> str | None:
    event_type: Any = None
    if isinstance(event, dict):
        event_type = event.get("type") or event.get("event_type")
        nested = event.get("event")
        if event_type is None and isinstance(nested, dict):
            event_type = nested.get("type") or nested.get("event_type")
    if not isinstance(event_type, str):
        return None
    return event_type.strip().lower().replace("-", ".")


def collect_named_values(value: Any, key_name: str) -> set[str]:
    found: set[str] = set()
    if isinstance(value, dict):
        for key, item in value.items():
            if key == key_name and isinstance(item, (str, int)):
                found.add(str(item))
            found.update(collect_named_values(item, key_name))
    elif isinstance(value, list):
        for item in value:
            found.update(collect_named_values(item, key_name))
    return found


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
    env: dict[str, str] | None = None,
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
            env=env,
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


def truthy_env(value: str | None) -> bool:
    return bool(value and value.strip().lower() in {"1", "true", "yes", "on"})


def resource_admission(
    cwd: pathlib.Path,
    env: dict[str, str],
    machine: dict[str, Any],
    cache: dict[str, Any],
    *,
    enabled: bool,
    allow_low_disk: bool,
    min_free_bytes: int,
    min_free_percent: float,
) -> dict[str, Any]:
    target = pathlib.Path(cache.get("target", {}).get("path") or cwd / "target")
    probe_path = target if target.exists() else cwd
    disk = shutil.disk_usage(probe_path)
    free_percent = (disk.free / disk.total * 100.0) if disk.total else 0.0
    target_size_bytes: int | None = None
    target_size_capture: dict[str, Any] = {
        "command_exit_code": None,
        "timed_out": False,
        "stdout_truncated": False,
        "stderr_truncated": False,
        "spawn_or_read_error": None,
    }
    if target.is_dir() and enabled:
        du = run_bounded_command(
            ["du", "-sk", str(target)],
            cwd,
            timeout=2.0,
            stdout_limit=4096,
            stderr_limit=MAX_METADATA_ERROR_BYTES,
        )
        target_size_capture = bounded_capture_metadata(du)
        if du.returncode == 0 and not du.timed_out and not du.stdout_truncated:
            first = du.stdout.decode("utf-8", errors="replace").split(None, 1)
            if first and first[0].isdigit():
                target_size_bytes = int(first[0]) * 1024
    pressure_reasons = []
    if disk.free < min_free_bytes:
        pressure_reasons.append(f"free bytes {disk.free} below minimum {min_free_bytes}")
    if free_percent < min_free_percent:
        pressure_reasons.append(
            f"free percent {free_percent:.2f} below minimum {min_free_percent:.2f}"
        )
    override = allow_low_disk or truthy_env(env.get("ROKO_EVIDENCE_ALLOW_LOW_DISK"))
    admitted = not enabled or not pressure_reasons or override
    return {
        "schema_version": SCHEMA_VERSION,
        "enabled": enabled,
        "admitted": admitted,
        "override": override,
        "pressure_detected": bool(pressure_reasons),
        "pressure_reasons": pressure_reasons,
        "thresholds": {
            "min_free_bytes": min_free_bytes,
            "min_free_percent": min_free_percent,
        },
        "disk": {
            "path": str(probe_path),
            "total_bytes": disk.total,
            "used_bytes": disk.used,
            "free_bytes": disk.free,
            "free_percent": round(free_percent, 3),
        },
        "swap": {
            "description": machine.get("swap"),
            "total_bytes": machine.get("swap_total_bytes"),
            "free_bytes": machine.get("swap_free_bytes"),
        },
        "memory": {
            "total_bytes": machine.get("memory_total_bytes"),
            "available_bytes": machine.get("memory_available_bytes"),
        },
        "target": {
            "path": str(target),
            "exists": target.exists(),
            "allocated_size_bytes": target_size_bytes,
            "size_capture": target_size_capture,
        },
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
    redacted = re.sub(r"(?i)(\bBearer\s+)[A-Za-z0-9._~+/=-]+", r"\1<redacted>", redacted)
    redacted = re.sub(
        r"(?i)([?&](?:api[_-]?key|auth|password|secret|token)=)[^&#\s]+",
        r"\1<redacted>",
        redacted,
    )
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
        if SECRET_FLAG_RE.match(argument) or argument in {"-H", "--header"}:
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
        "schema_version": SCHEMA_VERSION,
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
        "run_ids": [],
        "run_id_field_missing": 0,
        "timestamp_order_valid": None,
        "lifecycle_errors": [],
        "lifecycle_detection": "best_effort_known_type_names",
    }
    if not path.is_file():
        return result

    lifecycle_detected = False
    run_ids: set[str] = set()
    last_timestamp_ms: int | float | None = None
    ordered = True
    attempt_starts: Counter[str] = Counter()
    attempt_terminals: Counter[str] = Counter()
    dispatch_starts: Counter[str] = Counter()
    dispatch_completions: Counter[str] = Counter()
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

                event_run_ids = collect_named_values(event, "run_id")
                if event_run_ids:
                    run_ids.update(event_run_ids)
                else:
                    result["run_id_field_missing"] += 1
                if isinstance(event, dict):
                    timestamp_ms = event.get("timestamp_ms") or event.get("monotonic_ms")
                    if isinstance(timestamp_ms, (int, float)):
                        if last_timestamp_ms is not None and timestamp_ms < last_timestamp_ms:
                            ordered = False
                        last_timestamp_ms = timestamp_ms
                normalized = normalized_event_type(event)
                if normalized is None:
                    result["event_type_field_missing"] += 1
                    continue
                if normalized in RUN_START_NAMES:
                    result["run_start_count"] += 1
                    lifecycle_detected = True
                if normalized in RUN_TERMINAL_NAMES:
                    result["run_terminal_count"] += 1
                    lifecycle_detected = True
                if isinstance(event, dict):
                    attempt_key = "/".join(
                        str(event.get(key, ""))
                        for key in ("plan_id", "task_id", "attempt")
                    )
                    if all(event.get(key) is not None for key in ("plan_id", "task_id", "attempt")):
                        if normalized == "task.attempt.started":
                            attempt_starts[attempt_key] += 1
                        elif normalized == "task.attempt.completed":
                            attempt_terminals[attempt_key] += 1
                        elif normalized == "agent.dispatch.started":
                            dispatch_starts[attempt_key] += 1
                        elif normalized == "agent.dispatch.completed":
                            dispatch_completions[attempt_key] += 1
    except (OSError, UnicodeDecodeError) as error:
        result["valid_jsonl"] = False
        result["parse_error"] = {"line": None, "column": None, "message": str(error)[:500]}

    if lifecycle_detected:
        result["exactly_one_terminal"] = result["run_terminal_count"] == 1
    result["run_ids"] = sorted(run_ids)
    result["timestamp_order_valid"] = ordered if result["nonblank_lines"] else None
    lifecycle_errors = []
    for key in sorted(set(attempt_starts) | set(attempt_terminals)):
        if attempt_starts[key] != 1 or attempt_terminals[key] != 1:
            lifecycle_errors.append(
                f"attempt {key} has starts={attempt_starts[key]} terminals={attempt_terminals[key]}"
            )
    for key in sorted(set(dispatch_starts) | set(dispatch_completions)):
        if dispatch_starts[key] != 1 or dispatch_completions[key] != 1:
            lifecycle_errors.append(
                f"dispatch {key} has starts={dispatch_starts[key]} completions={dispatch_completions[key]}"
            )
    result["lifecycle_errors"] = lifecycle_errors
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


class StatusSampler:
    def __init__(
        self,
        source: pathlib.Path,
        destination: pathlib.Path,
        evidence_run_id: str,
        interval: float,
    ) -> None:
        self.source = source
        self.destination = destination
        self.evidence_run_id = evidence_run_id
        self.interval = max(0.1, interval)
        self.stop_event = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True, name="evidence-status")
        self.source_run_ids: set[str] = set()
        self.samples = 0
        self.parse_errors = 0
        self._last_signature: tuple[int, int] | None = None
        try:
            stat = source.stat()
            self._baseline_signature = (stat.st_mtime_ns, stat.st_size)
        except OSError:
            self._baseline_signature = None

    def start(self) -> None:
        self.destination.touch(mode=0o600, exist_ok=True)
        os.chmod(self.destination, 0o600)
        self.thread.start()

    def stop(self) -> None:
        self.stop_event.set()
        self.thread.join(timeout=max(1.0, self.interval * 2))
        self._sample_once()

    def _run(self) -> None:
        while not self.stop_event.wait(self.interval):
            self._sample_once()

    def _sample_once(self) -> None:
        try:
            stat = self.source.stat()
            signature = (stat.st_mtime_ns, stat.st_size)
            if signature == self._baseline_signature or signature == self._last_signature:
                return
            self._last_signature = signature
            payload, truncated = bounded_read(self.source, MAX_JSON_ARTIFACT_BYTES)
            if truncated:
                self.parse_errors += 1
                return
            status = json.loads(payload.decode("utf-8"))
            if not isinstance(status, dict):
                self.parse_errors += 1
                return
            source_run_id = status.get("run_id")
            if not isinstance(source_run_id, str) or not source_run_id:
                self.parse_errors += 1
                return
            if self.source_run_ids and source_run_id not in self.source_run_ids:
                # A concurrent or later run replaced the global status file. Do not
                # mix its samples into this bundle.
                return
            self.source_run_ids.add(source_run_id)
            append_jsonl(
                self.destination,
                {
                    "schema_version": SCHEMA_VERSION,
                    "run_id": self.evidence_run_id,
                    "source_run_id": source_run_id,
                    "sampled_utc": utc_now(),
                    "sampled_monotonic_ns": time.monotonic_ns(),
                    "source": str(self.source),
                    "status": redact_json(status),
                },
            )
            self.samples += 1
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            self.parse_errors += 1


class ProcessSampler:
    def __init__(
        self,
        pgid: int,
        destination: pathlib.Path,
        run_id: str,
        env: dict[str, str],
        interval: float = 1.0,
    ) -> None:
        self.pgid = pgid
        self.destination = destination
        self.run_id = run_id
        self.env = env
        self.interval = max(0.25, interval)
        self.stop_event = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True, name="evidence-processes")
        self.sample_count = 0
        self.max_processes = 0
        self.observed_pids: set[int] = set()
        self.commands: set[str] = set()
        self.truncated = False

    def start(self) -> None:
        self.destination.touch(mode=0o600, exist_ok=True)
        os.chmod(self.destination, 0o600)
        self._sample_once()
        self.thread.start()

    def stop(self) -> None:
        self.stop_event.set()
        self.thread.join(timeout=max(1.0, self.interval * 2))
        self._sample_once()

    def _run(self) -> None:
        while not self.stop_event.wait(self.interval):
            self._sample_once()

    def _sample_once(self) -> None:
        try:
            if self.destination.stat().st_size >= MAX_JSON_ARTIFACT_BYTES - 64 * 1024:
                self.truncated = True
                return
        except OSError:
            pass
        result = run_bounded_command(
            ["ps", "-axo", "pid=,ppid=,pgid=,state=,etime=,command="],
            pathlib.Path.cwd(),
            timeout=2.0,
            stdout_limit=MAX_JSON_ARTIFACT_BYTES,
        )
        if result.returncode != 0 or result.timed_out or result.stdout_truncated:
            return
        processes: list[dict[str, Any]] = []
        for raw_line in result.stdout.decode("utf-8", errors="replace").splitlines():
            fields = raw_line.strip().split(None, 5)
            if len(fields) < 6:
                continue
            try:
                pid, ppid, pgid = (int(fields[index]) for index in range(3))
            except ValueError:
                continue
            if pgid != self.pgid:
                continue
            command = redact_text(fields[5], self.env)[:1024]
            processes.append(
                {
                    "pid": pid,
                    "ppid": ppid,
                    "pgid": pgid,
                    "state": fields[3],
                    "elapsed": fields[4],
                    "command": command,
                }
            )
            self.observed_pids.add(pid)
            if len(self.commands) < 256:
                self.commands.add(command)
        append_jsonl(
            self.destination,
            {
                "schema_version": SCHEMA_VERSION,
                "run_id": self.run_id,
                "sampled_utc": utc_now(),
                "sampled_monotonic_ns": time.monotonic_ns(),
                "processes": processes,
            },
        )
        self.sample_count += 1
        self.max_processes = max(self.max_processes, len(processes))

    def summary(self) -> dict[str, Any]:
        return {
            "schema_version": SCHEMA_VERSION,
            "run_id": self.run_id,
            "samples": self.sample_count,
            "max_processes": self.max_processes,
            "observed_pids": sorted(self.observed_pids),
            "commands": sorted(self.commands),
            "truncated": self.truncated,
            "samples_artifact": "processes.jsonl",
        }


def append_log_baselines(cwd: pathlib.Path, configured: Sequence[str]) -> dict[pathlib.Path, int]:
    selected = list(DEFAULT_APPEND_LOGS)
    selected.extend(configured)
    baselines: dict[pathlib.Path, int] = {}
    for raw in selected:
        path = pathlib.Path(raw).expanduser()
        if not path.is_absolute():
            path = cwd / path
        path = path.resolve(strict=False)
        if path in baselines:
            continue
        try:
            baselines[path] = path.stat().st_size
        except OSError:
            baselines[path] = 0
    return baselines


def filter_append_logs(
    baselines: dict[pathlib.Path, int],
    bundle: pathlib.Path,
    run_ids: set[str],
) -> dict[str, Any]:
    destination = bundle / "filtered-logs"
    destination.mkdir(mode=0o700, exist_ok=True)
    records: list[dict[str, Any]] = []
    for index, (source, offset) in enumerate(sorted(baselines.items(), key=lambda item: str(item[0]))):
        record: dict[str, Any] = {
            "source": str(source),
            "offset": offset,
            "present": source.is_file(),
            "lines_considered": 0,
            "lines_selected": 0,
            "parse_errors": 0,
            "truncated": False,
            "artifact": None,
        }
        if not source.is_file():
            records.append(record)
            continue
        safe_name = re.sub(r"[^A-Za-z0-9._-]+", "-", source.name)[:48] or "log"
        artifact = destination / f"{index:02d}-{safe_name}.jsonl"
        selected: list[dict[str, Any]] = []
        selected_bytes = 0
        bytes_read = 0
        try:
            with source.open("rb") as stream:
                size = source.stat().st_size
                stream.seek(offset if size >= offset else 0)
                while True:
                    line = stream.readline(MAX_JSON_ARTIFACT_BYTES + 1)
                    if not line:
                        break
                    bytes_read += len(line)
                    if bytes_read > MAX_FILTERED_LOG_BYTES or len(line) > MAX_JSON_ARTIFACT_BYTES:
                        record["truncated"] = True
                        break
                    if not line.strip():
                        continue
                    record["lines_considered"] += 1
                    try:
                        value = json.loads(line)
                    except (json.JSONDecodeError, UnicodeDecodeError):
                        record["parse_errors"] += 1
                        continue
                    if collect_named_values(value, "run_id").intersection(run_ids):
                        redacted_value = redact_json(value)
                        encoded_size = len(json.dumps(redacted_value, separators=(",", ":")).encode("utf-8")) + 1
                        if selected_bytes + encoded_size > MAX_FILTERED_LOG_BYTES:
                            record["truncated"] = True
                            break
                        selected.append(redacted_value)
                        selected_bytes += encoded_size
        except OSError as error:
            record["error"] = f"{type(error).__name__}: {error}"
        write_jsonl_records(artifact, selected)
        record["lines_selected"] = len(selected)
        record["artifact"] = str(artifact.relative_to(bundle))
        record["sha256"] = sha256_file(artifact)
        records.append(record)
    index_value = {
        "schema_version": SCHEMA_VERSION,
        "run_ids": sorted(run_ids),
        "sources": records,
    }
    write_json(destination / "index.json", index_value)
    return index_value


class NoRedirectHandler(urllib.request.HTTPRedirectHandler):
    def redirect_request(
        self,
        req: urllib.request.Request,
        fp: Any,
        code: int,
        msg: str,
        headers: Any,
        newurl: str,
    ) -> None:
        return None


def safe_endpoint_base(raw: str, allow_remote: bool) -> str:
    parsed = urllib.parse.urlsplit(raw)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise ValueError("endpoint base must be an http(s) URL with a host")
    if parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise ValueError("endpoint base cannot contain credentials, a query, or a fragment")
    if not allow_remote:
        hostname = parsed.hostname.lower()
        loopback = hostname == "localhost"
        try:
            loopback = loopback or ipaddress.ip_address(hostname).is_loopback
        except ValueError:
            pass
        if not loopback:
            raise ValueError("remote endpoint collection requires --allow-remote-endpoints")
    return urllib.parse.urlunsplit((parsed.scheme, parsed.netloc, parsed.path.rstrip("/"), "", ""))


def endpoint_path_allowed(path: str) -> bool:
    parsed = urllib.parse.urlsplit(path)
    if parsed.scheme or parsed.netloc or not parsed.path.startswith("/"):
        return False
    decoded = urllib.parse.unquote(parsed.path)
    if ".." in pathlib.PurePosixPath(decoded).parts or "\\" in decoded:
        return False
    return True


def substitute_endpoint_path(path: str, run_id: str | None, plan_id: str | None) -> str | None:
    replacements = {"run_id": run_id, "plan_id": plan_id}
    rendered = path
    for name, value in replacements.items():
        if "{" + name + "}" in rendered:
            if not value:
                return None
            rendered = rendered.replace("{" + name + "}", urllib.parse.quote(value, safe=""))
    if re.search(r"\{[^{}]+\}", rendered):
        return None
    return rendered if endpoint_path_allowed(rendered) else None


def endpoint_request(
    opener: urllib.request.OpenerDirector,
    url: str,
    timeout: float,
) -> tuple[int | None, dict[str, str], bytes, bool, str | None, float]:
    started = time.monotonic()
    request = urllib.request.Request(
        url,
        headers={"Accept": "application/json, text/plain;q=0.9, */*;q=0.1", "User-Agent": "roko-evidence/1"},
        method="GET",
    )
    try:
        with opener.open(request, timeout=timeout) as response:
            payload = response.read(MAX_ENDPOINT_BYTES + 1)
            headers = {str(key).lower(): str(value)[:1000] for key, value in response.headers.items()}
            return (
                response.status,
                headers,
                payload[:MAX_ENDPOINT_BYTES],
                len(payload) > MAX_ENDPOINT_BYTES,
                None,
                time.monotonic() - started,
            )
    except urllib.error.HTTPError as error:
        try:
            payload = error.read(MAX_ENDPOINT_BYTES + 1)
        except OSError:
            payload = b""
        headers = {str(key).lower(): str(value)[:1000] for key, value in error.headers.items()}
        return (
            error.code,
            headers,
            payload[:MAX_ENDPOINT_BYTES],
            len(payload) > MAX_ENDPOINT_BYTES,
            f"HTTPError: {error.reason}",
            time.monotonic() - started,
        )
    except (OSError, ValueError) as error:
        return None, {}, b"", False, f"{type(error).__name__}: {error}", time.monotonic() - started


def redacted_response(payload: bytes, content_type: str) -> tuple[bytes, bool, bool]:
    if "json" in content_type.lower():
        try:
            value = json.loads(payload.decode("utf-8"))
            encoded = (json.dumps(redact_json(value), sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
            if len(encoded) > MAX_ENDPOINT_BYTES:
                omitted = json.dumps(
                    {"truncated": True, "reason": "redacted JSON exceeds endpoint capture limit"},
                    sort_keys=True,
                ).encode("utf-8") + b"\n"
                return omitted, True, True
            return encoded, False, True
        except (UnicodeDecodeError, json.JSONDecodeError):
            pass
    try:
        text_value = payload.decode("utf-8")
    except UnicodeDecodeError:
        return b"[binary endpoint response omitted]\n", True, False
    redacted = re.sub(
        r"(?i)((?:api[_-]?key|password|private[_-]?key|secret|token)\s*[=:]\s*)[^\s,;]+",
        r"\1<redacted>",
        text_value,
    )
    redacted = re.sub(r"(?i)(\bBearer\s+)[A-Za-z0-9._~+/=-]+", r"\1<redacted>", redacted)
    encoded = redacted.encode("utf-8")
    return encoded[:MAX_ENDPOINT_BYTES], len(encoded) > MAX_ENDPOINT_BYTES, False


def collect_endpoints(
    bundle: pathlib.Path,
    base_raw: str | None,
    run_id: str | None,
    plan_id: str | None,
    extra_paths: Sequence[str],
    *,
    timeout: float,
    discover_openapi: bool,
    allow_remote: bool,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "enabled": bool(base_raw),
        "method_policy": "GET only; redirects disabled; no credentials",
        "base_url": None,
        "evidence_run_id": bundle.name,
        "run_id": run_id,
        "plan_id": plan_id,
        "openapi": None,
        "results": [],
        "skipped_reason": None,
    }
    if not base_raw:
        result["skipped_reason"] = "no_endpoint_base"
        write_json(bundle / "endpoints.json", result)
        return result
    try:
        base = safe_endpoint_base(base_raw, allow_remote)
    except ValueError as error:
        result["skipped_reason"] = str(error)
        write_json(bundle / "endpoints.json", result)
        return result
    result["base_url"] = base
    opener = urllib.request.build_opener(NoRedirectHandler())
    paths = list(DEFAULT_SAFE_GET_PATHS)
    paths.extend(extra_paths)
    if discover_openapi:
        openapi_started_utc = utc_now()
        status, headers, payload, truncated, error, duration = endpoint_request(
            opener, base + "/api/openapi.json", timeout
        )
        openapi_record: dict[str, Any] = {
            "path": "/api/openapi.json",
            "started_utc": openapi_started_utc,
            "finished_utc": utc_now(),
            "status": status,
            "duration_ms": round(duration * 1000),
            "bytes": len(payload),
            "truncated": truncated,
            "error": error,
        }
        if status == 200 and not truncated:
            try:
                specification = json.loads(payload.decode("utf-8"))
                discovered = []
                for candidate, methods in specification.get("paths", {}).items():
                    if isinstance(candidate, str) and isinstance(methods, dict) and "get" in methods:
                        discovered.append(candidate)
                paths.extend(sorted(discovered))
                openapi_record["get_paths_discovered"] = len(discovered)
            except (AttributeError, UnicodeDecodeError, json.JSONDecodeError) as parse_error:
                openapi_record["parse_error"] = str(parse_error)[:500]
        result["openapi"] = openapi_record

    selected: list[str] = []
    for raw_path in paths:
        rendered = substitute_endpoint_path(raw_path.strip(), run_id, plan_id)
        if rendered and rendered not in selected:
            selected.append(rendered)
        if len(selected) >= MAX_ENDPOINTS:
            break

    artifacts = bundle / "endpoint-responses"
    artifacts.mkdir(mode=0o700, exist_ok=True)
    def query(selected_path: str) -> tuple[str, str, int | None, dict[str, str], bytes, bool, str | None, float, str]:
        request_started_utc = utc_now()
        status, headers, payload, truncated, error, duration = endpoint_request(
            urllib.request.build_opener(NoRedirectHandler()), base + selected_path, timeout
        )
        return (
            selected_path,
            request_started_utc,
            status,
            headers,
            payload,
            truncated,
            error,
            duration,
            utc_now(),
        )

    with ThreadPoolExecutor(max_workers=min(8, max(1, len(selected)))) as executor:
        responses = list(executor.map(query, selected))
    for index, response in enumerate(responses):
        path, request_started_utc, status, headers, payload, truncated, error, duration, request_finished_utc = response
        content_type = headers.get("content-type", "")
        if truncated:
            safe_payload = (
                json.dumps(
                    {"truncated": True, "reason": "endpoint response exceeds capture limit"},
                    sort_keys=True,
                ).encode("utf-8")
                + b"\n"
            )
            redaction_truncated = True
            valid_json = True
        else:
            safe_payload, redaction_truncated, valid_json = redacted_response(payload, content_type)
        suffix = ".json" if valid_json else ".txt"
        artifact = artifacts / f"{index:02d}{suffix}"
        artifact.write_bytes(safe_payload)
        os.chmod(artifact, 0o600)
        result["results"].append(
            {
                "method": "GET",
                "path": path,
                "started_utc": request_started_utc,
                "finished_utc": request_finished_utc,
                "started_after_command": True,
                "duration_ms": round(duration * 1000),
                "status": status,
                "content_type": content_type[:200],
                "bytes_received": len(payload) if not truncated else None,
                "bytes_received_at_least": len(payload) + (1 if truncated else 0),
                "bytes_captured": len(safe_payload),
                "truncated": truncated or redaction_truncated,
                "error": error,
                "artifact": str(artifact.relative_to(bundle)),
                "sha256": sha256_file(artifact),
            }
        )
    write_json(bundle / "endpoints.json", result)
    return result


def parse_named_command(raw: str) -> tuple[str, list[str]]:
    if "=" not in raw:
        raise ValueError("hook must use NAME=COMMAND syntax")
    name, command = raw.split("=", 1)
    safe_name = re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-.")[:48]
    if not safe_name:
        raise ValueError("hook name is empty")
    argv = shlex.split(command)
    if not argv:
        raise ValueError(f"hook {safe_name!r} has an empty command")
    return safe_name, argv


def expand_hook_argv(argv: Sequence[str], bundle: pathlib.Path, run_id: str, output: pathlib.Path | None) -> list[str]:
    replacements = {"{bundle}": str(bundle), "{run_id}": run_id}
    if output is not None:
        replacements["{output}"] = str(output)
    expanded = []
    for argument in argv:
        for marker, value in replacements.items():
            argument = argument.replace(marker, value)
        expanded.append(argument)
    return expanded


def run_cli_smokes(
    bundle: pathlib.Path,
    cwd: pathlib.Path,
    run_id: str,
    hooks: Sequence[str],
    env: dict[str, str],
    timeout: float,
) -> dict[str, Any]:
    directory = bundle / "cli-smoke"
    directory.mkdir(mode=0o700, exist_ok=True)
    results = []
    for index, raw in enumerate(hooks):
        try:
            name, argv = parse_named_command(raw)
            expanded = expand_hook_argv(argv, bundle, run_id, None)
            started_utc = utc_now()
            started = time.monotonic()
            command = run_bounded_command(
                expanded,
                cwd,
                timeout=timeout,
                stdout_limit=MAX_HOOK_OUTPUT_BYTES,
                stderr_limit=MAX_HOOK_OUTPUT_BYTES,
                env=env,
            )
            stdout_path = directory / f"{index:02d}-{name}.stdout.log"
            stderr_path = directory / f"{index:02d}-{name}.stderr.log"
            if stdout_path.is_symlink() or stderr_path.is_symlink():
                raise ValueError("smoke hook created a forbidden artifact symlink")
            stdout_path.write_bytes(command.stdout)
            stderr_path.write_bytes(command.stderr)
            os.chmod(stdout_path, 0o600)
            os.chmod(stderr_path, 0o600)
            results.append(
                {
                    "name": name,
                    "argv": redact_argv(expanded, env),
                    "started_utc": started_utc,
                    "finished_utc": utc_now(),
                    "duration_ms": round((time.monotonic() - started) * 1000),
                    "exit_code": command.returncode,
                    "passed": command.returncode == 0 and not command.timed_out and command.spawn_error is None,
                    "timed_out": command.timed_out,
                    "stdout_truncated": command.stdout_truncated,
                    "stderr_truncated": command.stderr_truncated,
                    "error": command.spawn_error,
                    "stdout": str(stdout_path.relative_to(bundle)),
                    "stderr": str(stderr_path.relative_to(bundle)),
                }
            )
        except (ValueError, OSError) as error:
            results.append({"name": raw.split("=", 1)[0][:48], "passed": False, "error": str(error)[:500]})
    value = {
        "schema_version": SCHEMA_VERSION,
        "enabled": bool(hooks),
        "timeout_seconds": timeout,
        "results": results,
    }
    write_json(bundle / "cli-smoke.json", value)
    return value


def png_dimensions(payload: bytes) -> tuple[int, int] | None:
    if len(payload) < 24 or payload[:8] != b"\x89PNG\r\n\x1a\n" or payload[12:16] != b"IHDR":
        return None
    return int.from_bytes(payload[16:20], "big"), int.from_bytes(payload[20:24], "big")


def screenshot_entry(path: pathlib.Path, bundle: pathlib.Path, run_id: str, name: str, kind: str) -> dict[str, Any]:
    payload, truncated = bounded_read(path, MAX_SCREENSHOT_BYTES)
    entry: dict[str, Any] = {
        "run_id": run_id,
        "name": name,
        "kind": kind,
        "file": str(path.relative_to(bundle)),
        "timestamp_utc": utc_now(),
        "bytes": len(payload),
        "truncated": truncated,
        "sha256": hashlib.sha256(payload).hexdigest(),
    }
    if kind == "png":
        dimensions = png_dimensions(payload)
        entry["valid_png"] = dimensions is not None
        if dimensions:
            entry["width"], entry["height"] = dimensions
    else:
        text_value = payload.decode("utf-8", errors="replace")
        lines = text_value.splitlines()
        entry["rows"] = len(lines)
        entry["columns"] = max((len(line) for line in lines), default=0)
    return entry


def collect_screenshots(
    bundle: pathlib.Path,
    cwd: pathlib.Path,
    run_id: str,
    text_hooks: Sequence[str],
    png_hooks: Sequence[str],
    env: dict[str, str],
    timeout: float,
    roko_screenshot_before: set[str],
    collect_roko: bool,
) -> dict[str, Any]:
    directory = bundle / "screenshots"
    directory.mkdir(mode=0o700, exist_ok=True)
    entries: list[dict[str, Any]] = []
    hook_results: list[dict[str, Any]] = []
    for index, raw in enumerate(text_hooks):
        try:
            name, argv = parse_named_command(raw)
            output = directory / f"text-{index:02d}-{name}.txt"
            command = run_bounded_command(
                expand_hook_argv(argv, bundle, run_id, None),
                cwd,
                timeout=timeout,
                stdout_limit=MAX_HOOK_OUTPUT_BYTES,
                stderr_limit=MAX_METADATA_ERROR_BYTES,
                env=env,
            )
            if output.is_symlink():
                raise ValueError("text hook created a forbidden artifact symlink")
            output.write_bytes(command.stdout)
            os.chmod(output, 0o600)
            entry = screenshot_entry(output, bundle, run_id, name, "text")
            entry["trigger"] = "post_command_hook"
            entry["command_exit_code"] = command.returncode
            entries.append(entry)
            hook_results.append({"name": name, "kind": "text", "passed": command.returncode == 0, "error": command.spawn_error})
        except (ValueError, OSError) as error:
            hook_results.append({"name": raw.split("=", 1)[0][:48], "kind": "text", "passed": False, "error": str(error)[:500]})
    for index, raw in enumerate(png_hooks):
        try:
            name, argv = parse_named_command(raw)
            output = directory / f"browser-{index:02d}-{name}.png"
            command = run_bounded_command(
                expand_hook_argv(argv, bundle, run_id, output),
                cwd,
                timeout=timeout,
                stdout_limit=MAX_METADATA_ERROR_BYTES,
                stderr_limit=MAX_METADATA_ERROR_BYTES,
                env=env,
            )
            if output.is_symlink() or not output.is_file():
                raise ValueError("PNG hook did not create the {output} file")
            payload, truncated = bounded_read(output, MAX_SCREENSHOT_BYTES)
            if truncated:
                raise ValueError("PNG hook output exceeds 8 MiB")
            os.chmod(output, 0o600)
            entry = screenshot_entry(output, bundle, run_id, name, "png")
            entry["trigger"] = "post_command_browser_hook"
            entry["command_exit_code"] = command.returncode
            entries.append(entry)
            hook_results.append(
                {
                    "name": name,
                    "kind": "png",
                    "passed": command.returncode == 0 and bool(entry.get("valid_png")),
                    "error": command.spawn_error,
                }
            )
        except (ValueError, OSError) as error:
            hook_results.append({"name": raw.split("=", 1)[0][:48], "kind": "png", "passed": False, "error": str(error)[:500]})

    roko_result: dict[str, Any] = {"enabled": collect_roko, "imported": 0, "skipped_reason": None}
    if collect_roko:
        source_root = cwd / ".roko" / "screenshots"
        current_dirs = {str(path.resolve(strict=False)) for path in source_root.glob("run-*") if path.is_dir()}
        new_dirs = sorted(current_dirs - roko_screenshot_before)
        if len(new_dirs) != 1:
            roko_result["skipped_reason"] = "expected_exactly_one_new_roko_screenshot_directory"
            roko_result["candidates"] = new_dirs[:10]
        else:
            source_dir = pathlib.Path(new_dirs[0])
            import_dir = directory / "roko"
            import_dir.mkdir(mode=0o700, exist_ok=True)
            for source in sorted(source_dir.iterdir())[:64]:
                if source.is_symlink() or not source.is_file() or source.suffix.lower() not in {".txt", ".png", ".json"}:
                    continue
                payload, truncated = bounded_read(source, MAX_SCREENSHOT_BYTES)
                if truncated:
                    continue
                target = import_dir / source.name
                target.write_bytes(payload)
                os.chmod(target, 0o600)
                if source.suffix.lower() in {".txt", ".png"}:
                    kind = "png" if source.suffix.lower() == ".png" else "text"
                    entry = screenshot_entry(target, bundle, run_id, source.stem, kind)
                    entry["trigger"] = "roko_event_collector"
                    entries.append(entry)
                roko_result["imported"] += 1

    value = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "enabled": bool(text_hooks or png_hooks or collect_roko),
        "entries": entries,
        "hooks": hook_results,
        "roko_collector": roko_result,
    }
    write_json(directory / "manifest.json", value)
    return value


def read_jsonl(path: pathlib.Path, limit: int = MAX_FILTERED_LOG_BYTES) -> tuple[list[Any], list[str]]:
    values: list[Any] = []
    errors: list[str] = []
    if not path.is_file():
        return values, errors
    consumed = 0
    try:
        with path.open("rb") as stream:
            for line_number, line in enumerate(stream, start=1):
                consumed += len(line)
                if consumed > limit:
                    errors.append(f"capture exceeds {limit} bytes")
                    break
                if not line.strip():
                    continue
                try:
                    values.append(json.loads(line))
                except (json.JSONDecodeError, UnicodeDecodeError) as error:
                    errors.append(f"line {line_number}: {error}")
    except OSError as error:
        errors.append(str(error))
    return values, errors


def event_metrics(events_path: pathlib.Path) -> dict[str, Any]:
    events, parse_errors = read_jsonl(events_path)
    types: Counter[str] = Counter()
    runner_run_ids: set[str] = set()
    plan_ids: set[str] = set()
    models: Counter[str] = Counter()
    providers: Counter[str] = Counter()
    dispatches: defaultdict[str, int] = defaultdict(int)
    gate_passed = 0
    gate_failed = 0
    retries = 0
    timeouts = 0
    first_timestamp_ms: int | float | None = None
    last_timestamp_ms: int | float | None = None
    first_failure: dict[str, Any] | None = None
    terminal_cost_usd: float | None = None
    terminal_agent_calls: int | None = None
    prompt_estimated_tokens = 0
    phase_totals: defaultdict[str, int] = defaultdict(int)
    for event in events:
        event_type = normalized_event_type(event) or "missing"
        types[event_type] += 1
        runner_run_ids.update(collect_named_values(event, "run_id"))
        plan_ids.update(collect_named_values(event, "plan_id"))
        if not isinstance(event, dict):
            continue
        timestamp_ms = event.get("timestamp_ms") or event.get("monotonic_ms")
        if isinstance(timestamp_ms, (int, float)):
            first_timestamp_ms = timestamp_ms if first_timestamp_ms is None else min(first_timestamp_ms, timestamp_ms)
            last_timestamp_ms = timestamp_ms if last_timestamp_ms is None else max(last_timestamp_ms, timestamp_ms)
        for key, counter in (("model", models), ("provider", providers), ("requested_model", models)):
            value = event.get(key)
            if isinstance(value, str) and value:
                counter[value] += 1
        if event_type == "agent.dispatch.started":
            identity = "/".join(
                str(event.get(key, "")) for key in ("plan_id", "task_id", "attempt")
            )
            dispatches[identity] += 1
        if event_type == "gate.completed":
            if event.get("passed") is True:
                gate_passed += 1
            else:
                gate_failed += 1
                if first_failure is None:
                    first_failure = {"type": event_type, "timestamp": event.get("timestamp"), "failure_kind": event.get("failure_kind")}
        if event_type == "retry.decision":
            retries += 1
        if event_type == "timeout.recorded":
            timeouts += 1
            if first_failure is None:
                first_failure = {"type": event_type, "timestamp": event.get("timestamp"), "failure_kind": "timeout"}
        if event_type == "prompt.assembled" and isinstance(event.get("estimated_tokens"), (int, float)):
            prompt_estimated_tokens += int(event["estimated_tokens"])
        if event_type == "task.attempt.completed":
            phase_durations = event.get("phase_durations")
            if isinstance(phase_durations, dict):
                for name, duration in phase_durations.items():
                    if isinstance(duration, (int, float)):
                        phase_totals[str(name)] += int(duration)
            outcome = str(event.get("outcome", "")).lower()
            if first_failure is None and outcome not in {"passed", "succeeded", "success", "completed"}:
                first_failure = {"type": event_type, "timestamp": event.get("timestamp"), "failure_kind": event.get("failure_kind") or outcome}
        if event_type in RUN_TERMINAL_NAMES:
            if isinstance(event.get("total_cost_usd"), (int, float)):
                terminal_cost_usd = float(event["total_cost_usd"])
            if isinstance(event.get("total_agent_calls"), int):
                terminal_agent_calls = event["total_agent_calls"]
    return {
        "present": events_path.is_file(),
        "parse_errors": parse_errors,
        "event_count": len(events),
        "event_types": dict(sorted(types.items())),
        "runner_run_ids": sorted(runner_run_ids),
        "plan_ids": sorted(plan_ids),
        "first_timestamp_ms": first_timestamp_ms,
        "last_timestamp_ms": last_timestamp_ms,
        "event_span_ms": (last_timestamp_ms - first_timestamp_ms) if first_timestamp_ms is not None and last_timestamp_ms is not None else None,
        "models": dict(sorted(models.items())),
        "providers": dict(sorted(providers.items())),
        "actual_launches": sum(dispatches.values()),
        "dispatches_per_attempt": dict(sorted(dispatches.items())),
        "duplicate_dispatch_attempts": sorted(key for key, count in dispatches.items() if count != 1),
        "gate_passed": gate_passed,
        "gate_failed": gate_failed,
        "retries": retries,
        "timeouts": timeouts,
        "prompt_estimated_tokens": prompt_estimated_tokens,
        "total_cost_usd": terminal_cost_usd,
        "terminal_agent_calls": terminal_agent_calls,
        "phase_duration_ms": dict(sorted(phase_totals.items())),
        "first_failure": first_failure,
    }


def status_metrics(path: pathlib.Path) -> dict[str, Any]:
    samples, errors = read_jsonl(path)
    phases: Counter[str] = Counter()
    max_active_agents = 0
    source_run_ids: set[str] = set()
    for sample in samples:
        if not isinstance(sample, dict):
            continue
        source_run_id = sample.get("source_run_id")
        if isinstance(source_run_id, str):
            source_run_ids.add(source_run_id)
        status = sample.get("status")
        if not isinstance(status, dict):
            continue
        phase = status.get("phase")
        if isinstance(phase, str):
            phases[phase] += 1
        active = status.get("active_agents")
        if isinstance(active, int):
            max_active_agents = max(max_active_agents, active)
    return {
        "samples": len(samples),
        "parse_errors": errors,
        "source_run_ids": sorted(source_run_ids),
        "phase_sample_counts": dict(sorted(phases.items())),
        "max_active_agents": max_active_agents,
    }


def git_change_metrics(cwd: pathlib.Path) -> dict[str, Any]:
    result = run_git(cwd, "diff", "--numstat", "HEAD", "--", ".", timeout=3.0)
    files = 0
    additions = 0
    deletions = 0
    binary_files = 0
    for line in result.stdout.decode("utf-8", errors="replace").splitlines():
        fields = line.split("\t", 2)
        if len(fields) != 3:
            continue
        files += 1
        if fields[0].isdigit() and fields[1].isdigit():
            additions += int(fields[0])
            deletions += int(fields[1])
        else:
            binary_files += 1
    return {
        "files": files,
        "additions": additions,
        "deletions": deletions,
        "loc": additions + deletions,
        "binary_files": binary_files,
        "capture": bounded_capture_metadata(result),
    }


def build_metrics(
    bundle: pathlib.Path,
    cwd: pathlib.Path,
    run_id: str,
    timings: dict[str, Any],
    endpoint_results: dict[str, Any],
    smoke_results: dict[str, Any],
    screenshots: dict[str, Any],
    processes: dict[str, Any],
    cache: dict[str, Any],
    admission: dict[str, Any],
) -> dict[str, Any]:
    events = event_metrics(bundle / "events.jsonl")
    statuses = status_metrics(bundle / "status-samples.jsonl")
    endpoint_rows = endpoint_results.get("results", [])
    smoke_rows = smoke_results.get("results", [])
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "latency_ms": {
            "command": timings.get("command_duration_ms"),
            "bundle_finalize": timings.get("bundle_finalize_duration_ms"),
            "total": timings.get("total_duration_ms"),
            "event_span": events.get("event_span_ms"),
            "runner_phases": events.get("phase_duration_ms", {}),
        },
        "events": events,
        "status": statuses,
        "git": git_change_metrics(cwd),
        "provider": {
            "models": events.get("models", {}),
            "providers": events.get("providers", {}),
            "actual_launches": events.get("actual_launches", 0),
            "prompt_estimated_tokens": events.get("prompt_estimated_tokens", 0),
            "cost_usd": events.get("total_cost_usd"),
            "retries": events.get("retries", 0),
            "timeouts": events.get("timeouts", 0),
        },
        "verification": {
            "gates_passed": events.get("gate_passed", 0),
            "gates_failed": events.get("gate_failed", 0),
            "cli_smokes_passed": sum(1 for row in smoke_rows if row.get("passed") is True),
            "cli_smokes_failed": sum(1 for row in smoke_rows if row.get("passed") is False),
            "endpoints_2xx": sum(1 for row in endpoint_rows if isinstance(row.get("status"), int) and 200 <= row["status"] < 300),
            "endpoints_failed": sum(1 for row in endpoint_rows if not isinstance(row.get("status"), int) or not 200 <= row["status"] < 300),
            "screenshots": len(screenshots.get("entries", [])),
        },
        "processes": processes,
        "cache": cache,
        "resource_admission": admission,
        "first_failure": events.get("first_failure"),
    }


def build_score(metrics: dict[str, Any], terminal_state: str, evidence_valid: bool | None) -> dict[str, Any]:
    latency = metrics.get("latency_ms", {})
    events = metrics.get("events", {})
    verification = metrics.get("verification", {})
    total_ms = latency.get("total")
    command_ms = latency.get("command")
    finalize_ms = latency.get("bundle_finalize")
    overhead_limit = max(2000, round((command_ms or 0) * 0.05))
    targets: dict[str, bool | None] = {
        "terminal_succeeded": terminal_state == "succeeded",
        "evidence_valid": evidence_valid,
        "exactly_one_dispatch_per_attempt": not events.get("duplicate_dispatch_attempts") if events.get("actual_launches", 0) else None,
        "total_at_or_below_300s": total_ms <= 300_000 if isinstance(total_ms, int) else None,
        "bundle_overhead_within_budget": finalize_ms <= overhead_limit if isinstance(finalize_ms, int) else None,
        "no_gate_failures": verification.get("gates_failed", 0) == 0,
        "all_cli_smokes_pass": verification.get("cli_smokes_failed", 0) == 0 if verification.get("cli_smokes_passed", 0) + verification.get("cli_smokes_failed", 0) else None,
        "all_endpoints_2xx": verification.get("endpoints_failed", 0) == 0 if verification.get("endpoints_2xx", 0) + verification.get("endpoints_failed", 0) else None,
    }
    applicable = [value for value in targets.values() if value is not None]
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": metrics.get("run_id"),
        "targets": targets,
        "passed": sum(value is True for value in applicable),
        "failed": sum(value is False for value in applicable),
        "not_applicable": sum(value is None for value in targets.values()),
        "ratio": round(sum(value is True for value in applicable) / len(applicable), 4) if applicable else None,
        "raw": {
            "total_ms": total_ms,
            "command_ms": command_ms,
            "bundle_finalize_ms": finalize_ms,
            "overhead_limit_ms": overhead_limit,
        },
    }


def markdown_value(value: Any) -> str:
    if value is None:
        return "not observed"
    return str(value).replace("`", "'").replace("\n", " ")[:500]


def build_debrief(
    manifest: dict[str, Any],
    summary: dict[str, Any],
    metrics: dict[str, Any],
    validation: dict[str, Any] | None,
) -> str:
    git = metrics.get("git", {})
    verification = metrics.get("verification", {})
    provider = metrics.get("provider", {})
    first_failure = metrics.get("first_failure")
    validation_state = validation.get("valid") if validation else "pending"
    lines = [
        f"# Evidence debrief — `{markdown_value(manifest.get('run_id'))}`",
        "",
        "This file is generated deterministically from bundle facts; it contains no LLM diagnosis.",
        "",
        "## 1. Outcome",
        "",
        f"- State: `{markdown_value(summary.get('state'))}`",
        f"- Process exit: `{markdown_value(summary.get('process_exit_code'))}`; wrapper exit: `{markdown_value(summary.get('exit_code'))}`",
        f"- Timed out: `{markdown_value(summary.get('timed_out'))}`; evidence valid: `{markdown_value(validation_state)}`",
        f"- Admission/artifact limit: `{markdown_value(summary.get('admission_error') or summary.get('artifact_limit_exceeded'))}`",
        "",
        "## 2. Phase timeline",
        "",
        f"- Command: `{markdown_value(metrics.get('latency_ms', {}).get('command'))} ms`",
        f"- Bundle finalization: `{markdown_value(metrics.get('latency_ms', {}).get('bundle_finalize'))} ms`",
        f"- Runner phase totals: `{markdown_value(metrics.get('latency_ms', {}).get('runner_phases'))}`",
        "",
        "## 3. First failure",
        "",
        f"- `{markdown_value(first_failure)}`",
        "",
        "## 4. Changed files / LOC",
        "",
        f"- Files: `{markdown_value(git.get('files'))}`; additions: `{markdown_value(git.get('additions'))}`; deletions: `{markdown_value(git.get('deletions'))}`",
        "",
        "## 5. Verification selected and why",
        "",
        f"- Runner gates passed/failed: `{markdown_value(verification.get('gates_passed'))}` / `{markdown_value(verification.get('gates_failed'))}`",
        f"- Explicit CLI smokes passed/failed: `{markdown_value(verification.get('cli_smokes_passed'))}` / `{markdown_value(verification.get('cli_smokes_failed'))}`",
        "- Selection rationale is operator-authored in the plan or explicit hook arguments; the collector does not invent verification.",
        "",
        "## 6. Endpoint / screenshot results",
        "",
        f"- Endpoint 2xx/failed: `{markdown_value(verification.get('endpoints_2xx'))}` / `{markdown_value(verification.get('endpoints_failed'))}`",
        f"- Screenshot artifacts: `{markdown_value(verification.get('screenshots'))}`",
        "",
        "## 7. Provider usage / cost / cache",
        "",
        f"- Models: `{markdown_value(provider.get('models'))}`; providers: `{markdown_value(provider.get('providers'))}`",
        f"- Launches: `{markdown_value(provider.get('actual_launches'))}`; estimated prompt tokens: `{markdown_value(provider.get('prompt_estimated_tokens'))}`; cost: `{markdown_value(provider.get('cost_usd'))}`",
        "",
        "## 8. Resource / cache state",
        "",
        f"- Maximum observed process-group size: `{markdown_value(metrics.get('processes', {}).get('max_processes'))}`",
        f"- Cargo target: `{markdown_value(metrics.get('cache', {}).get('target', {}).get('path'))}`",
        f"- Disk free: `{markdown_value(metrics.get('resource_admission', {}).get('disk', {}).get('free_bytes'))}` bytes (`{markdown_value(metrics.get('resource_admission', {}).get('disk', {}).get('free_percent'))}%`); admitted: `{markdown_value(metrics.get('resource_admission', {}).get('admitted'))}`",
        "",
        "## 9. Root-cause hypotheses",
        "",
        "- None generated automatically. Any hypothesis must be added by a reviewer and labeled as such.",
        "",
        "## 10. Recommended next action",
        "",
        "- Review the first failing event or smoke, then the bounded stdout/stderr and exact Git diff. Do not infer success when validation is false.",
        "",
    ]
    return "\n".join(lines)


def bundle_files(bundle: pathlib.Path) -> list[pathlib.Path]:
    return sorted(path for path in bundle.rglob("*") if path.is_file() or path.is_symlink())


def validate_bundle(
    bundle: pathlib.Path,
    *,
    require_events: bool = False,
    require_status_samples: bool = False,
    require_smoke_pass: bool = False,
    require_endpoints_pass: bool = False,
    require_screenshots: bool = False,
) -> dict[str, Any]:
    errors: list[str] = []
    warnings: list[str] = []
    required = (
        "manifest.json",
        "command.txt",
        "stdout.log",
        "stderr.log",
        "status.jsonl",
        "status-samples.jsonl",
        "commands.jsonl",
        "usage.jsonl",
        "timings.json",
        "summary.json",
        "metrics.json",
        "score.json",
        "processes.json",
        "processes.jsonl",
        "resource-admission.json",
        "artifact-limits.json",
        "events-validation.json",
        "gates.json",
        "diff-stat.json",
        "git-before.json",
        "git-after.json",
        "machine.json",
        "cache.json",
        "endpoints.json",
        "cli-smoke.json",
        "screenshots/manifest.json",
        "filtered-logs/index.json",
        "diff.patch",
        "DEBRIEF.md",
    )
    if bundle.is_symlink() or not bundle.is_dir():
        return {"schema_version": SCHEMA_VERSION, "valid": False, "errors": ["bundle must be a real directory"], "warnings": []}
    for name in required:
        if not (bundle / name).is_file():
            errors.append(f"missing required artifact: {name}")
    files = bundle_files(bundle)
    total_bytes = 0
    parsed_json: dict[str, Any] = {}
    parsed_jsonl: dict[str, list[Any]] = {}
    secret_hits: list[dict[str, str]] = []
    for path in files:
        relative = str(path.relative_to(bundle))
        if path.is_symlink():
            errors.append(f"symlink artifact is forbidden: {relative}")
            continue
        try:
            size = path.stat().st_size
            mode = path.stat().st_mode & 0o777
        except OSError as error:
            errors.append(f"cannot stat {relative}: {error}")
            continue
        total_bytes += size
        if mode & 0o077:
            errors.append(f"artifact is not private (mode {mode:04o}): {relative}")
        limit = MAX_JSON_ARTIFACT_BYTES if path.suffix in {".json", ".jsonl"} else MAX_SCREENSHOT_BYTES
        if relative in {"stdout.log", "stderr.log"}:
            limit = MAX_OUTPUT_BYTES
        elif relative.startswith("filtered-logs/") and path.suffix == ".jsonl":
            limit = MAX_FILTERED_LOG_BYTES
        elif relative.startswith("diff") and path.suffix in {".patch", ".txt"}:
            limit = MAX_DIFF_BYTES
        elif relative.startswith("endpoint-responses/"):
            limit = MAX_ENDPOINT_BYTES
        if size > limit:
            errors.append(f"artifact exceeds {limit} byte limit: {relative} ({size})")
            continue
        payload, truncated = bounded_read(path, limit)
        if truncated:
            errors.append(f"artifact read exceeded declared limit: {relative}")
            continue
        if path.suffix == ".json":
            try:
                parsed_json[relative] = json.loads(payload.decode("utf-8"))
            except (json.JSONDecodeError, UnicodeDecodeError) as error:
                errors.append(f"malformed JSON {relative}: {error}")
        elif path.suffix == ".jsonl":
            rows: list[Any] = []
            for line_number, line in enumerate(payload.splitlines(), start=1):
                if not line.strip():
                    continue
                try:
                    rows.append(json.loads(line))
                except (json.JSONDecodeError, UnicodeDecodeError) as error:
                    errors.append(f"malformed JSONL {relative}:{line_number}: {error}")
            parsed_jsonl[relative] = rows
        if path.suffix.lower() != ".png":
            for kind, pattern in SECRET_VALUE_PATTERNS:
                match = pattern.search(payload)
                if match:
                    secret_hits.append({"artifact": relative, "kind": kind})
                    break
    if total_bytes > MAX_BUNDLE_BYTES:
        errors.append(f"bundle exceeds {MAX_BUNDLE_BYTES} bytes ({total_bytes})")
    if secret_hits:
        errors.extend(f"possible {hit['kind']} secret in {hit['artifact']}" for hit in secret_hits)

    manifest = parsed_json.get("manifest.json", {})
    summary = parsed_json.get("summary.json", {})
    versioned_json = (
        "manifest.json",
        "summary.json",
        "timings.json",
        "metrics.json",
        "score.json",
        "processes.json",
        "resource-admission.json",
        "artifact-limits.json",
        "events-validation.json",
        "endpoints.json",
        "cli-smoke.json",
        "screenshots/manifest.json",
        "gates.json",
        "filtered-logs/index.json",
    )
    for relative in versioned_json:
        value = parsed_json.get(relative)
        if isinstance(value, dict) and value.get("schema_version") != SCHEMA_VERSION:
            errors.append(f"unsupported or inconsistent schema_version in {relative}")
    for relative in ("status.jsonl", "status-samples.jsonl", "commands.jsonl", "processes.jsonl", "usage.jsonl"):
        for row in parsed_jsonl.get(relative, []):
            if not isinstance(row, dict) or row.get("schema_version") != SCHEMA_VERSION:
                errors.append(f"unsupported or inconsistent schema_version in {relative}")
                break
    run_id = manifest.get("run_id") if isinstance(manifest, dict) else None
    recorded_requirements = manifest.get("requirements", {}) if isinstance(manifest, dict) else {}
    if isinstance(recorded_requirements, dict):
        require_events = require_events or recorded_requirements.get("events") is True
        require_status_samples = (
            require_status_samples or recorded_requirements.get("status_sample") is True
        )
        require_smoke_pass = (
            require_smoke_pass or recorded_requirements.get("cli_smoke_pass") is True
        )
        require_endpoints_pass = (
            require_endpoints_pass or recorded_requirements.get("endpoints_pass") is True
        )
        require_screenshots = (
            require_screenshots or recorded_requirements.get("screenshots") is True
        )
    if not isinstance(run_id, str) or not run_id:
        errors.append("manifest run_id is missing")
    status_rows = parsed_jsonl.get("status.jsonl", [])
    terminal_rows = [row for row in status_rows if isinstance(row, dict) and row.get("terminal") is True]
    if len(terminal_rows) != 1:
        errors.append(f"status.jsonl must contain exactly one terminal, found {len(terminal_rows)}")
    if run_id and any(isinstance(row, dict) and row.get("run_id") != run_id for row in status_rows):
        errors.append("status.jsonl contains another run_id")
    status_samples = parsed_jsonl.get("status-samples.jsonl", [])
    if require_status_samples and not status_samples:
        errors.append("at least one run-scoped status sample is required")
    if run_id and any(isinstance(row, dict) and row.get("run_id") != run_id for row in status_samples):
        errors.append("status-samples.jsonl contains another evidence run_id")
    if isinstance(summary, dict):
        if summary.get("terminal") is not True:
            errors.append("summary does not claim a terminal result")
        if terminal_rows and summary.get("exit_code") != terminal_rows[0].get("exit_code"):
            errors.append("summary exit code disagrees with terminal status")
        if summary.get("timed_out") is True and summary.get("state") == "succeeded":
            errors.append("timed-out command is reported successful")
        if summary.get("process_exit_code") not in {0, None} and summary.get("state") == "succeeded":
            errors.append("nonzero process is reported successful")
    admission = parsed_json.get("resource-admission.json", {})
    if isinstance(admission, dict) and admission.get("enabled") is True:
        if admission.get("admitted") is not True and isinstance(summary, dict) and summary.get("state") == "succeeded":
            errors.append("resource admission failed but the command is reported successful")
        if admission.get("pressure_detected") is True and admission.get("admitted") is True and admission.get("override") is not True:
            errors.append("disk-pressure admission was bypassed without an explicit override")

    events_path = bundle / "events.jsonl"
    events_validation = validate_events_jsonl(events_path)
    if require_events and not events_validation["present"]:
        errors.append("events.jsonl is required but absent")
    if events_validation["present"]:
        if events_validation["valid_jsonl"] is not True:
            errors.append("events.jsonl is malformed")
        if events_validation["run_start_count"] != 1:
            errors.append(f"events.jsonl must contain exactly one run start, found {events_validation['run_start_count']}")
        if events_validation["run_terminal_count"] != 1:
            errors.append(f"events.jsonl must contain exactly one run terminal, found {events_validation['run_terminal_count']}")
        if len(events_validation["run_ids"]) != 1:
            errors.append(f"events.jsonl must contain exactly one runner run_id, found {len(events_validation['run_ids'])}")
        if events_validation["run_id_field_missing"]:
            errors.append(
                f"events.jsonl has {events_validation['run_id_field_missing']} event(s) without run_id"
            )
        if events_validation["timestamp_order_valid"] is False:
            errors.append("events.jsonl timestamps are out of order")
        errors.extend(
            f"events.jsonl lifecycle imbalance: {detail}"
            for detail in events_validation.get("lifecycle_errors", [])
        )

    command_rows = parsed_jsonl.get("commands.jsonl", [])
    primary_commands = [
        row for row in command_rows if isinstance(row, dict) and row.get("command_id") == "command-1"
    ]
    if len(primary_commands) != 1:
        errors.append(f"commands.jsonl must contain exactly one primary command, found {len(primary_commands)}")

    endpoints = parsed_json.get("endpoints.json", {})
    if isinstance(endpoints, dict):
        if endpoints.get("evidence_run_id") != run_id:
            errors.append("endpoint evidence contains another evidence run_id")
        for row in endpoints.get("results", []):
            if not isinstance(row, dict) or row.get("method") != "GET":
                errors.append("endpoint evidence contains a non-GET request")
                break
        if require_endpoints_pass:
            rows = endpoints.get("results", [])
            if not rows:
                errors.append("endpoint evidence is required but no requests were collected")
            elif any(
                not isinstance(row, dict)
                or not isinstance(row.get("status"), int)
                or not 200 <= row["status"] < 300
                for row in rows
            ):
                errors.append("one or more required endpoint GET probes failed")
    smokes = parsed_json.get("cli-smoke.json", {})
    if require_smoke_pass:
        smoke_rows = smokes.get("results", []) if isinstance(smokes, dict) else []
        if not smoke_rows:
            errors.append("CLI smoke evidence is required but no hooks were run")
        elif any(not isinstance(row, dict) or row.get("passed") is not True for row in smoke_rows):
            errors.append("one or more required CLI smoke hooks failed")
    screenshots = parsed_json.get("screenshots/manifest.json", {})
    if isinstance(screenshots, dict):
        for entry in screenshots.get("entries", []):
            if isinstance(entry, dict) and entry.get("run_id") != run_id:
                errors.append("screenshot manifest contains another run_id")
            if isinstance(entry, dict) and entry.get("kind") == "png" and entry.get("valid_png") is not True:
                errors.append(f"invalid PNG screenshot: {entry.get('file')}")
        if require_screenshots and not screenshots.get("entries"):
            errors.append("screenshot evidence is required but no screenshots were collected")
        if require_screenshots and any(
            not isinstance(hook, dict) or hook.get("passed") is not True
            for hook in screenshots.get("hooks", [])
        ):
            errors.append("one or more required screenshot hooks failed")
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "valid": not errors,
        "errors": errors,
        "warnings": warnings,
        "files": len(files),
        "total_bytes": total_bytes,
        "secret_hits": secret_hits,
        "events": events_validation,
        "validated_utc": utc_now(),
    }


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a command with a deadline and write a private evidence bundle.",
        epilog=(
            "The command deadline excludes a few seconds of final Git/artifact capture. "
            "GET probes and screenshot/smoke hooks are opt-in; no build, mutation, cleanup, "
            "credential forwarding, or full environment dump is performed by the collector."
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
    parser.add_argument(
        "--status-file",
        default=".roko/state/status.json",
        metavar="PATH",
        help="lightweight status JSON to sample while the command runs",
    )
    parser.add_argument(
        "--status-interval",
        type=float,
        default=0.5,
        metavar="SECONDS",
        help="status polling interval (default: 0.5)",
    )
    parser.add_argument(
        "--append-log",
        action="append",
        default=[],
        metavar="PATH",
        help="additional append-only JSONL to slice by observed run_id (repeatable)",
    )
    parser.add_argument(
        "--endpoint-base",
        metavar="URL",
        help="opt in to bounded safe GET collection from this base URL",
    )
    parser.add_argument(
        "--endpoint",
        action="append",
        default=[],
        metavar="PATH",
        help="additional safe GET path (repeatable; unresolved placeholders are skipped)",
    )
    parser.add_argument(
        "--endpoint-run-id",
        metavar="RUN_ID",
        help="override the runner run ID substituted in endpoint paths",
    )
    parser.add_argument("--plan-id", metavar="PLAN_ID", help="plan ID substituted in endpoint paths")
    parser.add_argument(
        "--endpoint-timeout",
        type=float,
        default=2.0,
        metavar="SECONDS",
        help="per-GET deadline (default: 2)",
    )
    parser.add_argument("--no-openapi", action="store_true", help="disable same-origin OpenAPI GET discovery")
    parser.add_argument(
        "--allow-remote-endpoints",
        action="store_true",
        help="allow the explicit endpoint base to be non-loopback (no credentials are forwarded)",
    )
    parser.add_argument(
        "--cli-smoke",
        action="append",
        default=[],
        metavar="NAME=COMMAND",
        help="run an explicit shell-free post-command CLI smoke (repeatable)",
    )
    parser.add_argument(
        "--text-snapshot",
        action="append",
        default=[],
        metavar="NAME=COMMAND",
        help="capture stdout from an explicit shell-free command as a text screenshot",
    )
    parser.add_argument(
        "--png-hook",
        "--browser-hook",
        dest="png_hook",
        action="append",
        default=[],
        metavar="NAME=COMMAND",
        help="run an optional browser hook that writes a PNG to the {output} placeholder",
    )
    parser.add_argument(
        "--hook-timeout",
        type=float,
        default=10.0,
        metavar="SECONDS",
        help="per smoke/screenshot hook deadline (default: 10)",
    )
    parser.add_argument(
        "--collect-roko-screenshots",
        action="store_true",
        help="import exactly one screenshot directory newly created by the wrapped command",
    )
    parser.add_argument(
        "--admit-resources",
        action="store_true",
        help="check disk pressure and record disk/swap/target sizing before launch",
    )
    parser.add_argument(
        "--allow-low-disk",
        action="store_true",
        help="explicitly override a failed disk-pressure admission check",
    )
    parser.add_argument(
        "--min-free-gib",
        type=float,
        default=DEFAULT_MIN_FREE_BYTES / (1024**3),
        metavar="GIB",
        help="resource admission free-space floor (default: 5 GiB)",
    )
    parser.add_argument(
        "--min-free-percent",
        type=float,
        default=DEFAULT_MIN_FREE_PERCENT,
        metavar="PERCENT",
        help="resource admission free-space percentage floor (default: 3)",
    )
    parser.add_argument("--require-events", action="store_true", help="make a valid one-start/one-terminal events.jsonl mandatory")
    parser.add_argument("--require-status-sample", action="store_true", help="make at least one fresh status sample mandatory")
    parser.add_argument("--require-cli-smoke-pass", action="store_true", help="make all configured CLI smokes mandatory and passing")
    parser.add_argument("--require-endpoints-pass", action="store_true", help="make all collected GET probes mandatory and 2xx")
    parser.add_argument("--require-screenshots", action="store_true", help="make at least one valid text/PNG screenshot mandatory")
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
    if args.status_interval <= 0:
        parser.error("--status-interval must be greater than zero")
    if args.endpoint_timeout <= 0 or args.hook_timeout <= 0:
        parser.error("endpoint and hook timeouts must be greater than zero")
    if args.min_free_gib < 0 or not 0 <= args.min_free_percent <= 100:
        parser.error("resource admission thresholds must be non-negative and percent at most 100")
    if args.require_endpoints_pass and not args.endpoint_base:
        parser.error("--require-endpoints-pass requires --endpoint-base")
    if args.require_cli_smoke_pass and not args.cli_smoke:
        parser.error("--require-cli-smoke-pass requires at least one --cli-smoke")
    if args.require_screenshots and not (
        args.text_snapshot or args.png_hook or args.collect_roko_screenshots
    ):
        parser.error("--require-screenshots requires a screenshot collector or hook")
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
    run_id_base = f"{stamp}-{label}-{uuid.uuid4().hex[:8]}"
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

    status_source = pathlib.Path(args.status_file).expanduser()
    if not status_source.is_absolute():
        status_source = cwd / status_source
    status_source = status_source.resolve(strict=False)
    log_baselines = append_log_baselines(cwd, args.append_log)
    roko_screenshot_root = cwd / ".roko" / "screenshots"
    roko_screenshot_before = {
        str(path.resolve(strict=False))
        for path in roko_screenshot_root.glob("run-*")
        if path.is_dir()
    }

    git_before = git_snapshot(cwd)
    machine = machine_snapshot(cwd)
    cache = cache_snapshot(cwd, env)
    admission = resource_admission(
        cwd,
        env,
        machine,
        cache,
        enabled=args.admit_resources,
        allow_low_disk=args.allow_low_disk,
        min_free_bytes=round(args.min_free_gib * 1024**3),
        min_free_percent=args.min_free_percent,
    )
    admission_error = (
        "; ".join(admission["pressure_reasons"])
        if args.admit_resources and not admission["admitted"]
        else None
    )
    started_utc = utc_now()
    overall_started = time.monotonic()
    overall_started_ns = time.monotonic_ns()

    write_json(bundle / "git-before.json", git_before)
    write_json(bundle / "machine.json", machine)
    write_json(bundle / "cache.json", cache)
    write_json(bundle / "resource-admission.json", admission)
    write_text(bundle / "command.txt", " ".join(shlex.quote(arg) for arg in redacted_argv) + "\n")
    capture_git_diff(cwd, bundle / "diff-before.patch")
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "started_utc": started_utc,
        "started_monotonic_ns": overall_started_ns,
        "cwd": str(cwd),
        "bundle": str(bundle),
        "deadline_seconds": args.deadline,
        "command": redacted_argv,
        "command_redacted": redacted_argv != execution_argv,
        "environment": environment_snapshot(env),
        "git_before": git_before,
        "machine": machine,
        "cache": cache,
        "collection": {
            "status_file": str(status_source),
            "status_interval_seconds": args.status_interval,
            "append_logs": [str(path) for path in sorted(log_baselines, key=str)],
            "endpoint_base": args.endpoint_base,
            "endpoint_get_only": True,
            "cli_smoke_hooks": [raw.split("=", 1)[0] for raw in args.cli_smoke],
            "text_snapshot_hooks": [raw.split("=", 1)[0] for raw in args.text_snapshot],
            "png_hooks": [raw.split("=", 1)[0] for raw in args.png_hook],
            "collect_roko_screenshots": args.collect_roko_screenshots,
            "resource_admission": args.admit_resources,
        },
        "requirements": {
            "events": args.require_events,
            "status_sample": args.require_status_sample,
            "cli_smoke_pass": args.require_cli_smoke_pass,
            "endpoints_pass": args.require_endpoints_pass,
            "screenshots": args.require_screenshots,
        },
        "artifact_security": {
            "directory_mode": "0700",
            "file_mode": "0600",
            "content_warning": "Command output and Git diffs are captured verbatim and may contain sensitive data.",
        },
    }
    write_json(bundle / "manifest.json", manifest)
    started_status = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "state": "started",
        "terminal": False,
        "timestamp_utc": started_utc,
    }
    append_jsonl(bundle / "status.jsonl", started_status)
    status_sampler = StatusSampler(
        status_source,
        bundle / "status-samples.jsonl",
        run_id,
        args.status_interval,
    )
    status_sampler.start()

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
    process_sampler: ProcessSampler | None = None
    timed_out = False
    artifact_limit_exceeded: str | None = None
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
                if admission_error is None:
                    process = subprocess.Popen(
                        execution_argv,
                        cwd=str(cwd),
                        env=env,
                        stdin=None,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        start_new_session=True,
                    )
                else:
                    message = f"resource admission failed: {admission_error}"
                    stderr_artifact.write((message + "\n").encode("utf-8"))
                    stderr_artifact.flush()
                    print(f"run-evidence: {message}", file=sys.stderr)
            except OSError as error:
                spawn_error = f"{type(error).__name__}: {error}"
                stderr_artifact.write((spawn_error + "\n").encode("utf-8", errors="replace"))
                stderr_artifact.flush()
                print(f"run-evidence: {spawn_error}", file=sys.stderr)

            if process is not None:
                process_sampler = ProcessSampler(
                    process.pid,
                    bundle / "processes.jsonl",
                    run_id,
                    env,
                )
                process_sampler.start()
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
                        if (bundle / "events.jsonl").stat().st_size > MAX_JSON_ARTIFACT_BYTES:
                            artifact_limit_exceeded = (
                                f"events.jsonl exceeded {MAX_JSON_ARTIFACT_BYTES} bytes"
                            )
                            print(
                                f"[evidence] {artifact_limit_exceeded}; terminating process group",
                                file=sys.stderr,
                            )
                            stop_process_group(process, args.grace)
                            break
                    except OSError:
                        pass
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
        if process_sampler is not None:
            process_sampler.stop()
        else:
            (bundle / "processes.jsonl").touch(mode=0o600, exist_ok=True)
            os.chmod(bundle / "processes.jsonl", 0o600)
        status_sampler.stop()
        for signum, handler in previous_handlers.items():
            signal.signal(signum, handler)

    command_finished_utc = utc_now()
    command_finished_monotonic = time.monotonic()
    command_duration = max(0.0, command_finished_monotonic - command_started)
    if cancelled_signal:
        terminal_state = "cancelled"
        shell_exit_code = 128 + cancelled_signal[0]
    elif timed_out:
        terminal_state = "timeout"
        shell_exit_code = 124
    elif artifact_limit_exceeded is not None:
        terminal_state = "artifact_limit"
        shell_exit_code = 125
    elif admission_error is not None:
        terminal_state = "admission_failed"
        shell_exit_code = 75
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
    diff_stat_result = run_git(cwd, "diff", "--stat", "HEAD", "--", ".", timeout=3.0)
    diff_stat_payload = diff_stat_result.stdout
    if diff_stat_result.stdout_truncated:
        marker = b"\n# [run-evidence diff stat truncated]\n"
        diff_stat_payload = diff_stat_payload[: MAX_GIT_METADATA_BYTES - len(marker)] + marker
    (bundle / "diff-stat.txt").write_bytes(diff_stat_payload)
    os.chmod(bundle / "diff-stat.txt", 0o600)

    event_limit = truncate_jsonl(bundle / "events.jsonl", MAX_JSON_ARTIFACT_BYTES)
    if event_limit.get("truncated") is True and artifact_limit_exceeded is None:
        artifact_limit_exceeded = f"events.jsonl exceeded {MAX_JSON_ARTIFACT_BYTES} bytes"
        if shell_exit_code == 0:
            terminal_state = "artifact_limit"
            shell_exit_code = 125
    artifact_limits = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "direct_write_limit_exceeded": artifact_limit_exceeded,
        "events": event_limit,
    }
    write_json(bundle / "artifact-limits.json", artifact_limits)
    events_validation = validate_events_jsonl(bundle / "events.jsonl")
    write_json(bundle / "events-validation.json", events_validation)
    event_run_ids = set(events_validation.get("run_ids", []))
    runner_run_ids = event_run_ids or status_sampler.source_run_ids
    if event_run_ids:
        sampled_rows, _ = read_jsonl(bundle / "status-samples.jsonl")
        scoped_samples = [
            row
            for row in sampled_rows
            if isinstance(row, dict) and row.get("source_run_id") in event_run_ids
        ]
        write_jsonl_records(bundle / "status-samples.jsonl", scoped_samples)
        status_sampler.samples = len(scoped_samples)
    runner_run_id = args.endpoint_run_id
    if runner_run_id is None and len(runner_run_ids) == 1:
        runner_run_id = next(iter(runner_run_ids))
    initial_event_facts = event_metrics(bundle / "events.jsonl")
    plan_id = args.plan_id
    if plan_id is None and len(initial_event_facts.get("plan_ids", [])) == 1:
        plan_id = initial_event_facts["plan_ids"][0]
    log_filter = filter_append_logs(log_baselines, bundle, {run_id, *runner_run_ids})
    endpoint_results = collect_endpoints(
        bundle,
        args.endpoint_base if admission_error is None else None,
        runner_run_id,
        plan_id,
        args.endpoint,
        timeout=args.endpoint_timeout,
        discover_openapi=not args.no_openapi,
        allow_remote=args.allow_remote_endpoints,
    )
    smoke_results = run_cli_smokes(
        bundle,
        cwd,
        run_id,
        args.cli_smoke if admission_error is None else [],
        env,
        args.hook_timeout,
    )
    screenshots = collect_screenshots(
        bundle,
        cwd,
        run_id,
        args.text_snapshot if admission_error is None else [],
        args.png_hook if admission_error is None else [],
        env,
        args.hook_timeout,
        roko_screenshot_before,
        args.collect_roko_screenshots and admission_error is None,
    )
    processes = (
        process_sampler.summary()
        if process_sampler is not None
        else {
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "samples": 0,
            "max_processes": 0,
            "observed_pids": [],
            "commands": [],
            "samples_artifact": "processes.jsonl",
        }
    )
    write_json(bundle / "processes.json", processes)

    attempt_evidence_dir = bundle / "attempt-evidence"
    attempt_evidence = (
        sorted(str(path.relative_to(bundle)) for path in attempt_evidence_dir.iterdir() if path.is_file())
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
        "bundle_finalize_duration_ms": round((time.monotonic() - command_finished_monotonic) * 1000),
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
        "process_group_id": process.pid if process is not None else None,
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
    command_records = [command_record]
    for index, smoke in enumerate(smoke_results.get("results", []), start=1):
        if not isinstance(smoke, dict) or not isinstance(smoke.get("argv"), list):
            continue
        command_records.append(
            {
                "schema_version": SCHEMA_VERSION,
                "run_id": run_id,
                "command_id": f"cli-smoke-{index}",
                "kind": "cli_smoke",
                "name": smoke.get("name"),
                "argv": smoke.get("argv"),
                "cwd": str(cwd),
                "started_utc": smoke.get("started_utc"),
                "finished_utc": smoke.get("finished_utc"),
                "duration_ms": smoke.get("duration_ms"),
                "exit_code": smoke.get("exit_code"),
                "state": "succeeded" if smoke.get("passed") is True else "failed",
                "timed_out": smoke.get("timed_out"),
                "stdout": smoke.get("stdout"),
                "stderr": smoke.get("stderr"),
            }
        )
    write_jsonl_records(bundle / "commands.jsonl", command_records)

    metrics = build_metrics(
        bundle,
        cwd,
        run_id,
        timings,
        endpoint_results,
        smoke_results,
        screenshots,
        processes,
        cache,
        admission,
    )
    if admission_error is not None:
        metrics["first_failure"] = {
            "type": "resource_admission",
            "reason": admission_error,
        }
    elif artifact_limit_exceeded is not None and metrics.get("first_failure") is None:
        metrics["first_failure"] = {
            "type": "artifact_limit",
            "reason": artifact_limit_exceeded,
        }
    write_json(bundle / "metrics.json", metrics)
    write_json(bundle / "diff-stat.json", metrics["git"])
    write_json(
        bundle / "gates.json",
        {
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "passed": metrics["verification"]["gates_passed"],
            "failed": metrics["verification"]["gates_failed"],
        },
    )
    write_jsonl_records(
        bundle / "usage.jsonl",
        [
            {
                "schema_version": SCHEMA_VERSION,
                "run_id": run_id,
                **metrics["provider"],
            }
        ],
    )

    summary = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "runner_run_ids": sorted(runner_run_ids),
        "terminal": True,
        "state": terminal_state,
        "exit_code": shell_exit_code,
        "process_exit_code": return_code,
        "timed_out": timed_out,
        "descendants_terminated_after_leader_exit": descendants_terminated_after_leader_exit,
        "cancelled_signal": cancelled_signal[0] if cancelled_signal else None,
        "spawn_error": spawn_error,
        "admission_error": admission_error,
        "artifact_limit_exceeded": artifact_limit_exceeded,
        "duration_ms": timings["command_duration_ms"],
        "evidence_valid": None,
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
        "collection": {
            "status_samples": status_sampler.samples,
            "status_parse_errors": status_sampler.parse_errors,
            "filtered_log_lines": sum(row.get("lines_selected", 0) for row in log_filter["sources"]),
            "endpoint_requests": len(endpoint_results.get("results", [])),
            "cli_smokes": len(smoke_results.get("results", [])),
            "screenshots": len(screenshots.get("entries", [])),
            "process_samples": processes.get("samples", 0),
        },
        "artifacts": {
            "command": "command.txt",
            "events": "events.jsonl" if events_validation["present"] else None,
            "events_validation": "events-validation.json",
            "attempt_evidence": attempt_evidence,
            "status_samples": "status-samples.jsonl",
            "processes": "processes.json",
            "resource_admission": "resource-admission.json",
            "artifact_limits": "artifact-limits.json",
            "filtered_logs": "filtered-logs/index.json",
            "endpoints": "endpoints.json",
            "cli_smoke": "cli-smoke.json",
            "screenshots": "screenshots/manifest.json",
            "metrics": "metrics.json",
            "score": "score.json",
            "debrief": "DEBRIEF.md",
            "validation": "validation.json",
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
    score = build_score(metrics, terminal_state, None)
    write_json(bundle / "score.json", score)
    write_json(bundle / "summary.json", summary)
    terminal_status = {
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "state": terminal_state,
        "terminal": True,
        "exit_code": shell_exit_code,
        "timestamp_utc": utc_now(),
    }
    write_jsonl_records(bundle / "status.jsonl", [started_status, terminal_status])
    write_text(bundle / "DEBRIEF.md", build_debrief(manifest, summary, metrics, None))

    validation_options = {
        "require_events": args.require_events,
        "require_status_samples": args.require_status_sample,
        "require_smoke_pass": args.require_cli_smoke_pass,
        "require_endpoints_pass": args.require_endpoints_pass,
        "require_screenshots": args.require_screenshots,
    }
    validation = validate_bundle(bundle, **validation_options)
    final_exit_code = shell_exit_code
    if not validation["valid"] and shell_exit_code == 0:
        terminal_state = "evidence_invalid"
        final_exit_code = 125
    summary["state"] = terminal_state
    summary["exit_code"] = final_exit_code
    summary["evidence_valid"] = validation["valid"]
    terminal_status["state"] = terminal_state
    terminal_status["exit_code"] = final_exit_code
    timings["bundle_finalize_duration_ms"] = round((time.monotonic() - command_finished_monotonic) * 1000)
    timings["total_duration_ms"] = round((time.monotonic() - overall_started) * 1000)
    metrics["latency_ms"]["bundle_finalize"] = timings["bundle_finalize_duration_ms"]
    metrics["latency_ms"]["total"] = timings["total_duration_ms"]
    score = build_score(metrics, terminal_state, validation["valid"])
    write_json(bundle / "timings.json", timings)
    write_json(bundle / "metrics.json", metrics)
    write_json(bundle / "score.json", score)
    write_json(bundle / "summary.json", summary)
    write_jsonl_records(bundle / "status.jsonl", [started_status, terminal_status])
    write_text(bundle / "DEBRIEF.md", build_debrief(manifest, summary, metrics, validation))
    validation = validate_bundle(bundle, **validation_options)
    if not validation["valid"] and final_exit_code == 0:
        terminal_state = "evidence_invalid"
        final_exit_code = 125
        summary["state"] = terminal_state
        summary["exit_code"] = final_exit_code
        summary["evidence_valid"] = False
        terminal_status["state"] = terminal_state
        terminal_status["exit_code"] = final_exit_code
        score = build_score(metrics, terminal_state, False)
        write_json(bundle / "score.json", score)
        write_json(bundle / "summary.json", summary)
        write_jsonl_records(bundle / "status.jsonl", [started_status, terminal_status])
        write_text(bundle / "DEBRIEF.md", build_debrief(manifest, summary, metrics, validation))
        validation = validate_bundle(bundle, **validation_options)
    write_json(bundle / "validation.json", validation)

    print(
        f"[evidence] terminal={terminal_state} exit={final_exit_code} duration={command_duration:.1f}s valid={validation['valid']}",
        file=sys.stderr,
    )
    print(f"[evidence] summary={bundle / 'summary.json'}", file=sys.stderr)
    return final_exit_code


def resolve_bundle(root_raw: str, selector: str | None) -> pathlib.Path | None:
    root = pathlib.Path(root_raw).expanduser().resolve(strict=False)
    if selector:
        candidate = pathlib.Path(selector).expanduser()
        if candidate.is_dir():
            return candidate.resolve(strict=False)
        candidate = root / selector
        return candidate.resolve(strict=False) if candidate.is_dir() else None
    candidates = sorted(
        (path for path in root.iterdir() if path.is_dir() and not path.is_symlink()),
        key=lambda path: path.name,
        reverse=True,
    ) if root.is_dir() else []
    return candidates[0].resolve(strict=False) if candidates else None


def validation_cli(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description="Strictly validate one Roko evidence bundle.")
    parser.add_argument("bundle")
    parser.add_argument("--require-events", action="store_true")
    parser.add_argument("--require-status-sample", action="store_true")
    parser.add_argument("--require-cli-smoke-pass", action="store_true")
    parser.add_argument("--require-endpoints-pass", action="store_true")
    parser.add_argument("--require-screenshots", action="store_true")
    parser.add_argument("--write", action="store_true", help="refresh validation.json in the bundle")
    args = parser.parse_args(argv)
    bundle = pathlib.Path(args.bundle).expanduser().resolve(strict=False)
    result = validate_bundle(
        bundle,
        require_events=args.require_events,
        require_status_samples=args.require_status_sample,
        require_smoke_pass=args.require_cli_smoke_pass,
        require_endpoints_pass=args.require_endpoints_pass,
        require_screenshots=args.require_screenshots,
    )
    if args.write and bundle.is_dir() and not bundle.is_symlink():
        write_json(bundle / "validation.json", result)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["valid"] else 1


def feedback_cli(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description="Show the deterministic debrief for an evidence run.")
    parser.add_argument("selector", nargs="?", help="bundle path or run ID (default: latest)")
    parser.add_argument("--run-id", dest="run_id", help="run ID under --bundle-root")
    parser.add_argument("--bundle-root", default=".roko/runs")
    parser.add_argument("--json", action="store_true", help="print summary, metrics, and validation as JSON")
    args = parser.parse_args(argv)
    selector = args.run_id or args.selector
    bundle = resolve_bundle(args.bundle_root, selector)
    if bundle is None:
        print("evidence feedback: bundle not found", file=sys.stderr)
        return 2
    validation = validate_bundle(bundle)
    if args.json:
        payload = {"bundle": str(bundle), "validation": validation}
        for name in ("manifest.json", "summary.json", "metrics.json", "score.json"):
            try:
                payload[name.removesuffix(".json")] = json.loads((bundle / name).read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, json.JSONDecodeError):
                payload[name.removesuffix(".json")] = None
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        try:
            sys.stdout.write((bundle / "DEBRIEF.md").read_text(encoding="utf-8"))
        except OSError as error:
            print(f"evidence feedback: {error}", file=sys.stderr)
            return 2
        print(f"\nBundle: {bundle}\nValidation: {'valid' if validation['valid'] else 'INVALID'}")
        for error in validation["errors"]:
            print(f"- {error}")
    return 0 if validation["valid"] else 1


def percentile(values: Sequence[int | float], fraction: float) -> int | float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, int((len(ordered) * fraction + 0.999999) - 1)))
    return ordered[index]


def score_cli(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description="Aggregate metrics from completed evidence bundles.")
    parser.add_argument("selectors", nargs="*", help="bundle paths or run IDs (default: every bundle)")
    parser.add_argument("--bundle-root", default=".roko/runs")
    parser.add_argument("--output", metavar="PATH", help="also write the JSON scorecard here")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.bundle_root).expanduser().resolve(strict=False)
    bundles: list[pathlib.Path] = []
    if args.selectors:
        for selector in args.selectors:
            bundle = resolve_bundle(str(root), selector)
            if bundle and bundle not in bundles:
                bundles.append(bundle)
    elif root.is_dir():
        bundles = sorted(path.resolve(strict=False) for path in root.iterdir() if path.is_dir() and not path.is_symlink())
    rows: list[dict[str, Any]] = []
    for bundle in bundles:
        try:
            metrics = json.loads((bundle / "metrics.json").read_text(encoding="utf-8"))
            summary = json.loads((bundle / "summary.json").read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            continue
        validation = validate_bundle(bundle)
        latency = metrics.get("latency_ms", {})
        rows.append(
            {
                "run_id": summary.get("run_id"),
                "bundle": str(bundle),
                "state": summary.get("state"),
                "valid": validation["valid"],
                "command_ms": latency.get("command"),
                "finalize_ms": latency.get("bundle_finalize"),
                "total_ms": latency.get("total"),
                "launches": metrics.get("provider", {}).get("actual_launches"),
                "cost_usd": metrics.get("provider", {}).get("cost_usd"),
            }
        )
    if not rows:
        print("evidence score: no complete bundles found", file=sys.stderr)
        return 2
    scorecard: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "generated_utc": utc_now(),
        "runs": len(rows),
        "succeeded": sum(row["state"] == "succeeded" for row in rows),
        "valid": sum(row["valid"] is True for row in rows),
        "latency_ms": {},
        "rows": rows,
        "policy": "timeouts and failures are retained at their observed duration",
    }
    for key in ("command_ms", "finalize_ms", "total_ms"):
        values = [row[key] for row in rows if isinstance(row.get(key), (int, float))]
        scorecard["latency_ms"][key] = {
            "p50": percentile(values, 0.50),
            "p95": percentile(values, 0.95),
            "min": min(values) if values else None,
            "max": max(values) if values else None,
        }
    rendered = json.dumps(scorecard, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(rendered)
    if args.output:
        output = pathlib.Path(args.output).expanduser().resolve(strict=False)
        output.parent.mkdir(parents=True, exist_ok=True)
        write_text(output, rendered)
    return 0


def entrypoint(argv: Sequence[str] | None = None) -> int:
    selected = list(sys.argv[1:] if argv is None else argv)
    if selected and selected[0] == "validate":
        return validation_cli(selected[1:])
    if selected and selected[0] == "feedback":
        return feedback_cli(selected[1:])
    if selected and selected[0] == "score":
        return score_cli(selected[1:])
    return main(selected)


if __name__ == "__main__":
    raise SystemExit(entrypoint())
