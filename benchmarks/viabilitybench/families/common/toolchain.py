"""The host's Rust toolchain, for a Rust family's checks (F7) and the agents that work on one.

Will's decision of 2026-10-02 (gap-46fd19, task 3324): "Read-only real toolchain: pass RUSTUP_HOME and CARGO_HOME to
the sandbox read-only, with the toolchain bin on PATH; the cargo registry cache is per run."

An agent's environment and each verifier's have a HOME of their own (`driver/agent_env`, `ci/verify_verifiers`), so
rustup's `cargo` proxy finds no `~/.rustup` there and stops: "rustup could not choose a version of cargo to run".
`find` resolves the toolchain in the operator's environment instead:
- RUSTUP_HOME: `rustup show home`;
- the toolchain's own bin directory, which holds the real `cargo`, `rustc`, `rustdoc`, `rustfmt` and clippy: the
  directory of `rustup which cargo`, run from `/` so that no directory override applies (the directory `cargo`
  resolves to on a host without rustup);
- the operator's CARGO_HOME: where the `cargo` proxy lives, beside `rustup` in `<CARGO_HOME>/bin`.

`Toolchain.env(home)` gives an environment whose HOME is `home` what it needs to build, and its caller puts
`bin_dir` first on PATH:
- PATH: the toolchain's bin directory, so the real binaries run without a proxy. The operator's `<CARGO_HOME>/bin`
  stays off it: besides rustup's proxies it holds whatever `cargo install` put there.
- RUSTUP_HOME: the real one, which holds the toolchain.
- CARGO_HOME: `<home>/.cargo`, the run's own. Cargo keeps its registry cache, its locks and its last-use data there,
  so each run starts empty: no run sees what another fetched or what the operator's cache holds, and an `--offline`
  build cannot gain a dependency.

`read_only` names what `sandbox.command` keeps read-only for the code it confines: RUSTUP_HOME and the operator's
CARGO_HOME. Agent code can read and run the toolchain, but it cannot change the compiler, rustup's settings, the
proxies or the operator's cargo config for later runs. A process that only has an environment `env` built (a truth
suite started by the census) does not see the operator's CARGO_HOME, so `read_only` also names the default homes in
the user's real home directory, the password database's rather than $HOME.

API:
    Toolchain(bin_dir: Path, rustup_home: Path | None, cargo_home: Path | None)
    Toolchain.env(home: Path) -> dict[str, str]         # RUSTUP_HOME and the run's own CARGO_HOME
    find(environ: Mapping[str, str] | None = None) -> Toolchain | None      # None: no cargo on this host
    read_only(environ: Mapping[str, str] | None = None) -> tuple[Path, ...]  # existing directories only
"""

from __future__ import annotations

import os
import shutil
import subprocess
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

RUSTUP_TIMEOUT_S = 60.0
# The variables `find`'s answer depends on; it resolves once per distinct set of their values.
RESOLVED_FROM = ("PATH", "HOME", "RUSTUP_HOME", "CARGO_HOME", "RUSTUP_TOOLCHAIN")
_found: dict[tuple[str | None, ...], Toolchain | None] = {}


@dataclass(frozen=True)
class Toolchain:
    bin_dir: Path  # cargo, rustc, rustdoc, rustfmt and clippy themselves, not rustup's proxies
    rustup_home: Path | None  # None for a toolchain rustup did not install
    cargo_home: Path | None  # the operator's; None when no `cargo` proxy shows where it is

    def env(self, home: Path) -> dict[str, str]:
        """RUSTUP_HOME, and a CARGO_HOME of the run's own under `home` (the module docstring)."""
        variables = {"CARGO_HOME": str(Path(home) / ".cargo")}
        if self.rustup_home is not None:
            variables["RUSTUP_HOME"] = str(self.rustup_home)
        return variables


def find(environ: Mapping[str, str] | None = None) -> Toolchain | None:
    """The toolchain `environ` (by default this process's) reaches, or None when it reaches no cargo."""
    environ = os.environ if environ is None else environ
    key = tuple(environ.get(name) for name in RESOLVED_FROM)
    if key not in _found:
        _found[key] = _resolve(environ)
    return _found[key]


def read_only(environ: Mapping[str, str] | None = None) -> tuple[Path, ...]:
    """The directories a sandbox keeps read-only: RUSTUP_HOME and the operator's CARGO_HOME as `find` resolves them,
    and the default `.rustup` and `.cargo` in the user's real home directory; those that exist, each once."""
    found = find(environ)
    candidates = [found.rustup_home, found.cargo_home] if found is not None else []
    real_home = _real_home()
    if real_home is not None:
        candidates += [real_home / ".rustup", real_home / ".cargo"]
    kept: dict[str, Path] = {}
    for path in candidates:
        if path is not None and path.is_dir():
            kept.setdefault(os.path.realpath(path), path)
    return tuple(kept[real] for real in sorted(kept))


def _resolve(environ: Mapping[str, str]) -> Toolchain | None:
    search = environ.get("PATH", os.defpath)
    rustup, cargo = shutil.which("rustup", path=search), shutil.which("cargo", path=search)
    rustup_home = Path(environ["RUSTUP_HOME"]) if environ.get("RUSTUP_HOME") else None
    bin_dir = None
    if rustup is not None:
        rustup_home = rustup_home or _rustup_path(rustup, environ, "show", "home")
        real_cargo = _rustup_path(rustup, environ, "which", "cargo")
        bin_dir = real_cargo.parent if real_cargo is not None else None
    if bin_dir is None and cargo is not None and not _same_file(cargo, rustup):
        bin_dir = Path(cargo).resolve().parent  # a packaged toolchain: its own bin, not a shared /opt/.../bin
    if bin_dir is None:
        return None
    proxy = Path(rustup).parent / "cargo" if rustup is not None else None
    cargo_home = proxy.parent.parent if proxy is not None and _same_file(proxy, rustup) else None
    return Toolchain(bin_dir=bin_dir, rustup_home=rustup_home, cargo_home=cargo_home)


def _rustup_path(rustup: str, environ: Mapping[str, str], *args: str) -> Path | None:
    """The path `rustup ARGS` prints, run from `/` with `environ`; None if it fails or prints no existing path."""
    try:
        done = subprocess.run([rustup, *args], cwd="/", env=dict(environ), stdin=subprocess.DEVNULL,
                              capture_output=True, text=True, timeout=RUSTUP_TIMEOUT_S, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    printed = done.stdout.strip()
    if done.returncode != 0 or not printed or not os.path.isabs(printed) or not os.path.exists(printed):
        return None
    return Path(printed)


def _same_file(path: str | Path, other: str | None) -> bool:
    try:
        return other is not None and os.path.samefile(path, other)
    except OSError:
        return False


def _real_home() -> Path | None:
    """The user's home directory from the password database, which an environment's HOME does not change."""
    try:
        import pwd
    except ImportError:  # not a Unix host
        return None
    try:
        return Path(pwd.getpwuid(os.getuid()).pw_dir)
    except KeyError:
        return None
