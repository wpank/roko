+++
id = "gap-3cfe4f"
kind = "gap"
title = "fd_claude's agent shell can read the operator's real Anthropic keychain credential via the security wrapper"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "wave-3 follow-up reports 2026-10-02 (w3-live gap-154f93)"
discovered_from = "gap-154f93 (its own Progress notes and run_cli.py's module docstring already name the mechanism, but filed no item)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py::KEYCHAIN_WRAPPER", "benchmarks/viabilitybench/driver/agent_env.py", "benchmarks/viabilitybench/families/common/sandbox.py::profile", "benchmarks/viabilitybench/driver/census.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_agent_shell_cannot_read_the_real_keychain' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_agent_shell_cannot_read_the_real_keychain -q"
+++

## Problem

In the `fd_claude` arm, the agent's own shell commands (run same-uid as the driver, inside Claude Code's
`--dangerously-skip-permissions` session) can read the operator's **real, live Anthropic subscription OAuth
credential** from the macOS login keychain — not a benchmark secret, but Will's actual account session.

`run_cli.py` sets up the session so Claude Code itself can find the subscription login despite running under a
per-task `HOME`: with `[cli] credentials = "keychain"` (the default), it writes a `security` wrapper to the
session's `.vb-bin/security` (`KEYCHAIN_WRAPPER`, written by the function at `run_cli.py:729-733`) that runs
`/usr/bin/security` with the **operator's real `HOME`** (`SECURITY = "/usr/bin/security"`, `run_cli.py:150`), while
the session keeps its own per-task `HOME` for everything else. The module's own docstring (`run_cli.py:59-63`)
already names the consequence: "The agent's shell can run the wrapper too, as it could already run
`/usr/bin/security` on the login keychain by its path: the same-uid limit `agent_env` describes."

So any shell command the agent runs — `.vb-bin/security find-generic-password -a "$USER" -w -s "Claude
Code-credentials"`, or even plain `/usr/bin/security ... ` under `HOME=<operator's real home>` directly, since
nothing stops it from setting that itself — returns the same OAuth/session blob Claude Code uses to authenticate.
`common/sandbox.py::profile` is a deny-list sandbox (`rules += f' (deny file* (subpath "{path}"))'` per path in the
caller's `deny` list); fd_claude's arm never adds the keychain, `/usr/bin/security`, or `security`'s `.vb-bin`
wrapper to that list, so nothing in the OS-level confinement blocks the read.

## Why it matters

This is not benchmark-integrity exposure (an agent reading a hidden-test secret) — it is personal credential theft
against the operator. A compromised, adversarial, or merely curious task could have its agent print the token into
its own transcript ("could carry it" into a saved bundle a human reviews later, exactly as reported), or simply
reason over it inline in the conversation the model itself receives. Either way the same-uid boundary that
`agent_env.build`'s "What it cannot do" section already documents for the *benchmark's own* secrets (gap-308373,
bug-979a06, bug-a66941 — all filed p1) applies here too, but to a credential with real-world reach outside the
sandbox entirely: a live Claude Code session token, usable to impersonate the operator's account wherever Claude
Code accepts it, not just inside this benchmark run.

`census.py`'s leak detection is purely canary-string based (`canary.find` over the transcript/diff/outputs,
`census.py:224-236`) — it looks for `vb-canary-…` markers, which an exfiltrated OAuth token would never contain.
So "the canaries don't catch it" is exactly right: there is no detection path for this at all, let alone
prevention.

## Where

- `benchmarks/viabilitybench/driver/run_cli.py`:
  - `SECURITY = "/usr/bin/security"` (:150) and `KEYCHAIN_WRAPPER` (:152), the wrapper script template.
  - The function writing `home / ".vb-bin" / "security"` (:729-733).
  - The module docstring's "Credentials" section (:59-63), which already states the limit in prose.
- `benchmarks/viabilitybench/driver/agent_env.py`: the "What it cannot do" section (:25-31) lists the same-uid
  limit generally but does not name the keychain wrapper as an instance of it.
- `benchmarks/viabilitybench/families/common/sandbox.py::profile` (:119-129): the deny-list sandbox that could
  block this if the keychain / `/usr/bin/security` / the wrapper's own path were in its `deny` set, but isn't
  asked to.
- `benchmarks/viabilitybench/driver/census.py` (:224-236): canary-only leak detection, no path for this.

## Current state

Unmitigated. The wrapper was added deliberately (so Claude Code itself, not the agent, could authenticate) by the
live-probe work (gap-154f93, done, 2026-10-02) as a fix for "Not logged in" — that work's own Progress notes state
the resulting agent-shell exposure as a known, accepted consequence, in the same prose the module docstring now
carries, but filed no follow-up item for it.

## Plan

1. Deny the vector at the sandbox, mirroring how `common/sandbox.py` already denies other paths for other arms:
   add the real keychain (`~/Library/Keychains/` under the *operator's* HOME, resolved once at startup) and
   `/usr/bin/security` itself to the agent shell's `deny` set for every arm except the one still-privileged call
   Claude Code's own process makes before handing off to the agent (if that distinction is feasible — Claude Code
   and the agent's shell commands may not be separable at the sandbox-exec layer; confirm before committing to this
   option).
2. If (1) cannot distinguish Claude Code's own process from the agent's shell children, consider option (a) from
   gap-308373/S08 decision 4 (a container or stronger per-task sandbox) instead, since the same same-uid ceiling
   applies.
3. At minimum, add a census check modelled on gap-308373's tripwire: stamp something about keychain access (macOS
   logs keychain `security` invocations in some auditable form) or, more simply, detect any shell command whose
   argv contains `security`/`find-generic-password` and mark the run `leak_suspected` even without proving the read
   succeeded — a cheap detector even if not a preventer.
4. Confirm with Will whether this is acceptable residual risk (same-uid benchmarks on a personal machine) before
   investing in prevention — S08 decision 4 already treats the general same-uid ceiling as a known, accepted-for-now
   limitation; this may be judged the same way, but should be a recorded decision, not a silent gap.

## Done when

- Either the agent's shell cannot read the operator's keychain entry at all (prevention), or a run in which it does
  is marked `leak_suspected` (detection), with a test proving it.
- The `[[verify]]` command passes.

## Notes

- Do not weaken Claude Code's own ability to authenticate — only the agent's *shell tool calls* are the target.
- Related but distinct: gap-308373 (benchmark secret file, same-uid, tripwire-detected), bug-979a06 (provider API
  keys, driver-only file), bug-32eb77 (driver's own env vars, scrubbed). None of them anchor on `run_cli.py`'s
  keychain wrapper or mention OAuth/session credentials.
- `run_cli.py:62`'s own words: "the same-uid limit `agent_env` describes" — this item makes that specific instance
  trackable instead of only prose in a docstring.
