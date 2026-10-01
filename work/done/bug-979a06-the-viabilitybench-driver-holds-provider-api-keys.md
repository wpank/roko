+++
id = "bug-979a06"
kind = "bug"
title = "The ViabilityBench driver holds provider API keys in its own environment, where same-uid agents can read them"
status = "done"
triage = "verified"
last_verified = 2026-09-29
last_verified_rev = "161f29dda"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-secret's report on gap-a8a160)"
anchors = ["benchmarks/viabilitybench/driver/provider.py:97", "benchmarks/viabilitybench/driver/vb.py:272", "benchmarks/viabilitybench/driver/agent_env.py", "benchmarks/viabilitybench/driver/run_roko.py:163", "benchmarks/viabilitybench/arms/cheap_direct.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e003ec", "gap-a8a160", "bug-a66941", "gap-8c3752", "gap-308373"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_provider_key_in_the_driver_environment' benchmarks/viabilitybench/driver/test_secret.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_secret.py -k test_no_provider_key_in_the_driver_environment -q"

[closed]
at = 2026-09-29
commit = "161f29dda"
by = "wk-bench-fix2"
evidence = "Provider keys now live in a driver-only key file (--key-file, $VB_KEY_FILE, ~/.config/viabilitybench/keys; secret.load_keys, secret.py keys) and reach only the driver's memory: vb run hands them to the in-process metering proxy (FaultProxy(keys=...)), the chat client never reads a key, and os.environ is never read for one. secret.preflight refuses a run whose environment holds any arm's api_key_env or a key value, and forbids the keys in agent environments (escape hatch KEYS_IN_ENV_OK for tests). The [[verify]] command passes: test_no_provider_key_in_the_driver_environment runs vb.py as a process through the proxy; the stub sees the key file's key on every request while ps -E and /proc on the driver show the probe marker and no key; the refusals exit 2 before any request or directory; a silent read of the key file is leak_suspected (key-file:ctime). benchmarks/viabilitybench: 309 passed, 4 skipped."
+++

## Problem

The driver reads provider keys from its own environment:

- Each arm names an `api_key_env`: `CEREBRAS_API_KEY` for `cheap_direct` and `roko_fixed`, `OPENAI_API_KEY` for `fd_api`.
- `vb run` refuses to start unless that variable is set (vb.py:272), and `provider.py:97` reads it for every request. The Roko runner resolves the same variable for roko (run_roko.py:163).

So the key is in the driver's exec-time environment. `agent_env.build` gives agents an allowlisted environment, but an agent running as the same user can read its parent's environment: `ps -E -p $PPID` (or `ps eww`) on macOS, `/proc/<ppid>/environ` on Linux. `agent_env.py:19-22` names this limit, and wk-bench-secret verified it on macOS. Deleting the variable from `os.environ` after startup doesn't help: both `ps -E` and `/proc` show the environment the process started with.

Every direct-loop agent runs its commands as the driver's children, so every direct-loop agent can read the provider key.

## Why it matters

Pilot benchmark (epic spec-567e52), p1:

- An agent that holds a key can make unmetered calls, bypassing the ledger's budget caps and the metering proxy. That breaks the cost numbers the report rests on.
- The agent can also leak the key.

gap-a8a160 fixed the same exposure for `VB_SECRET` only, and secret.preflight refuses a run whose environment holds the secret. Nothing does the same for provider keys.

## Where

- `driver/provider.py` (:97), `driver/vb.py` (:272), `driver/run_roko.py` (:163-166): the key reads.
- `driver/agent_env.py`: the agent environment and its documented limit.
- The arms' `api_key_env`.

## Current state

At BASE (4315add32), the keys come from the driver's environment. The metering proxy (gap-e003ec) is open and doesn't hold keys yet.

## Plan

1. **Keep the keys out of the driver's exec environment.** Two options:
   - The metering proxy (gap-e003ec) reads the keys from a driver-only file (mode 0600, like the secret file) and adds the Authorization header. The driver and roko talk to the proxy on loopback, with a dummy key.
   - Or a small key helper, started with the key file, does the same for the direct loop.
2. **Fail closed.** `vb run` refuses to start when any arm's `api_key_env` is set in its own environment, the way `secret.preflight` treats `VB_SECRET`. Tests keep an explicit escape hatch.
3. **Roko arm.** Roko gets a dummy key and the proxy's URL, so the real key never reaches the Roko process tree either.
4. **Test.** Add `test_no_provider_key_in_the_driver_environment`. Start `vb run` on an offline fake with a key file. A child process that reads its parent's initial environment (`ps -E -p` or `/proc/<ppid>/environ`) finds no key value.

## Done when

- [x] No process an agent can inspect holds a provider key, and `vb run` refuses to start with one in its environment.
- [x] The `[[verify]]` command passes.

## Notes

- A container per task (S08 decision 4) would also close this, but is a larger change. See gap-308373 and gap-8c3752.
- bug-a66941 is the same class of problem for Roko's own agents and `~/.roko/.env`.
- **Done 2026-09-29 (wk-bench-fix2)**, on top of gap-e90ebd's proxy wiring (work/bug-2930a8, merged into this
  branch).
  - The keys live in a driver-only key file: `--key-file`, else `$VB_KEY_FILE`, else `~/.config/viabilitybench/keys`.
    It follows the secret file's rules, and `secret.py keys` checks it.
  - `vb run` reads the key into the driver's memory for the in-process metering proxy (`FaultProxy(keys=…)`), which
    alone sends it. The chat client never reads a key, and Roko gets the proxy's URL and a placeholder.
  - `secret.preflight` refuses a run whose environment holds any arm's `api_key_env` or a key value under any name,
    and forbids the key values in every agent environment. `secret.KEYS_IN_ENV_OK` is the tests' escape hatch, and
    `driver/conftest.py` keeps a developer's exported keys and key file out of the tests.
  - The tripwire (gap-308373) holds the key file at mode 000 for the run, so a silent read of it is `leak_suspected`
    too (`key-file:ctime`).
- **Evidence.** `test_no_provider_key_in_the_driver_environment` runs `vb.py` as its own process through the proxy.
  The stub sees the key file's key on every request. `ps -E` and `/proc` on the driver show the probe's marker but no
  key, and the key file reads "Permission denied". The refusals exit 2 before any request or directory.
- **Not covered here:**
  - Credentials that aren't arm keys and sit in the operator's environment (`ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, …)
    are still visible to agents through `ps -E`. Launch `vb` from a clean environment.
  - On Linux with `kernel.yama.ptrace_scope=0`, or on macOS with developer tools enabled, a same-uid agent could read
    the driver's memory. A container per task (S08 decision 4) closes that.
  - `run_roko._roko_env`'s "set CEREBRAS_API_KEY" message is stale. It can only fire for an unproxied network
    endpoint, which a billed Roko run no longer has.
- **Follow-up (same day, wk-bench-fix2, per the coordinator).**
  - `run_roko._roko_env` no longer reads a key from the environment. Roko gets the placeholder on a loopback URL (the
    proxy's or a stub's), and a network endpoint is refused before Roko starts, with a message that points to
    `vb run`'s proxy and `--key-file`.
  - The chat client is keyless, rather than taking `key=`, since fix1's wiring makes the proxy the only sender.
