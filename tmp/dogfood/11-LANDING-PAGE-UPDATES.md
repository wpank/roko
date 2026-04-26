# Landing Page Updates Checklist

> **Context**: Nunchi is pitching a16z on May 6, 2026. The landing page at `nunchi.network` will be checked by partners during diligence. It must align with the pitch narrative ("Agent Coordination Plane"), contain NO fake data, and signal production-readiness.
>
> **Current state**: The page has 7 scroll sections (Loop, Scaffold, Anatomy, Memory, Collective, Chain, Proof) with ROSEDUST dark aesthetic. Problems: mock data counters (84,213 / 12,425 / 3,240), no mention of "Agent Coordination Plane," no cost comparison with HAL numbers, inscrutable "Twelve organs. Five zones." section, no /changelog or /docs pages.
>
> **Dashboard app code**: `/Users/will/dev/nunchi/nunchi-dashboard/`
> **Screenshots of current page**: `/Users/will/dev/nunchi/roko/roko/tmp/deck/` (11 PNGs)
> **Design system (R15 locked)**: Geist Sans + Geist Mono, `#000000` bg, `#FAFAFA` text, `#0070F3` accent. Tokyo Night for code. Clack-style symbols. NO emoji.

---

## Critical (Must fix before May 6)

### C1: Remove or replace mock data

The three counters (84,213 / 12,425 / 3,240) near the top of the page are FAKE. An investor who discovers this loses trust immediately.

- [ ] OPTION A (recommended): Remove the counters entirely
- [ ] OPTION B: Replace with REAL numbers (e.g., lines of code, gate checks run, episodes logged — from actual `.roko/` data)
- [ ] OPTION C: Label clearly as "simulated" if keeping
- [ ] Verify: no fake testimonials, no "trusted by X developers" claims, no mock dashboard screenshots

---

### C2: Update hero positioning

Current: "Observe. Predict. Compound."
Problem: This is a tagline, not a value proposition. Visitors don't know what Nunchi DOES.

- [ ] Change to: "The Agent Coordination Plane" or "The durable runtime for production agents"
- [ ] Add subtext: one sentence explaining what it does — e.g., "Identity, routing, gates, and shared knowledge for multi-agent systems."
- [ ] Keep "Observe. Predict. Compound." as a secondary tagline if desired, but not the hero

---

### C3: Add /changelog page

R13 finding: "A live changelog showing 3-4 shipped items in 30 days is the most underused trust signal."

- [ ] Create `/changelog` route
- [ ] Add 3-5 recent shipped items with dates (from git log or manual)
- [ ] Format: date, title, one-sentence description
- [ ] Add to nav bar

---

### C4: Add /docs page

Even if minimal, partners click /docs during diligence.

- [ ] Create `/docs` route
- [ ] Can link to existing docs or show a "Getting Started" page
- [ ] Include: `nunchi init`, `nunchi run`, `nunchi status` examples
- [ ] Add to nav bar

---

### C5: Remove "trust layer" language

All references to "trust layer" should be replaced with "coordination plane" across the landing page codebase.

- [ ] Search for "trust" in landing page code and copy
- [ ] Replace "trust layer" with "coordination plane" or "Agent Coordination Plane"
- [ ] Verify: no residual "soulbound," "passport," "Spore," "Engram" language

---

## Important (Should fix if time allows)

### I1: Add the CLI output to the page

The `nunchi run` / `nunchi audit` output format is the most compelling visual. Show it on the landing page.

- [ ] Add a code block section showing the CLI output with syntax highlighting
- [ ] Use the R15 output format (◆ ├─ └ ✔ ✖ ⚠ identity/predict/gates/knowledge lines)
- [ ] Make it copy-pasteable (include a copy button)

---

### I2: Simplify the "Anatomy" section

"Twelve organs. Five zones. One specimen." is inscrutable to someone who hasn't read the docs.

- [ ] Simplify to something like "One runtime. Nine protocols." or remove entirely
- [ ] If keeping, add a one-sentence explanation of what each "organ" does
- [ ] Consider replacing with the layer-cake diagram (Keycard → Temporal → Nunchi)

---

### I3: Add cost comparison section

The HAL benchmark cost comparison ($44.86 → $1.42) should be visible on the landing page.

- [ ] Add a section showing side-by-side cost: "Naive agent: $44.86/task. With Nunchi: $1.42."
- [ ] Add source citation: "Princeton HAL benchmark, ICLR 2026"
- [ ] Note: HAL costs exclude caching — be honest about methodology

---

### I4: Add compliance/EU AI Act section

Currently no mention of regulatory drivers on the page.

- [ ] Add a brief section: "EU AI Act Article 50 enforcement: August 2, 2026"
- [ ] Use specific text with date and penalty — NOT a countdown timer (R10 finding: timers pattern-match to ICO marketing)
- [ ] Frame as: regulation creates the buyer, Nunchi enables the buyer

---

## Design System Alignment

### D1: Font update

If the landing page doesn't already use Geist Sans/Mono:
- [ ] Install Geist Sans and Geist Mono (free, from Vercel)
- [ ] Apply across the page
- [ ] Code blocks: Geist Mono at 14-16px (web equivalent of 24-28pt in slides)

### D2: Color verification

- [ ] Background: `#000000` (pure black, not dark gray)
- [ ] Text: `#FAFAFA`
- [ ] Accent: `#0070F3` (Vercel blue) — replaces ROSEDUST pink if applicable
- [ ] Code blocks: Tokyo Night palette on `#1A1B26`
- [ ] Note: ROSEDUST rose/pink may remain as brand identity; R15 says it's "retired from deck and terminal" but may still be valid for landing page brand. Check with founder preference.

---

## Verification

After changes:
- [ ] View on MacBook (the device partners will use)
- [ ] View on iPhone (partners check sites on mobile)
- [ ] Check all nav links work (/changelog, /docs, /customers if applicable)
- [ ] Ensure no console errors
- [ ] Load time under 3 seconds
- [ ] No broken images or missing fonts
