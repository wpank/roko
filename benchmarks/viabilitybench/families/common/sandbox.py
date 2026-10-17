"""Confine the agent's code wherever it runs: its files and its network while it runs as an agent, and again while a
truth suite or the census runs it (gap-8c3752, gap-0bd49a).

A truth suite (`hidden.py`) is trusted code. It reads the benchmark secret and the private task directory (DIR, which
holds `task.json`, the spec variants and the pristine bundle), draws the hidden cases, and then runs the agent's code
on them: F4 its migration script, F1 its converter in probe processes, the driver's toy family its `clamp`. That code
runs as the driver's user, and the census opens the secret file (mode 0400) for exactly as long as `hidden.py` runs.
So each suite starts the agent's code through `command`, which denies it every file operation on the paths it is
given, the secret file and DIR (`denied`), and every outgoing connection. The census's visible re-run and the audit
battery's re-runs do the same.

- **macOS** (`KIND` "sandbox-exec"): `sandbox-exec` with a profile that allows everything else. Apple marks the tool
  deprecated, and it still works (checked on macOS 26.4). The sandbox matches resolved paths, so `profile` resolves
  them first. A read through a symlink is denied, and a hard link into a denied path cannot be made from inside. Code
  in the sandbox cannot start a setuid program either, `ps` among them. A hard link made earlier, in the agent's own
  run, changes the secret file's ctime, so the tripwire (`driver/secret.py`) has already kept the file from
  `hidden.py`.
- **Elsewhere** (`KIND` "none"): no confinement. Linux would need bubblewrap and unprivileged user namespaces, which
  this module does not use yet (gap-29ac83). The suites put the kind in their verdict, and the run record keeps it.
- **The seam for another host** (gap-29ac83). `command` asks the host's backend (`BACKENDS[KIND]`) for the argv
  prefix that applies the rules, and `_host_kind` picks `KIND`. sandbox-exec is the only backend. A Linux one
  (`unshare -n` for the rule "none", bubblewrap for the rest) adds its entry here and its detection to `_host_kind`;
  it must raise ValueError for a rule or a deny list it cannot apply, never return a prefix that applies less, since
  `kind` reports what applied.

**The network** (gap-0bd49a, decision 3301's option (b)). `command` takes a network rule, which binds every process
the command starts. Confined code gets no network unless its rule names what it may reach:
- "none" (`NETWORK_NONE`), the default: no outgoing connection at all, Unix sockets and so DNS included
  (`(deny network-outbound)`). The direct loop runs every agent command under it, since the driver makes every model
  call itself. Every truth suite and the audit battery run the agent's code under it too, by default, and the
  census's visible re-run names it: none of them needs a network, and a fetch there could bring in what the agent
  could not fetch while it ran.
- "loopback:<port>[,<port>...]": outgoing connections only to those ports on this host's loopback (127.0.0.1 and
  ::1). The Roko and mini-swe-agent arms run under the metering proxy's port, and the Claude Code and Codex arms under
  their egress proxy's. `sockets` adds the Unix sockets under the given directories, such as Roko's per-run `inject`
  socket.
- None: no rule, so the network stays open. Nothing that runs the agent's code passes it; with nothing to deny
  either, `command` returns argv as it is.
Binding and accepting stay allowed, so a test may listen on a port, yet nothing inside can connect to a port the rule
does not name. An unknown rule raises ValueError, on every host, rather than running unconfined. The rule confines
the processes' own sockets: a system service the code asks over Mach IPC is not confined by it. `kind` reports
"sandbox-exec+net" when a network rule applies; off macOS it reports "none", since nothing applies it there, and the
run record says so.

**The Rust toolchain** (gap-46fd19, Will's decision of 2026-10-02). Every sandbox that `command` applies also keeps
the host's Rust toolchain read-only (`toolchain.read_only`: RUSTUP_HOME and the operator's CARGO_HOME), so the
confined code can read and run cargo and rustc but cannot change them, rustup's settings or the operator's cargo
config for later runs. Passing `read_only` replaces that default list. It adds to a sandbox that `deny` or a network
rule applies and on its own applies none, so `kind` is the same with or without it. The toolchain reaches the
confined code through its environment (`toolchain.Toolchain.env`, which `driver/agent_env` and the verifier CI
apply).

API:
    KIND: str                                           # "sandbox-exec" or "none", on this host
    NETWORK_NONE: str                                   # "none", the default rule: no outgoing connection
    BACKENDS: dict                                      # KIND -> the argv prefix that applies the rules
    command(argv: Sequence[str], *, deny: Iterable[str | Path], network: str | None = NETWORK_NONE,
            sockets: Iterable[str | Path] = (), read_only: Iterable[str | Path] | None = None) -> list[str]
    kind(deny: Sequence[str | Path], network: str | None = NETWORK_NONE) -> str   # what `command` applies here
    loopback(*ports: int) -> str                        # the rule for those loopback ports
    ports(network: str | None) -> tuple[int, ...]       # the loopback ports a rule admits; ValueError if unknown
    denied(secret_file: str | Path, task_file: str | Path) -> tuple[Path, Path]   # the secret file and DIR
    profile(deny: Iterable[str | Path], network: str | None = NETWORK_NONE, sockets: Iterable[str | Path] = (),
            read_only: Iterable[str | Path] = ()) -> str
"""

from __future__ import annotations

import os
import re
import sys
from collections.abc import Callable, Iterable, Sequence
from pathlib import Path

from . import toolchain

SANDBOX_EXEC = "/usr/bin/sandbox-exec"
NETWORK_NONE = "none"
NETWORK_SUFFIX = "+net"  # on KIND when a network rule applies
LOOPBACK_RULE = re.compile(r"loopback:([0-9]{1,5}(?:,[0-9]{1,5})*)")


def _host_kind() -> str:
    """The confinement this host can apply: "sandbox-exec" on macOS, else "none". A Linux backend (gap-29ac83) is
    detected here, next to its entry in BACKENDS."""
    if sys.platform == "darwin" and os.access(SANDBOX_EXEC, os.X_OK):
        return "sandbox-exec"
    return "none"


KIND = _host_kind()


def command(argv: Sequence[str], *, deny: Iterable[str | Path], network: str | None = NETWORK_NONE,
            sockets: Iterable[str | Path] = (), read_only: Iterable[str | Path] | None = None) -> list[str]:
    """`argv` run so that every file operation on a path in `deny` fails, every write to a path in `read_only` (by
    default the Rust toolchain, `toolchain.read_only`) fails and every outgoing connection the `network` rule does not
    admit fails, none by default; `argv` itself when there is no sandbox on this host (`KIND`), or when there is
    nothing to deny and no rule (None). An unknown rule raises ValueError on every host."""
    deny, sockets = list(deny), list(sockets)
    ports(network)  # refuse an unknown rule here too, not only where a profile is built
    if KIND == "none" or not (deny or network is not None):
        return list(argv)
    kept = toolchain.read_only() if read_only is None else list(read_only)
    return [*BACKENDS[KIND](deny, network, sockets, kept), *argv]


def kind(deny: Sequence[str | Path], network: str | None = NETWORK_NONE) -> str:
    """The confinement `command` applies for `deny` and `network` on this host: "none" when there is nothing to apply
    or no sandbox, else KIND, with "+net" when a network rule applies (by default it does)."""
    ports(network)
    if KIND == "none" or not (deny or network is not None):
        return "none"
    return KIND + NETWORK_SUFFIX if network is not None else KIND


def loopback(*port_numbers: int) -> str:
    """The rule that admits outgoing connections to these loopback ports only."""
    rule = "loopback:" + ",".join(str(port) for port in port_numbers)
    ports(rule)
    return rule


def ports(network: str | None) -> tuple[int, ...]:
    """The loopback ports a network rule admits: none for "none" or no rule. Raises ValueError for any other rule,
    and for a port outside 1-65535."""
    if network is None or network == NETWORK_NONE:
        return ()
    match = LOOPBACK_RULE.fullmatch(network) if isinstance(network, str) else None
    found = tuple(int(port) for port in match[1].split(",")) if match else ()
    if not found or not all(1 <= port <= 65535 for port in found):
        raise ValueError(f'unknown network rule {network!r}: use None, "none" or "loopback:<port>[,<port>...]"')
    return found


def denied(secret_file: str | Path, task_file: str | Path) -> tuple[Path, Path]:
    """What a truth suite denies the agent's code: the secret file, and DIR, the directory of `task.json`."""
    return Path(secret_file), Path(task_file).parent


def profile(deny: Iterable[str | Path], network: str | None = NETWORK_NONE, sockets: Iterable[str | Path] = (),
            read_only: Iterable[str | Path] = ()) -> str:
    """A sandbox-exec profile that allows everything but file operations on the resolved `deny` paths, writes to the
    resolved `read_only` paths and, under a `network` rule (by default "none"), outgoing connections the rule does
    not admit. A later rule wins, so the allows follow the deny."""
    paths = sorted({os.path.realpath(path) for path in deny})
    rules = "".join(f' (deny file* (subpath "{_quoted(path)}"))' for path in paths)
    rules += "".join(f' (deny file-write* (subpath "{_quoted(path)}"))'
                     for path in sorted({os.path.realpath(path) for path in read_only}))
    if network is not None:
        rules += " (deny network-outbound)"
        rules += "".join(f' (allow network-outbound (remote ip "localhost:{port}"))' for port in ports(network))
        rules += "".join(f' (allow network-outbound (subpath "{_quoted(path)}"))'
                         for path in sorted({os.path.realpath(path) for path in sockets}))
    return "(version 1) (allow default)" + rules


def _sandbox_exec(deny: list[str | Path], network: str | None, sockets: list[str | Path],
                  read_only: list[str | Path]) -> list[str]:
    return [SANDBOX_EXEC, "-p", profile(deny, network, sockets, read_only)]


# KIND -> the argv prefix that applies `deny`, `network`, `sockets` and `read_only` (module docstring, "the seam").
BACKENDS: dict[str, Callable[[list[str | Path], str | None, list[str | Path], list[str | Path]], list[str]]] = {
    "sandbox-exec": _sandbox_exec,
}


def _quoted(path: str) -> str:
    return path.replace("\\", "\\\\").replace('"', '\\"')
