# PRD-02: Browser Runner, Evidence Collection, and Computational Metrics Engine

**Prerequisites**: PRD-00 (research foundations), PRD-01 (data model).

---

## 1. Overview

This document specifies the runtime components: the browser runner, the 15-metric computational engine, APCA contrast computation, AIM layout metrics, saliency scoring, design-token extraction pipeline, visual regression (odiff+dssim), Core Web Vitals collection, and reduced-motion compliance testing. Every algorithm has its formula, threshold, and source paper.

---

## 2. Browser Runner Architecture

### 2.1 Two Backend Options

**Option A: Node.js Playwright (MVP)**. `tools/roko-ui-runner.mjs`. UiGate shells out. Advantages: Playwright's full API, auto-waiting, robust locators, trace capture. Disadvantage: Node runtime dependency.

**Option B: chromiumoxide (target architecture)**. Rust-native CDP client (`mattsse/chromiumoxide`, MIT/Apache, ~0.7.x). Direct CDP gives deterministic screenshots via `Page.captureScreenshot { fromSurface:true, captureBeyondViewport:true }`, animation freeze via `Animation.setPlaybackRate(0)`, request mocking via `Fetch.enable`, DPR/viewport pinning via `Emulation.setDeviceMetricsOverride`. ~5–15ms CDP round-trip locally vs Playwright Node's 30–50ms. No Node runtime, no version-skew. Pin to single `chromium-headless-shell` major with `--headless=new` mode.

**Recommended path**: Start with Node Playwright for MVP (VG-02). Migrate to chromiumoxide for production gate (removes Node dependency from Rust gate crate). Both backends implement the `BrowserBackend` trait and produce identical `BrowserRunResult` JSON.

### 2.2 Execution Flow

```
1. Parse spec.json
2. Create output directory
3. Launch browser (Chromium)
4. Pin container fonts via fontconfig + Noto/Liberation (prevent 0.5–1% pixel drift)
5. For each viewport:
   a. Create context (viewport, mobile, touch, DPR=1 pinned)
   b. Enable tracing if save_trace
   c. Attach listeners: console, pageerror, request, response, requestfailed
   d. For each journey:
      i.    Navigate to start_url (waitUntil: networkidle)
      ii.   Freeze animations: Animation.setPlaybackRate(0)
      iii.  Capture "before" screenshot
      iv.   Execute steps in order (try-catch per step)
      v.    Capture "final" screenshot
      vi.   Collect layout metrics (page.evaluate)
      vii.  Collect accessibility: Playwright a11y snapshot + axe-core + IBM achecker
      viii. Collect DOM snapshot, visible text
      ix.   Evaluate assertions
   e. Close context
6. Compute computational metrics (Section 3)
7. Run visual regression if golden screenshots provided (Section 5)
8. Run Core Web Vitals if configured (Section 6)
9. Run reduced-motion test if configured (Section 7)
10. Stop trace/HAR
11. Write result.json + all artifacts
12. Exit 0 (even if assertions fail; exit nonzero only for infrastructure errors)
```

### 2.3 Event Capture, Locator Resolution, Step Execution

(Identical to previous PRD-02 — see Section 2.3–2.5 of prior version. All event listeners, locator resolution order, step-to-Playwright mapping tables, and error handling apply unchanged.)

### 2.4 Dev Server Management

RAII guard (`DevServerHandle`) spawns process with `setsid` (Unix) for process group. Drop kills entire process group (`killpg`). Waits for HTTP 200 with exponential backoff, 30s default timeout. Process group killing is critical — `npm run dev` spawns shell → vite/next/webpack child. Killing only shell leaves server holding port.

### 2.5 Secret Redaction

Apply to console text, network headers, HAR, DOM, extracted text. Default patterns: `sk-[A-Za-z0-9_-]+`, `Bearer [A-Za-z0-9._-]+`. Configurable via `[gates.ui.security]`.

---

## 3. Computational Metrics Engine: 15 Metrics

Every render computes these before any LLM judge. Each metric has a source paper, formula, threshold, and classification (hard gate vs soft/Pareto).

### 3.1 Token Coverage (Hard)

**Source**: W3C Design Tokens Community Group 2025.10, stylelint-plugin-carbon-tokens pattern.
**Formula**: Walk all CSS declarations. For each value, check if it resolves to a design token via alias. `coverage = token_resolved_count / total_declaration_count`. 
**Threshold**: ≥0.9. 
**Implementation**: Static lint on generated source pre-render via Project Wallace (`@projectwallace/css-analyzer`). Feedforward check — cheaper than runtime and catches literal `#3B82F6` violations instantly.

### 3.2 WCAG Contrast Pass Rate (Hard)

**Source**: WCAG 2.1 Success Criterion 1.4.3.
**Formula**: For each text element, compute contrast ratio between foreground and background. Pass if ≥4.5:1 for normal text, ≥3:1 for large text. Rate = passing / total.
**Threshold**: 1.00 (hard gate — zero tolerance).
**Implementation**: axe-core `color-contrast` rule. But note: axe uses WCAG ratio, not APCA. APCA is Tier 4.

### 3.3 APCA Body Text Pass Rate (Hard, Tier 4)

**Source**: Myndex/apca-w3 (Apache-2.0). APCA lookup table thresholds per font size and weight.
**Formula**: For each text element, compute APCA Lc value from composited foreground and background colors (handle alpha + stacking contexts, polarity-corrected). Compare against `thresholdFor(fontSize, fontWeight)`.
**Thresholds**: 
- Body text: |Lc| ≥ 60 (minimum), ≥75 (preferred)
- Large text: |Lc| ≥ 45
- Floor: |Lc| ≥ 30
- Non-text: |Lc| ≥ 15
**Pass rate**: fraction of body text elements with |Lc| ≥ 60. Target: ≥0.95.
**Why APCA over WCAG ratio**: Catches orange-button/thin-font failures that pass WCAG AA but are perceptually weak. APCA models font-weight, which WCAG ratio does not.

### 3.4 Grid Adherence (Hard)

**Source**: Material Design 3 spacing system, 4px/8px grid convention.
**Formula**: Extract all computed spacing values (margin, padding, gap) from visible elements. Count how many are divisible by the grid base (4 or 8). `adherence = divisible / total`.
**Threshold**: ≥0.95.

### 3.5 Alignment Score (Hard)

**Source**: Koch & Oulasvirta CHI 2016 Gestalt operationalization.
**Formula**: Collect x-coordinates of left edges of all major layout elements. Count unique x-coordinates. `alignment = 1 − (unique_x / element_count)`. Higher = more aligned.
**Threshold**: ≥0.7.

### 3.6 Modular Scale Conformity (Hard)

**Source**: Typographic modular scales (Bringhurst).
**Formula**: Extract all computed font sizes. For each candidate ratio r ∈ {1.125, 1.2, 1.25, 1.333, 1.414, 1.5, 1.618}, check how many font sizes fit the scale `base × r^n`. Pick the best-fitting ratio. `conformity = fitting_sizes / total_sizes`.
**Threshold**: ≥0.9.

### 3.7 Color Palette Compactness (Hard)

**Source**: Color science (CIEDE2000/OKLCH).
**Formula**: Sample pixel colors from the rendered screenshot (excluding images). Run k-means in OKLCH color space with k=5–8. For each pixel, compute ΔE2000 to nearest cluster centroid. For each cluster centroid, compute ΔE2000 to nearest design token. `compactness = fraction of pixels with ΔE2000 ≤ 2.3 to nearest token`.
**Threshold**: >0.95.
**Cross-check**: Validate against `node-vibrant`/`colorthief` on screenshot. If cluster centroid within ΔE2000<4 of a Vibrant swatch, mark as confirmed.

### 3.8 Colorfulness (Soft)

**Source**: Hasler & Süsstrunk metric. Reinecke et al. CHI 2013 first-impression band.
**Formula**: `M = sqrt(σ_rg² + σ_yb²) + 0.3 × sqrt(μ_rg² + μ_yb²)` where rg = R−G, yb = 0.5(R+G)−B across all pixels.
**Optimal band**: M ∈ [15, 35]. Outside = penalty.

### 3.9 Element Density (Soft)

**Source**: Miniukovich AVI 2014.
**Formula**: Count visible interactive + text elements in viewport.
**Thresholds**: ≤30 mobile, ≤50 desktop. Above = penalty.

### 3.10 Text/Whitespace Ratio (Soft)

**Formula**: `ratio = text_pixel_area / total_viewport_area`. Use text bounding boxes.
**Optimal band**: [0.15, 0.40]. Below = too sparse. Above = too dense.

### 3.11 Visual Balance (Soft)

**Source**: Gestalt figure/ground theory.
**Formula**: Divide viewport vertically. Compute visual weight per half (sum of element areas × contrast). `BM_v = |W_L − W_R| / max(W_L, W_R)`.
**Threshold**: <0.15 unless explicit asymmetry.

### 3.12 Saliency on CTA (Soft)

**Source**: DeepGaze IIE (arXiv 2105.12441, AUC 88.3) + UMSI++ (UEyes, CHI 2023, Zenodo 8010312, UI-specific SOTA). Ensemble-averaged.
**Formula**: Run both models on screenshot. Average saliency maps. Identify top-3 saliency peaks. Check if primary CTA element falls within a top-3 peak.
**Score**: S_cta — fraction of CTA area covered by top-3 peaks. Target: >0.7.
**Additional check**: Banner blindness (Pernice/NN/g) — penalize CTAs styled like ads. Left-side bias: primary nav and content should be on left for desktop (~80% of fixation time per Fessenden).

### 3.13 Layout Pattern Correlation (Soft)

**Source**: F-pattern reading research, layer-cake pattern.
**Formula**: On text-heavy pages, generate saliency heatmap, then compute Pearson correlation against F-template kernel (heavy top, left-biased scan pattern).
**Threshold**: r > 0.4 for text-heavy pages. Not applicable for dashboard/app layouts.

### 3.14 LPIPS vs Golden (Soft)

**Source**: Learned Perceptual Image Patch Similarity.
**Formula**: Compute LPIPS distance between current screenshot and golden reference.
**Threshold**: <0.15.
**Note**: Only applicable when golden reference screenshots are provided.

### 3.15 AIM Metrics (Soft)

**Source**: AIM — Aalto Interface Metrics (Oulasvirta et al., UIST 2018). Twenty-one computational metrics.
**Key metrics extracted**: 
- **Feature Congestion** (Rosenholtz): measures visual clutter from color, orientation, and luminance contrast. Lower = cleaner.
- **Grid Quality**: measures how well elements align to an implicit grid. Higher = more organized.
- **Color-blindness simulation**: test under protanopia, deuteranopia, tritanopia — ensure sufficient distinction remains.
**Implementation**: Vendor the `aalto-ui/aim` Python library. Run via subprocess or port key metrics to Rust.

---

## 4. Design Token Extraction Pipeline

**Source**: Project Wallace, W3C DTCG 2025.10, Style Dictionary v4.

### 4.1 Three-Fidelity Extraction

**Tier 1: Qualitative** (vision LLM with structured output). Extract: voice, material, density, palette character, typography character, shadow character, radius character, imagery, notable patterns. Fast, cheap, low confidence.

**Tier 2: Mid-level statistical** (Wallace + custom analyzer). `@projectwallace/css-analyzer` (MIT) ingests raw CSS, emits near-DTCG JSON. Custom Rust analyzer walks `getComputedStyle()` over every visible element, harvests area-weighted color values, runs k-means in OKLCH (Rust `palette` crate), applies modal extraction on spacing/radius/font-size. Cross-check palette against `node-vibrant`/`colorthief` on screenshot. Auto-name primitives by hue family + lightness bucket. Promote to semantic aliases by role inference (dominant text color → `color.text.primary`; highest-area accent on `<button>` → `color.action.primary`).

**Tier 3: Reproducible W3C tokens** (assembled DTCG file). Output in 2025.10 format. File extension `.tokens` or `.tokens.json`, MIME `application/design-tokens+json`. Uses `$type`/`$value` with composite types. Aliases use `{dot.path}` syntax. Three-tier taxonomy: primitive, semantic, component.

**Screenshot-only extraction limitation**: Feasible for Tier 1 and partial Tier 2, NOT adequate for Tier 3. Pixel values lie about CSS values (subpixel AA produces hundreds of slightly-off RGB values). Spacing is ambiguous from raster. Composite tokens largely lost. Aliases unrecoverable. Tag screenshot-only outputs with `$extensions: { "ai.uigate.confidence": "low" }`.

### 4.2 Token Adherence Scoring Algorithm

```
For each visible element E:
  For each token category C ∈ {color, spacing, fontSize, radius, shadow, fontFamily}:
    actual_value = getComputedStyle(E)[property_for(C)]
    nearest_token = argmin_{t ∈ tokens[C]} distance(actual_value, t.value)
    d = distance(actual_value, nearest_token.value)
    hit = (d ≤ ε_C)  // ε varies by category
    area = bounding_rect(E).width × bounding_rect(E).height
    
    category_hits[C] += hit × area
    category_total[C] += area

Per-category score: S_C = category_hits[C] / category_total[C]

Overall = Σ(w_C × S_C) where weights = {color:0.30, spacing:0.25, fontSize:0.15, radius:0.10, shadow:0.10, fontFamily:0.10}

Violations list = elements where d > ε, sorted descending by area.
```

Distance functions per category:
- **Color**: ΔE2000 in OKLCH, ε=2.0
- **Spacing/radius/fontSize**: absolute px difference, ε=0.5
- **Shadow**: weighted L2 over (x_offset, y_offset, blur, spread, ΔE_color), ε=4.0
- **Font family**: exact match within weight bucket (100/200/.../900), ε=0 (exact)

Skip `transparent`, `currentColor`, `inherit` to avoid bias. Resolve `{...}` aliases to leaf values first.

### 4.3 Static Source Lint (Pre-Render Feedforward)

Before rendering, lint generated CSS/Tailwind for token usage. Vendor patterns from:
- `stylelint-plugin-carbon-tokens` (Carbon Design System)
- `@kong/design-tokens/stylelint-plugin` — checks type-correctness per property (`kui-color-text-primary` on `color`, not `background-color`)

This is cheaper than runtime adherence and catches literal hex values instantly.

---

## 5. Visual Regression: odiff + dssim

**Source**: odiff (MIT, Zig+SIMD, ~6.6× pixelmatch), dssim (Rust, AGPL-or-commercial).

### 5.1 Algorithm

```
1. Load current screenshot and golden reference.
2. Run odiff: emits {match, reason, diffCount, diffPercentage}.
3. If diffPercentage > odiff_threshold (default 0.1%):
   a. Run dssim: compute structural similarity score.
   b. If dssim > dssim_threshold (0.05): FAIL (real visual regression).
   c. If dssim < dssim_noise_floor (0.01): PASS (anti-aliasing noise only).
   d. Otherwise: WARN (ambiguous, flag for human review).
4. Output reg-suit-compatible JSON:
   { failedItems, newItems, deletedItems, passedItems, diffItems }
```

### 5.2 Why Not Applitools/Percy/Chromatic

SaaS-locked, ML-opaque, antithetical to the deterministic-verifier philosophy. Their Layout-mode and Ultrafast-Grid ideas are worth implementing in <500 LoC of Rust on top of CDP `DOM.getBoxModel` + `Accessibility.getFullAXTree`.

---

## 6. Core Web Vitals Collection

**Source**: web.dev Core Web Vitals thresholds (2024), Lighthouse CI.

### 6.1 Method

Run Lighthouse CI (LHCI) with median-of-5 runs. Pin Chrome version. Block third-party domains in test profile (single-run LCP/CLS variance is ±5% without this).

**Thresholds**:
- LCP ≤ 2500ms
- CLS ≤ 0.10
- TBT ≤ 200ms (lab proxy for INP)
- FCP ≤ 1800ms

**Advanced (post-MVP)**:
- Scripted INP p99 ≤ 200ms via `web-vitals` attribution build
- LoAF `blockingDuration` p95 ≤ 100ms
- `non-composited-animations.score == 1`

**Note**: INP in lab is synthetic. Real INP requires field RUM via CrUX or self-hosted `web-vitals`. Use TBT as lab proxy plus synthetic interaction script for the gate.

---

## 7. Reduced-Motion Compliance

**Source**: WCAG 2.3.3, Material Design 3 motion specs.

### 7.1 Method

Two Playwright runs:
1. `emulateMedia({ reducedMotion: 'reduce' })`
2. `emulateMedia({ reducedMotion: 'no-preference' })`

### 7.2 Checks

- **Differential test**: Reject if reduced-motion variant is visually identical to default (site ignores the user preference entirely — the most common failure).
- **Duration bands**: Under no-preference, animation durations must fall in Material 3 bands: 100–150ms (small), 200–300ms (medium), 400–500ms (large).
- **Easing allowlist**: `cubic-bezier(0.2, 0, 0, 1)` and M3 siblings.
- **Flash**: No flash >3Hz.
- **Autoplay**: No autoplay >5s.
- **Frame jank**: frame-jank-rate <2%.

---

## 8. Accessibility Collection

### 8.1 axe-core (Primary)

Configure maximally: all WCAG tags through `wcag22aa`, plus `wcag2aaa`, `wcag21aaa`, `best-practice`, `ACT`, `experimental`. Accept extra noise for higher recall. axe is the spine.

### 8.2 IBM Equal Access (Differential Coverage)

Layer on top because its independent rule engine has a different false-positive surface and adds 5–10% recall on real audits. Source: `achecker`.

### 8.3 Tab-Order Graph

Playwright Tab walk asserting:
- Completeness: all interactive elements reachable via Tab
- No traps: Tab always progresses forward
- DOM order ≈ visual order: Levenshtein distance between DOM tab sequence and visual layout sequence should be low

### 8.4 Focus-Visible Contrast

Screenshot diff between focused and unfocused states. Require ≥3:1 contrast change on the focus indicator.

### 8.5 EARL+JSON-LD Normalization

Normalize all tool outputs (axe, IBM, LHCI, custom) to EARL (Evaluation and Report Language) + JSON-LD. `UiGateReport { tier, passed, violations: Vec<Finding> }` with `Finding { rule_id, wcag_sc, impact, selector, snippet, fix_hint, source_tool }`. Enables deterministic diffing across runs.

---

## 9. Gestalt Operationalization

**Source**: Koch & Oulasvirta CHI 2016.

Computed as DOM/AST + screenshot probes:

- **Proximity**: DBSCAN clustering on element centroids. Silhouette coefficient measures grouping quality.
- **Similarity**: Cosine similarity on style vectors (background-color, font-size, font-family, border-radius, padding) per role-class. Elements with same role should have similar style.
- **Continuity**: Shared-axis alignment runs ≥3. Count how many elements share x or y coordinates.
- **Closure**: Bounded-region detection via `border | background-color | box-shadow`. Elements within visually bounded regions are perceived as groups.
- **Figure/Ground**: APCA on primary surfaces. Foreground elements must have sufficient Lc separation from background.

---

## 10. Nielsen's Heuristics as DOM Probes

**Source**: Nielsen's 10 usability heuristics, collapsed to Shneiderman's overlap.

Operationalized as automated checks:
- **Visibility of system status**: Loading indicator appears within 200ms of any async dispatch.
- **Error prevention**: Every required input has `aria-describedby`. Confirmation modal for destructive operations.
- **Recognition over recall**: Icon-only buttons require `aria-label` and tooltip.
- **Aesthetic and minimalist design**: Element count ≤30 per viewport (mobile), density 0.2–0.5.

Each probe emits 0/1 (pass/fail) plus continuous severity score.

---

## 11. Performance Budget

| Operation | Expected Duration |
|---|---|
| Dev server start | 2–15s |
| Browser launch | 1–3s |
| Computational metrics (15 metrics) | 2–5s |
| axe-core + IBM achecker | 2–7s |
| APCA per-element computation | 1–3s |
| AIM metrics (Feature Congestion, Grid Quality) | 1–3s |
| Saliency (DeepGaze + UMSI++) | 5–15s (GPU), 30–60s (CPU) |
| odiff visual regression | 0.1–0.5s |
| LHCI median-of-5 | 15–30s |
| Reduced-motion differential | 5–10s (two full runs) |
| Token extraction + adherence | 2–5s |
| Judge panel (3 models × 2 positions × 5 samples) | 30–90s |
| **Total (all features, desktop+mobile)** | **60–240s** |

MVP without saliency, reduced-motion, and full panel: ~30–60s per run.
