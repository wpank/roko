# 10 — Quality of Life Improvements

**Status**: spec
**Scope**: Various files

## Overview

Small changes that compound into a dramatically better daily experience.
None of these are flashy on their own, but together they eliminate friction.

---

## Feature 10A: Welcome Banner with System Info

Show a brief, informative banner on startup:

```
◆ roko v0.1.0  ·  claude-sonnet-4-6  ·  auth: API key
│ workspace: /Users/will/dev/project  ·  .roko/ initialized
│ 3 plans  ·  12 tasks  ·  $0.42 lifetime spend
└ Type a message. Ctrl-D to exit. /help for commands.
```

- Shows version, model, auth method
- Workspace path and initialization status
- Aggregate stats from `.roko/` directory
- Replaces the current minimal header

**Implementation**: Read stats from `.roko/` on startup. ~30 lines in
`run_unified_inline` / `run_chat_inline`.

---

## Feature 10B: Smart Prompt Suggestions

When the input buffer is empty and the user hasn't typed anything for 3 seconds,
show contextual prompt suggestions:

```
                                    Try:
                                    · "fix the failing test"
                                    · "explain this error"
                                    · "refactor main.rs"
```

- Suggestions based on workspace state:
  - If tests fail: "fix the failing test in X"
  - If clippy warns: "fix clippy warnings"
  - If there are TODO comments: "implement the TODO in X"
  - Default: generic helpful prompts
- Disappear on any keypress
- Rendered in TEXT_GHOST color, right-aligned

**Implementation**: ~40 lines. Timer-based suggestion display, workspace
state detection from `.roko/` signals.

---

## Feature 10C: Response Length Indicator

Show estimated reading time for long responses:

```
◆ roko                                          ~2 min read
│ [long response...]
```

- Calculate from word count (avg 200 wpm reading speed)
- Only show for responses > 100 words
- Rendered in TEXT_GHOST next to the agent header

**Implementation**: 5 lines in `push_agent_response`.

---

## Feature 10D: Last Session Summary on Start

When starting a new chat session, if there's a previous session in the same
workspace, show a brief summary:

```
│ Last session: 2h ago  ·  5 turns  ·  $0.08
│ Last topic: "implementing the gate pipeline"
```

- Read from `.roko/episodes.jsonl` (last session's entries)
- Shows time elapsed, turn count, cost
- Extract topic from first user message

**Implementation**: ~25 lines. Parse last session from episodes file.

---

## Feature 10E: Typing Speed Indicator

Subtle typing speed indicator that shows how fast the user is typing
(fun, slightly gamified):

```
Status: ❯ 82 wpm                    (shown briefly, fades after pause)
```

- Calculate WPM from recent keystrokes
- Only show when actively typing (hide after 2s pause)
- Optional — enabled via `chat.show_typing_speed` config

**Implementation**: ~15 lines. Track keystroke timestamps, compute WPM.

---

## Feature 10F: Error Recovery Suggestions

When a dispatch error occurs, provide contextual recovery suggestions:

```
│ error    connection refused
│
│ Suggestions:
│   1. Start the server: roko serve
│   2. Check if port 6677 is in use: lsof -i :6677
│   3. Switch to direct mode: roko (no subcommand)
```

- Pattern-match on error messages
- Provide actionable commands the user can run
- Color: WARNING for suggestions, EMBER for the error

**Implementation**: ~30 lines. Error pattern matching with suggestion map.

---

## Feature 10G: Session Auto-Save

Automatically save conversation state to `.roko/sessions/` so it can be
resumed later:

```bash
roko                # starts new session or resumes last
roko --resume       # explicitly resume last session
roko --new          # force new session
```

- Save: messages, model state, cost accumulator
- Resume: show previous messages in scrollback, continue conversation
- Auto-save every 5 turns or on clean exit

**Implementation**: Serialize `ChatSession` state (minus dispatch handles).
~50 lines for save/load.

---

## Feature 10H: Rich Error Formatting

Format compilation and test errors with visual structure:

```
│ error[E0382]: use of moved value: `x`
│   ┌─ src/main.rs:12:5
│   │
│ 10│     let x = String::from("hello");
│ 11│     let y = x;
│   │             - value moved here
│ 12│     println!("{}", x);
│   │                    ^ value used here after move
│   │
│   = note: consider using `.clone()`
```

- Detect rustc/clippy error format in agent responses
- Render with proper indentation and coloring
- File paths in BONE, error codes in EMBER, suggestions in SAGE

**Implementation**: ~40 lines in markdown renderer. Pattern-match rustc output format.

---

## Priority Order

1. **10A** Welcome banner — immediate visual upgrade, trivial
2. **10F** Error recovery — reduces frustration
3. **10E** Typing speed — fun, engaging (optional)
4. **10C** Response length — helpful for long outputs
5. **10D** Last session summary — continuity between sessions
6. **10G** Session auto-save — major workflow improvement
7. **10B** Smart suggestions — contextual help
8. **10H** Rich error formatting — developer QoL
