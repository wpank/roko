# PRD-06: Standalone Visual Analysis — `roko viz fb` and `roko viz insp`

**Prerequisites**: PRD-00 through PRD-05 define the full UiGate pipeline. This PRD defines a standalone, single-shot variant that runs outside the gate pipeline — no running app, no orchestration, no retry loop. Just: screenshots in, analysis out.

---

## 1. What This Is

Two CLI commands (and matching API endpoints) for one-shot visual analysis:

1. **`roko viz fb`** — "Feedback." Accepts screenshots (or URLs, or HTML files). Scores them across every dimension from PRDs 02–03. Outputs a markdown document with quantified scores, human-readable findings, and an agent-agnostic checklist of self-contained improvement tasks. Each checklist item includes enough context that a fresh agent can pick it up in isolation, implement the fix, and move on — no prior conversation or project state needed.

2. **`roko viz insp`** — "Inspiration." Accepts screenshots, URLs, or any visual reference (mood boards, design mockups, photographs). Extracts the design system: tokens, typography, spacing, color palette, shadows, radii, motion language, component patterns, visual identity, aesthetic fingerprint, brand voice — in both quantified W3C DTCG format and human-readable prose. A developer or agent can use the output to replicate the visual language.

Both commands support batch mode: run against multiple inputs and aggregate or index the results.

---

## 2. CLI Interface

### 2.1 Feedback Mode

```bash
# Single screenshot
roko viz fb screenshot.png -o result.md

# Multiple screenshots
roko viz fb desktop.png mobile.png tablet.png -o result.md

# Folder of screenshots
roko viz fb ./screenshots/ -o result.md

# Live URL (navigates, captures, then analyzes)
roko viz fb https://myapp.com/dashboard -o result.md

# Live URL with viewports
roko viz fb https://myapp.com/dashboard \
  --viewport desktop:1440x900 \
  --viewport mobile:390x844:mobile \
  -o result.md

# Folder of static HTML files
roko viz fb ./dist/ --html -o result.md

# With design token reference for adherence scoring
roko viz fb screenshot.png --tokens design-tokens.json -o result.md

# With golden reference for regression comparison
roko viz fb current.png --golden golden.png -o result.md

# With context (helps the judge understand intent)
roko viz fb screenshot.png \
  --goal "SaaS dashboard with project cards and a creation modal" \
  --acceptance "Cards should be responsive, modal should work on mobile" \
  -o result.md

# Batch: multiple URLs, individual reports + summary index
roko viz fb https://app.com/login https://app.com/dashboard https://app.com/settings \
  --batch-dir ./reports/ \
  --batch-mode individual+summary

# Batch: folder of screenshots, single merged report
roko viz fb ./screenshots/ \
  -o merged-report.md \
  --batch-mode merged

# JSON output (for API consumption)
roko viz fb screenshot.png --format json -o result.json
```

### 2.2 Inspiration Mode

```bash
# Extract design system from a URL
roko viz insp https://stripe.com -o stripe-design.md

# Extract from a screenshot (less accurate, tagged as low-confidence)
roko viz insp stripe-homepage.png -o stripe-design.md

# Extract from multiple URLs (cross-page consistency report)
roko viz insp https://stripe.com https://stripe.com/pricing https://stripe.com/docs \
  -o stripe-design.md

# Extract from a folder of reference images (mood board)
roko viz insp ./mood-board/ -o vibe.md

# Full extraction with DTCG token output
roko viz insp https://linear.app \
  -o linear-design.md \
  --tokens linear-tokens.json \
  --format md+dtcg

# Batch: analyze multiple reference sites
roko viz insp https://stripe.com https://linear.app https://vercel.com \
  --batch-dir ./references/ \
  --batch-mode individual+summary

# With specific extraction focus
roko viz insp https://stripe.com \
  --extract color,typography,spacing,shadows,motion,components \
  -o stripe-design.md
```

### 2.3 API Mode

Both commands are also available via the `roko-serve` HTTP API:

```
POST /api/viz/fb
  Body: { screenshots: [base64...], urls: [...], goal: "...", tokens_path: "...", format: "md" }
  Response: { report_md: "...", scores: {...}, checklist: [...] }

POST /api/viz/insp
  Body: { screenshots: [base64...], urls: [...], extract: [...], format: "md+dtcg" }
  Response: { report_md: "...", tokens: {...}, analysis: {...} }
```

And via the Rust library API:

```rust
use roko_viz::{FeedbackAnalysis, InspirationExtraction};

let result = FeedbackAnalysis::new()
    .screenshot("screenshot.png")
    .goal("SaaS dashboard")
    .tokens("tokens.json")
    .run()
    .await?;

let extraction = InspirationExtraction::new()
    .url("https://stripe.com")
    .extract(&[Extract::Color, Extract::Typography, Extract::Spacing])
    .run()
    .await?;
```

---

## 3. Input Handling

### 3.1 Input Types

| Input | How It's Processed |
|---|---|
| `.png` / `.jpg` / `.webp` file | Direct screenshot analysis. No DOM, no CSS, no computed styles. Screenshot-only metrics. |
| Multiple image files | Each analyzed separately. Results merged or indexed per batch mode. |
| Folder of images | Discover all image files. Process each. |
| Live URL | Navigate with Playwright/chromiumoxide. Capture screenshot(s) at configured viewports. Collect DOM, computed styles, accessibility, CSS, network. Full metrics available. |
| Multiple URLs | Each navigated and captured separately. |
| `.html` file | Open in headless browser via `file://` protocol. Capture like a URL. |
| Folder of `.html` files | Discover all HTML files. Process each. |

### 3.2 Fidelity Levels

The analysis depth depends on input type:

| Capability | Screenshot Only | URL / HTML |
|---|---|---|
| Visual judge panel (BT pairwise) | ✓ | ✓ |
| APCA contrast per element | ✗ (approximated from pixels) | ✓ (getComputedStyle) |
| axe-core accessibility | ✗ | ✓ |
| IBM Equal Access | ✗ | ✓ |
| Tab-order graph | ✗ | ✓ |
| Console errors | ✗ | ✓ |
| Network requests | ✗ | ✓ |
| Layout metrics (overflow, clipping) | ✗ (heuristic from pixels) | ✓ (DOM measurement) |
| Core Web Vitals (LHCI) | ✗ | ✓ |
| Design token extraction | ✗ (inferred from pixels) | ✓ (CSS/computed styles) |
| Token adherence scoring | ✗ (low-confidence pixel estimation) | ✓ (exact CSS values) |
| Reduced-motion compliance | ✗ | ✓ |
| AIM metrics | ✓ (from screenshot) | ✓ (from screenshot) |
| Saliency (DeepGaze + UMSI++) | ✓ (from screenshot) | ✓ (from screenshot) |
| Colorfulness (Hasler-Süsstrunk) | ✓ (from screenshot) | ✓ (from screenshot) |
| Element density | ✗ (estimated from contour detection) | ✓ (DOM count) |
| Visual regression (odiff + dssim) | ✓ (if golden provided) | ✓ (if golden provided) |
| Gestalt analysis | Partial (from visual features) | ✓ (DOM + visual) |
| Typography analysis | Partial (OCR-based) | ✓ (font stacks, sizes, weights from CSS) |

The report always states which fidelity level was used and marks low-confidence results.

### 3.3 URL Navigation

When a URL is provided:

1. Launch headless Chromium (via Playwright or chromiumoxide).
2. Pin viewport to configured dimensions (default: 1440×900 desktop).
3. Navigate with `waitUntil: networkidle`.
4. Freeze animations (`Animation.setPlaybackRate(0)`).
5. Capture screenshot.
6. Collect DOM snapshot, computed styles, accessibility snapshot, CSS source.
7. Run axe-core + IBM Equal Access.
8. Collect layout metrics.
9. If additional viewports specified, create new contexts and repeat.
10. Close browser.

For `viz insp` with `--pages N`, auto-discover links from navigation and analyze up to N pages for a cross-page consistency report.

---

## 4. `viz fb` Output: The Feedback Report

### 4.1 Report Structure

The output markdown has five sections:

```markdown
# Visual Feedback Report
> Generated by `roko viz fb` on {date}
> Input: {input_description}
> Fidelity: {screenshot-only | full-dom}

## 1. Scorecard
{Quantified scores across all dimensions — the numbers.}

## 2. Findings
{Human-readable findings ordered by severity — what's wrong and why.}

## 3. Improvement Checklist
{Self-contained tasks, each with full context for an isolated agent.}

## 4. Evidence
{Screenshots, metric details, tool outputs.}

## 5. Appendix: Agent Task Definitions
{Structured TOML task blocks for Roko agents.}
```

### 4.2 Section 1: Scorecard

```markdown
## 1. Scorecard

### Overall: 6.8 / 10 — Needs Improvement

| Dimension | Score | Weight | Status |
|---|---|---|---|
| Task completion | 0.85 | 0.25 | ✓ Pass |
| Layout integrity | 0.52 | 0.20 | ✗ Below threshold |
| Responsive quality | 0.40 | 0.15 | ✗ Below threshold |
| Interaction clarity | 0.70 | 0.10 | ⚠ Marginal |
| Visual polish | 0.76 | 0.10 | ✓ Pass |
| Design-system fit | 0.65 | 0.10 | ⚠ Marginal |
| Accessibility | 0.80 | 0.10 | ✓ Pass |

### Computational Metrics

| Metric | Value | Threshold | Status |
|---|---|---|---|
| APCA body text pass rate | 0.87 | ≥0.95 | ✗ FAIL |
| Grid adherence | 0.92 | ≥0.95 | ⚠ WARN |
| Alignment score | 0.78 | ≥0.70 | ✓ PASS |
| Modular scale conformity | 0.95 | ≥0.90 | ✓ PASS |
| Color palette compactness | 0.91 | ≥0.95 | ⚠ WARN |
| Colorfulness (Hasler-Süsstrunk) | 28.3 | 15–35 | ✓ PASS |
| Element density | 34 | ≤50 desktop | ✓ PASS |
| Text/whitespace ratio | 0.32 | 0.15–0.40 | ✓ PASS |
| Visual balance (BM_v) | 0.12 | <0.15 | ✓ PASS |
| AIM Feature Congestion | 3.2 | <5.0 | ✓ PASS |
| AIM Grid Quality | 0.68 | ≥0.60 | ✓ PASS |
| Token adherence (overall) | 0.74 | ≥0.90 | ✗ FAIL |

### Hard Gates (if URL input)

| Gate | Status | Details |
|---|---|---|
| axe-core critical/serious | ✗ 2 violations | color-contrast (1), button-name (1) |
| IBM Equal Access | ✓ 0 additional | — |
| Console errors | ✓ 0 | — |
| Horizontal overflow | ✗ mobile | Document 431px > viewport 390px |
```

### 4.3 Section 2: Findings

```markdown
## 2. Findings

### Critical

1. **APCA contrast failure on body text** — 13% of body text elements have Lc below 60.
   - Worst offender: gray (#9CA3AF) on white (#FFFFFF), Lc = 47.2 (required: 60)
   - Location: `.card-description` paragraphs across all project cards
   - Impact: Text is perceptually hard to read, especially for users over 40

2. **Horizontal overflow on mobile** — Document width 431px exceeds 390px viewport.
   - Cause: `.project-grid` uses `min-width: 400px` on child cards
   - Location: Dashboard main content area
   - Impact: Mobile users see horizontal scroll, content is cut off

### High

3. **Token adherence at 74%** — 26% of CSS values don't match any design token.
   - Top violations by area:
     - Hero background: `#1a1a2e` (nearest token: `color.surface.dark` = `#1e1e2e`, ΔE = 3.1)
     - Card border-radius: `6px` (nearest token: `radius.md` = `8px`, Δ = 2px)
     - Section padding: `18px` (nearest token: `spacing.5` = `20px`, Δ = 2px)

### Medium

4. **Saliency mismatch** — Primary CTA ("Create Project") is NOT in top-3 saliency peaks.
   - Top saliency: hero illustration (rank 1), navigation logo (rank 2), section heading (rank 3)
   - CTA is rank 5 — users' eyes will reach it late
   - Suggested: Increase CTA size, contrast, or visual weight

5. **Grid adherence at 92%** — 8% of spacing values not on 4px/8px grid.
   - Violations: `margin-top: 18px` (×3), `padding: 14px` (×2)
```

### 4.4 Section 3: Improvement Checklist

Each item is **self-contained**. A fresh agent can read just this one item and implement the fix without any prior context about the project, the conversation, or the other checklist items.

```markdown
## 3. Improvement Checklist

### Task 1: Fix APCA contrast on card descriptions
**Priority**: Critical | **Estimated effort**: Small | **Files likely affected**: CSS/Tailwind styles for `.card-description`

**Context**: The dashboard has project cards, each containing a description paragraph. These paragraphs use gray text (#9CA3AF) on a white background (#FFFFFF). This combination has an APCA Lc value of 47.2, which is below the minimum threshold of 60 for body text. APCA (Accessible Perceptual Contrast Algorithm) is a perceptual contrast measure that accounts for font weight — it's stricter than WCAG 2.1 AA for thin fonts.

**What to do**: Change the text color of `.card-description` (or equivalent Tailwind class) to a darker gray that achieves Lc ≥ 60 against the white background. Recommended: `#6B7280` (Tailwind `gray-500`, Lc ≈ 63) or `#4B5563` (Tailwind `gray-600`, Lc ≈ 74).

**Acceptance criteria**:
- All `.card-description` elements have APCA Lc ≥ 60 against their background
- No other text elements regress in contrast
- Visual appearance remains cohesive with the rest of the card

**Verification**: Run `roko viz fb` on the updated screenshot. Check "APCA body text pass rate" is ≥0.95.

---

### Task 2: Remove horizontal overflow on mobile viewport
**Priority**: Critical | **Estimated effort**: Small | **Files likely affected**: CSS for `.project-grid` or equivalent grid/flex container

**Context**: When viewed at 390×844 (iPhone 14 equivalent), the page's document width is 431px, exceeding the viewport by 41px. This causes a horizontal scrollbar and clips content on the right. The root cause is a CSS declaration on `.project-grid` child elements with `min-width: 400px`, which cannot shrink below 400px even when the viewport is 390px.

**What to do**: Replace `min-width: 400px` with a responsive value. Options:
- `min-width: min(400px, 100%)` — allows shrinking to viewport width
- Remove `min-width` entirely and use `width: 100%` on mobile
- Use a media query: `@media (max-width: 640px) { .project-card { min-width: 100% } }`

**Acceptance criteria**:
- At 390×844 viewport, document width ≤ viewport width (no horizontal overflow)
- Cards remain readable and usable at mobile size
- Desktop layout is unchanged

**Verification**: Run `roko viz fb <url> --viewport mobile:390x844:mobile`. Check "Horizontal overflow" is "No".

---

### Task 3: Align CSS values to design tokens
**Priority**: High | **Estimated effort**: Medium | **Files likely affected**: Global CSS variables, Tailwind config, component styles

**Context**: 26% of CSS values in the rendered page don't match any defined design token. The largest violations by visual area are: hero background color (`#1a1a2e` vs token `color.surface.dark` = `#1e1e2e`, ΔE2000 = 3.1), card border-radius (`6px` vs token `radius.md` = `8px`), and section padding (`18px` vs token `spacing.5` = `20px`). Design token adherence measures how consistently the implementation uses the defined design system values.

**What to do**:
1. Hero background: change `#1a1a2e` to `var(--color-surface-dark)` or the Tailwind equivalent.
2. Card border-radius: change `rounded-[6px]` to `rounded-lg` (8px) or the token equivalent.
3. Section padding: change `p-[18px]` to `p-5` (20px) or the token equivalent.

**Acceptance criteria**:
- Token adherence overall score ≥ 0.90
- No visual regressions (layout should look the same or better)
- All color, spacing, and radius values reference design tokens, not hardcoded values

**Verification**: Run `roko viz fb <url> --tokens design-tokens.json`. Check "Token adherence (overall)" is ≥0.90.

---

### Task 4: Increase visual prominence of primary CTA
**Priority**: Medium | **Estimated effort**: Small | **Files likely affected**: CTA button component styles

**Context**: Eye-tracking saliency analysis (using DeepGaze IIE + UMSI++ ensemble) shows the "Create Project" button ranks 5th in visual attention. The hero illustration, logo, and section heading all draw more attention. For a dashboard whose primary action is creating projects, the CTA should be in the top 3 saliency peaks.

**What to do**: Increase the button's visual weight. Options:
- Increase button size (larger font, more padding)
- Use a more saturated or contrasting background color
- Add a subtle animation or shadow to draw the eye
- Move the button to a more prominent position (above the fold, left-aligned)

**Acceptance criteria**:
- Primary CTA appears in top-3 saliency peaks when analyzed
- Button remains accessible (APCA Lc ≥ 60 for text, minimum 44×44px tap target)

**Verification**: Run `roko viz fb` on the updated screenshot. Check "Saliency on CTA" score > 0.7.
```

### 4.5 Section 4: Evidence

Raw data, screenshot paths, metric computation details. Included for auditing and debugging. Can be collapsed in rendered markdown.

### 4.6 Section 5: Appendix — Agent Task Definitions (TOML)

```toml
# Auto-generated by roko viz fb
# Each task is self-contained and can be executed in isolation

[[task]]
id = "VIZ-001"
title = "Fix APCA contrast on card descriptions"
priority = "critical"
effort = "small"
description = """
The dashboard has project cards with description paragraphs using gray text (#9CA3AF)
on white (#FFFFFF). APCA Lc = 47.2, below minimum 60 for body text. Change to
#6B7280 (Tailwind gray-500, Lc ≈ 63) or darker.
"""
acceptance = [
    "All .card-description elements have APCA Lc ≥ 60",
    "No other text elements regress in contrast",
]
verify = [
    { phase = "viz", command = "roko viz fb {url} --check apca_body_pass_rate>=0.95" }
]
context = """
APCA (Accessible Perceptual Contrast Algorithm) is stricter than WCAG 2.1 AA for
thin fonts. It models font-weight in its contrast calculation. The Myndex/apca-w3
library (Apache-2.0) computes Lc values. Threshold: Lc 60 minimum for body text,
Lc 75 preferred, Lc 45 for large text.
"""

[[task]]
id = "VIZ-002"
title = "Remove horizontal overflow on mobile viewport"
priority = "critical"
effort = "small"
description = """
At 390x844 viewport, document width is 431px (overflow of 41px).
Root cause: .project-grid child elements have min-width: 400px.
Replace with min-width: min(400px, 100%) or remove on mobile.
"""
acceptance = [
    "At 390x844 viewport, document width ≤ viewport width",
    "Cards remain readable at mobile size",
    "Desktop layout unchanged",
]
verify = [
    { phase = "viz", command = "roko viz fb {url} --viewport mobile:390x844:mobile --check no_horizontal_overflow" }
]

[[task]]
id = "VIZ-003"
title = "Align CSS values to design tokens"
priority = "high"
effort = "medium"
description = """
26% of CSS values don't match design tokens. Top violations:
- Hero background: #1a1a2e → use var(--color-surface-dark)
- Card border-radius: 6px → use rounded-lg (8px)
- Section padding: 18px → use p-5 (20px)
"""
acceptance = [
    "Token adherence overall ≥ 0.90",
    "All color, spacing, radius values reference tokens",
]
verify = [
    { phase = "viz", command = "roko viz fb {url} --tokens design-tokens.json --check token_adherence>=0.90" }
]
```

---

## 5. `viz insp` Output: The Inspiration Report

### 5.1 Report Structure

```markdown
# Design Inspiration Report
> Extracted from: {source}
> Generated by `roko viz insp` on {date}
> Fidelity: {screenshot-only | full-dom | multi-page}

## 1. Visual Identity Summary
{One-paragraph human-readable description of the design language.}

## 2. Color System
{Palette with roles, relationships, and contrast information.}

## 3. Typography System
{Type scale, font stacks, weights, line heights.}

## 4. Spacing System
{Spacing scale, grid base, common patterns.}

## 5. Shape Language
{Border radii, shadows, elevation system.}

## 6. Motion Language
{Easing, duration bands, animation patterns.}

## 7. Component Patterns
{Recurring structural patterns with CSS snippets.}

## 8. Layout Architecture
{Grid systems, breakpoints, container patterns.}

## 9. Interaction States
{Hover, focus, active, disabled patterns.}

## 10. Visual DNA
{Material language, imagery style, density, feel fingerprint.}

## 11. Brand Voice (if text extracted)
{Tone, pronoun posture, heading style, CTA verbs, microcopy patterns.}

## 12. Accessibility Baseline
{Current WCAG conformance, APCA analysis, contrast remediation suggestions.}

## 13. Cross-Page Consistency (if multi-page)
{Shared vs unique tokens across pages, Jaccard similarity.}

## 14. Design Score
{7-dimension score of the source design quality.}

## 15. Appendix: Design Tokens (W3C DTCG 2025.10)
{Machine-readable token file in standard format.}
```

### 5.2 Section 1: Visual Identity Summary

```markdown
## 1. Visual Identity Summary

**Stripe.com** presents a refined, engineering-forward design language built on
a deep navy-to-white gradient system with electric violet as the primary accent.
Typography uses a custom variable font (Söhne) at a 1.25 modular scale. The
spacing system follows an 8px grid with remarkable discipline (98% adherence).
The overall density is low-to-moderate (element density: 22 on desktop), creating
a sense of spaciousness that serves the complexity of the financial content.
Motion is restrained and springy — durations in the 200–300ms medium band with
a distinctive ease-out curve. The material language is "polished glass" — subtle
gradients, fine borders, and layered translucency create depth without heaviness.
Component library detection: shadcn/ui with heavy customization (confidence: 0.65).
```

### 5.3 Section 2: Color System

```markdown
## 2. Color System

### Palette

| Role | Token | Value | Lc vs White | Lc vs Navy |
|---|---|---|---|---|
| Action primary | `color.action.primary` | #533AFD | 52.3 | 71.8 |
| Surface default | `color.surface.default` | #FFFFFF | — | 97.2 |
| Surface dark | `color.surface.dark` | #0A2540 | 97.2 | — |
| Text body | `color.text.body` | #425466 | 63.1 | 42.8 |
| Text heading | `color.text.heading` | #0A2540 | 97.2 | — |
| Border subtle | `color.border.subtle` | #E3E8EE | 12.4 | 82.1 |
| Success | `color.feedback.success` | #30B130 | 48.7 | 55.9 |
| Warning | `color.feedback.warning` | #F5A623 | 28.6 | 72.3 |

### Color Relationships
- **Primary action** (#533AFD) has insufficient Lc (52.3) against white for body text.
  Remediation: Use only for large text (Lc ≥ 45) or interactive elements with
  underlines/borders as secondary affordances.
- **Palette compactness**: ΔE2000 max = 1.8 (excellent — tight, intentional palette)
- **Colorfulness**: M = 19.2 (within optimal 15–35 band)

### Color Extraction Confidence
- From CSS: high (getComputedStyle on 847 elements)
- Cross-validated: node-vibrant swatches match within ΔE < 3 on all primaries
```

### 5.4 Section 15: Appendix — DTCG Tokens

```json
{
  "$schema": "https://www.designtokens.org/schemas/2025.10/format.json",
  "color": {
    "action": {
      "primary": { "$type": "color", "$value": "#533afd" },
      "primary-hover": { "$type": "color", "$value": "#6c52ff" }
    },
    "surface": {
      "default": { "$type": "color", "$value": "#ffffff" },
      "dark": { "$type": "color", "$value": "#0a2540" },
      "subtle": { "$type": "color", "$value": "#f6f9fc" }
    },
    "text": {
      "body": { "$type": "color", "$value": "#425466" },
      "heading": { "$type": "color", "$value": "#0a2540" },
      "muted": { "$type": "color", "$value": "#8898aa" }
    }
  },
  "spacing": {
    "1": { "$type": "dimension", "$value": "4px" },
    "2": { "$type": "dimension", "$value": "8px" },
    "3": { "$type": "dimension", "$value": "12px" },
    "4": { "$type": "dimension", "$value": "16px" },
    "5": { "$type": "dimension", "$value": "20px" },
    "6": { "$type": "dimension", "$value": "24px" },
    "8": { "$type": "dimension", "$value": "32px" },
    "10": { "$type": "dimension", "$value": "40px" },
    "12": { "$type": "dimension", "$value": "48px" },
    "16": { "$type": "dimension", "$value": "64px" }
  },
  "typography": {
    "body": {
      "$type": "typography",
      "$value": {
        "fontFamily": "sohne-var, 'Helvetica Neue', Arial, sans-serif",
        "fontSize": "16px",
        "fontWeight": 400,
        "lineHeight": 1.6,
        "letterSpacing": "0px"
      }
    }
  },
  "borderRadius": {
    "sm": { "$type": "dimension", "$value": "4px" },
    "md": { "$type": "dimension", "$value": "8px" },
    "lg": { "$type": "dimension", "$value": "12px" },
    "xl": { "$type": "dimension", "$value": "16px" },
    "full": { "$type": "dimension", "$value": "9999px" }
  }
}
```

---

## 6. Batch Mode

### 6.1 Flags

```bash
--batch-dir ./reports/          # Directory for individual reports
--batch-mode individual         # One report per input (default)
--batch-mode merged             # Single report, sections per input
--batch-mode individual+summary # Individual + summary index
```

### 6.2 Summary Index

When `--batch-mode individual+summary` is used, generate an `index.md`:

```markdown
# Visual Analysis Batch Summary
> 5 inputs analyzed on {date}

| Input | Overall | Layout | Responsive | A11y | Token Adh. | Top Issue |
|---|---|---|---|---|---|---|
| dashboard.png | 6.8 | 0.52 | 0.40 | 0.80 | 0.74 | Horizontal overflow |
| settings.png | 8.2 | 0.85 | 0.78 | 0.92 | 0.88 | Saliency mismatch |
| modal.png | 5.1 | 0.35 | 0.25 | 0.60 | 0.65 | APCA violations |
| login.png | 9.0 | 0.92 | 0.95 | 0.95 | 0.94 | — |
| profile.png | 7.5 | 0.78 | 0.70 | 0.88 | 0.82 | Grid adherence |

### Common Issues Across Pages
1. **APCA contrast** — 3/5 pages have body text below Lc 60 threshold
2. **Token adherence** — mean 0.81 (target: ≥0.90), worst: modal (0.65)
3. **Mobile responsiveness** — 2/5 pages have horizontal overflow at 390px

### Aggregate Checklist (deduplicated)
{Merged checklist with items grouped by type, deduplicated across pages}
```

For `viz insp` batch, the summary shows cross-reference consistency:

```markdown
# Inspiration Batch Summary

| Source | Colorfulness | Density | Type Scale | Grid Base | Material |
|---|---|---|---|---|---|
| stripe.com | 19.2 | 22 | 1.25 | 8px | polished glass |
| linear.app | 14.8 | 18 | 1.20 | 8px | matte dark |
| vercel.com | 21.5 | 25 | 1.25 | 4px | crisp minimal |

### Design Language Comparison
{Prose comparison of design approaches across all reference sources}

### Synthesized Tokens
{Merged/averaged token recommendations based on all sources}
```

---

## 7. The `--check` Flag for CI Integration

For scripted use, `viz fb` supports a `--check` flag that exits nonzero if a condition fails:

```bash
# CI: fail if APCA pass rate < 0.95
roko viz fb https://myapp.com --check "apca_body_pass_rate >= 0.95"

# CI: fail if token adherence < 0.90
roko viz fb https://myapp.com --tokens tokens.json --check "token_adherence >= 0.90"

# CI: fail if any critical accessibility violations
roko viz fb https://myapp.com --check "axe_critical == 0"

# CI: multiple checks (all must pass)
roko viz fb https://myapp.com \
  --check "apca_body_pass_rate >= 0.95" \
  --check "no_horizontal_overflow" \
  --check "axe_critical == 0" \
  --check "element_density <= 50"
```

Exit codes: 0 = all checks pass, 1 = at least one check failed, 2 = infrastructure error.

---

## 8. Existing Tools to Learn From

**designlang** (Manavarya09/design-extract, MIT, Playwright, Node 20+). Extracts complete design systems from live URLs. Outputs W3C DTCG tokens, Tailwind config, shadcn/ui theme, Figma variables, AI-optimized markdown, component anatomy stubs, brand voice summary, page intent classification, visual DNA fingerprint, CSS health audit, WCAG remediation, motion tokens, multi-page consistency reports. Also provides `designlang score` for quality rating, `designlang lint` for token auditing, `designlang drift` for live-site-vs-local-tokens comparison. The closest existing tool to `roko viz insp`. Study its 19-section markdown output format and its MCP server integration.

**dembrandt** (MIT, Node.js). Simpler extraction: colors, typography, spacing, borders from live URLs. Outputs DTCG format, brand guide PDF, `DESIGN.md` for AI agents. Multi-page support via `--pages N`. Mobile viewport via `--mobile`. Faster, less comprehensive.

**Project Wallace** (`@projectwallace/css-analyzer`, MIT). Static CSS analysis extracting token-shaped data. Does not require rendering — analyzes CSS source directly.

**Design Token Extractor** (Chrome extension). Interactive extraction with Tailwind export. Useful UI patterns to learn from but not a CLI tool.

The key differentiator of `roko viz`: it combines **quantitative computational metrics** (APCA, AIM, saliency, Design2Code floor scores) with **LLM judge evaluation** (pairwise BT panel) and produces **agent-executable improvement tasks**, not just a report.

---

## 9. Implementation Notes

### 9.1 Where It Lives

```
crates/roko-cli/src/commands/viz.rs     # CLI entry point
crates/roko-viz/                        # Core analysis library
  src/
    lib.rs
    feedback.rs                         # viz fb analysis pipeline
    inspiration.rs                      # viz insp extraction pipeline
    metrics/                            # Computational metrics (shared with UiGate)
    judges/                             # Judge panel (shared with UiGate)
    report/
      markdown.rs                       # Markdown report generator
      toml_tasks.rs                     # TOML task block generator
      checklist.rs                      # Self-contained checklist item builder
    extract/
      color.rs                          # Color extraction and analysis
      typography.rs                     # Typography extraction
      spacing.rs                        # Spacing system detection
      tokens.rs                         # DTCG token assembler
      motion.rs                         # Motion/animation extraction
      components.rs                     # Component pattern detection
      voice.rs                          # Brand voice analysis (via LLM)
      dna.rs                            # Visual DNA fingerprint
    batch.rs                            # Batch mode orchestration
```

### 9.2 Shared Infrastructure with UiGate

`roko-viz` shares core infrastructure with UiGate (PRDs 01–05):
- Computational metrics engine (15 metrics)
- Judge panel infrastructure (BT aggregation, position swap)
- Browser runner (Playwright/chromiumoxide)
- Token adherence scoring algorithm
- APCA computation
- axe-core + IBM Equal Access integration
- AIM metrics
- Saliency models

The difference: UiGate runs inside the gate pipeline with retry loops. `roko viz` runs standalone with no retry — it produces the report and exits.

### 9.3 Checklist Item Generation

The checklist generator follows these rules (informed by Self-Refine, Madaan et al. NeurIPS 2023):

1. **Self-contained**: Each item includes ALL context needed. A fresh agent with zero project knowledge can read the item and implement the fix.
2. **Specific**: Reference exact CSS selectors, color values, pixel measurements. Never "fix the contrast" — always "change `.card-description` color from `#9CA3AF` to `#6B7280`."
3. **Verifiable**: Each item includes acceptance criteria AND a verification command.
4. **Isolated**: Implementing one item must not require implementing any other item first. No dependencies between checklist items.
5. **Prioritized**: Critical → High → Medium → Low. Within priority, ordered by visual area impact.
6. **Limited**: Maximum 10 items per report. If more findings exist, group related issues into a single item.

### 9.4 Performance Budget

| Operation | Screenshot Only | Full URL |
|---|---|---|
| Image loading + preprocessing | 0.5s | — |
| Browser launch + navigation | — | 3–8s |
| DOM/CSS collection | — | 1–2s |
| Computational metrics (15) | 2–5s | 2–5s |
| axe-core + IBM achecker | — | 2–7s |
| APCA computation | 1s (estimated from pixels) | 1–3s (from CSS) |
| AIM metrics | 1–3s | 1–3s |
| Saliency (DeepGaze + UMSI++) | 5–15s (GPU) | 5–15s |
| Judge panel (3 judges × 5 samples) | 15–45s | 15–45s |
| Token extraction | — | 2–5s |
| Report generation | 1–2s | 1–2s |
| **Total (single input)** | **25–70s** | **30–90s** |

The judge panel is the bottleneck. For quick feedback without the full panel, add `--fast` flag that skips the panel and uses computational metrics + single-model absolute scoring only (~5–15s total).

---

## 10. Acceptance Criteria

1. `roko viz fb screenshot.png -o result.md` produces a valid markdown report with scorecard, findings, and checklist.
2. `roko viz fb https://url -o result.md` navigates, captures, and produces a full-fidelity report.
3. `roko viz fb ./folder/ -o result.md` processes all images in the folder.
4. `roko viz fb ./dist/ --html -o result.md` opens HTML files in a browser and analyzes.
5. Each checklist item is self-contained — a fresh agent can implement it without prior context.
6. TOML appendix contains valid Roko task definitions.
7. `roko viz insp https://url -o design.md --tokens tokens.json` produces design report + DTCG file.
8. `roko viz insp screenshot.png -o design.md` works with lower confidence, clearly marked.
9. Batch mode produces individual reports + summary index OR merged report per `--batch-mode`.
10. `--check` flag exits nonzero when conditions fail (CI integration).
11. `--fast` flag skips judge panel and produces results in <15s.
12. All outputs state their fidelity level and confidence.
