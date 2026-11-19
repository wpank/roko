+++
id = "gap-195374"
kind = "gap"
title = "The portal looks like nothing else roko makes and tells a first-time user nothing: ROSEDUST dark only, and an empty project shows PLANS 0 beside an enabled Run all and the same sentence twice; move it to roko's design system (day and night tokens, self-hosted Archivo and Martian Mono, the kit as components) with a top bar, a plan list, an empty-project intro, a new-plan box, offline and sign-in pages and a phone layout"
status = "done"
triage = "verified"
severity = "p1"
goal = "visibility"
size = "L"
subsystem = ["apps/portal"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "862646e54"
source = "round v8 portal-v8 audit (tmp/hermes-roko-v8/design/portal/before, main bd93521ee), the lead's approved proposal (tmp/hermes-roko-v8/design/portal/README.md, 2026-11-12)"
discovered_from = "agent:portal-v8 (round v8, bd93521ee)"
anchors = ["apps/portal/src/styles/rosedust.css", "apps/portal/src/app/layout.tsx::RootLayout", "apps/portal/src/components/shell/Workspace.tsx::Workspace", "apps/portal/src/components/rail/PlanRail.tsx::PlanRail", "apps/portal/src/components/stage/Stage.tsx::Stage"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF 'an empty project says what roko is and asks for a request' apps/portal/src && test -f apps/portal/public/fonts/OFL-Archivo.txt && (cd apps/portal && npx vitest run src/components/shell/look.accept.test.tsx)"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T12:11:45Z"
commit = "862646e54"
by = "portal-v8 (round v8)"
executor = "claude-agent"
via = "manual"
model = "claude-opus-5-5"
size = "L"
claimed_at = "2026-11-12T11:18:00Z"
forced = false
evidence = "apps/portal moves to roko's design system: roko-tokens.css and roko-kit.css with aliases for the plan view, Archivo and Martian Mono self-hosted (OFL), the kit as components (DotMatrix, Key, Led, Strip, Wordmark, ThemeSwitch), TopBar with the status chip, PlanList, Intro and HowItWorks, SignIn, the phone picker. Checks at 862646e54 in roko-work-gap-195374: tsc clean, vitest 809/809 (look.accept 11 cases, planList.accept 10, roko.accept, words, planWords, statusChip, dots), orphans none, build:export OK. Round serve shots, day and night, desktop and phone: tmp/hermes-roko-v8/design/portal/item1/."
+++

## Problem

The portal (`apps/portal`, the static export `roko serve` hosts at `/`) looks like nothing else roko makes, and it
tells a first-time user nothing.

- **The look** is ROSEDUST (Undertow's phosphor look), dark only, square everything, 12px grey monospace for every
  word and number. roko's other surfaces (the Hermes page, the guide, the terminal design) share one design system
  that the portal ignores.
- **An empty project** (`tmp/hermes-roko-v8/design/portal/before/00-empty-workspace.png`): a black screen, `ws · main`,
  `PLANS 0` beside an enabled `▶ Run all`, a filter with nothing to filter, the same sentence twice ("No plans in ws
  yet. Describe what you want to build."), four example chips unrelated to the project, and a dead
  `transcript │ checks ▶` bar. Nothing says what roko is, what a plan is, or what Generate will do.
- **Offline and signing in:** with serve stopped, the alert keeps a stale error and "Lost the server" shows only in
  the rail; signed out, the same sentence shows three times while every key stays live.
- **Small screens:** at 390px the 336px plan rail fills the screen and the plan is a 40px column.

## Why it matters

Goal visibility. Will shows roko to Nous Research this round, and the portal is roko's own face in a browser. The
lead approved the overhaul on 2026-11-12 (`tmp/hermes-roko-v8/design/portal/README.md`). This item is its
foundation: the look and the frame every later item builds on (gap-60f6b6: the plan page, gap-6d2dd2: where the work went,
gap-6c4d24: writing and fixing a plan).

## Where

- `apps/portal/src/styles/` (`rosedust.css`, `tokens.css`, `globals.css`): the ROSEDUST tokens and the shell's classes.
- `apps/portal/src/app/layout.tsx`: `<html class="dark">`, no theme choice.
- `apps/portal/src/components/shell/` (`Workspace.tsx`, `Header.tsx`, `AlertBand.tsx`) and
  `apps/portal/src/components/rail/` (`PlanRail.tsx`, `PlanRow.tsx`).
- `apps/portal/src/components/stage/Stage.tsx`, `PromptPanel.tsx` (the empty project and the new-plan box) and
  `apps/portal/src/lib/emptyState.ts` (the empty and offline sentences).
- The design system: `tmp/hermes-roko-v8/design/roko-design-system.md` (§5.2 is the portal), with
  `design/kit/roko-tokens.css`, `roko-kit.css`, `roko-dots.js`. The proposal's mock: `design/portal/mock/`.

## Plan

1. **Tokens and theme.** Replace ROSEDUST with the kit's tokens (`--rk-*`): day (aluminium), night (graphite) and the
   glass, which is black in both. Day and night follow the system; a switch in the top bar overrides it and is kept
   in `localStorage` (wrapped in try/catch), applied before first paint so the page never flashes. The old token names
   the current plan view uses become aliases of the new ones, so it repaints until gap-60f6b6 replaces it.
2. **Fonts.** Self-host the latin woff2 subsets of Archivo (90 KB) and Martian Mono (38 KB), both OFL 1.1, in
   `apps/portal/public/fonts/` with their licence files beside them. System fallbacks hold the look without them.
3. **The kit as components** (`src/components/kit/`): dot-matrix numerals (the plugin's 5x7 glyph table, from
   `roko-dots.js`; changed glyphs roll in; an `aria-label` with the plain value), glass readout and cells, key (the
   orange key once per view), pill, LED, meter, pips, strip, fold.
4. **The top bar:** roko's wordmark on glass, the project's name and branch, the status chip (the most urgent state:
   needs you, a failed run nobody opened, running with its time, idle, off; a click selects that plan), and the day
   and night switch.
5. **The plan list:** one row per plan: an LED, the title, one status line (`ready · 3 tasks`, `running · 1 of 3 ·
   0:09`, `passed · 20s · $0.44`, `failed · 13s · $0.30`, `needs you · 1 problem`), and its strip (one square per
   task). `new plan` (key `n`) at the top; `run all` at the foot only when two or more plans can run; the filter
   only past 8 plans. Groups stay, collapsible.
6. **An empty project:** "Ask for a change. Get checked work.", two sentences on what roko does, the request box with
   the orange `write plan`, and four steps: ask, review, run, merge.
7. **The new-plan box:** "What should roko do in <project>?", three examples, "roko writes a plan first: small
   tasks, each with a check. Nothing runs until you say so." and `write plan` (⌘↵).
8. **Offline and signing in:** one sentence and the one command or key that fixes it, on its own page; nothing
   else on screen pretends to work.
9. **Phones (below 820px):** the plan list becomes a picker under the top bar; the stage takes the width.
10. Remove what this replaces (the orphan check in CI fails on unreachable files).

## Done when

- `npm run typecheck`, `npm test`, `node scripts/orphans.mjs` and `npm run build:export` pass (CI's portal job).
- The new accept test passes: `src/components/shell/look.accept.test.tsx`, with "an empty project says what roko is
  and asks for a request" among its cases.
- Screenshots from a round serve, day and night, desktop and phone, match the mock's frame
  (`tmp/hermes-roko-v8/design/portal/after/`).

## Notes

- Keep the API contracts as they are (P-3 validate, P-7 run and the resume question from gap-0eb763).
- Keep `<meta name="roko-portal">`: CI and the release check grep the export for it.
- Words follow the design system's §6 (no task ids, no engine terms on screen).
