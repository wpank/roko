+++
id = "spec-13c7ba"
kind = "spec"
title = "Hermes starts roko by itself: one roko serve per project, started by the plugin, and prebuilt binaries for people without Rust"
status = "open"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "L"
subsystem = ["roko-serve/lifecycle", "nous/plugin"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "Will, 2026-10-19: Hermes roko page unreachable; make roko start with Hermes and just work (roko-autostart round)"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

To use roko from Hermes, a `roko serve` has to be running for the project, and nothing starts one. On 2026-10-19 Will's Hermes roko page said "could not reach roko serve at http://127.0.0.1:6677 (ConnectError)". It only offered `roko serve --workdir WORKSPACE` to copy into a terminal.

To get it working by hand, the person must:
- keep a terminal open;
- use a roko binary new enough to have `/mcp` (Will's PATH resolves an August build that lacks it);
- stay on the one project the plugin knows.

## Why it matters

This is the hermes goal. Will's answer on 2026-10-19 was that roko should "just work" for anyone who installs the Hermes plugin, not only on a machine with a Rust toolchain and a server started by hand.

## Decisions (Will, 2026-10-19)

- **When:** roko starts with Hermes. Hermes starts `roko serve` when the app opens, or when a chat (desktop, CLI, Telegram…) first uses roko. It keeps running after Hermes quits, so runs in flight finish. There is no login item by default.
- **Who for:** anyone who installs the plugin. People without Rust get a prebuilt roko, downloaded by the plugin on first use or installed from a Homebrew tap. Publishing binaries needs Will's go.
- **Projects:** picked per run. The roko page and chats choose a project. Each project gets its own `roko serve` on its own port.
- **Binary on Will's Mac:** an installed copy. `./dev.sh install` copies the release build to `~/.cargo/bin/roko`, replacing the August one, and restarts idle serves onto it.
- **How it gets built:** a small round with the lead plus three agents (roko serve lifecycle, plugin backend, plugin pages). Work happens on branches, the lead merges into local main, and nothing is pushed.

## Plan

- **Phase 1 (roko):** `roko serve --detach` plus `status`/`stop`/`restart`, an automatic port, a serve that can prove its identity, clean shutdown on signals, idle exit, a registry of running serves, and `./dev.sh install`. See the child gap.
- **Phase 2 (plugin 0.7.0, local items):** the plugin finds roko and starts it per project. It chooses the project per run in the chat tools and on the page, and adds Start/Restart on the page.
- **Phase 3 (held for Will):** prebuilt release binaries with checksums, a download step in the plugin, and a Homebrew tap.

The interface contract between the three parts is in `tmp/roko-autostart/BRIEF.md`. It is a round scratch file and untracked.

## Done when

- The children are done.
- A scratch Hermes running plugin 0.7.0 starts serves for two projects without a terminal.
- Those serves keep running after the Hermes backend quits.
- An idle serve exits by itself.
- `./dev.sh install` moves idle serves to the new build.

## Notes

- Never touch `~/.hermes` or port 6677 in tests.
- Related: bug-06ac5f (stale serve PIDs trusted).
