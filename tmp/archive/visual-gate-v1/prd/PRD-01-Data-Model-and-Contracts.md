# PRD-01: Complete Data Model, Contracts, and Configuration

**Prerequisites**: Read PRD-00 for architecture, research foundations, and citation index.

---

## 1. Overview

Every Rust struct, JSON schema, TOML config section, assertion variant, metric type, and artifact path for the UiGate system. All types defined inline with full field documentation. Types marked "Conceptual Rust" define the contract — they may need minor adjustments for the actual crate.

---

## 2. Task Definition Extension

### 2.1 UiTaskSpec

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTaskSpec {
    pub url: Option<String>,
    pub dev_server: Option<String>,
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_target_score")]
    pub target_score: f64,             // 0.0–10.0, default 8.5
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,             // default 3
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub hard_fail_on_console_error: bool,
    #[serde(default)]
    pub hard_fail_on_failed_request: bool,
    #[serde(default)]
    pub require_accessibility_snapshot: bool,
    #[serde(default)]
    pub visual_goal: Option<String>,
    #[serde(default)]
    pub viewports: Vec<UiViewport>,
    #[serde(default)]
    pub journeys: Vec<UiJourney>,
    #[serde(default)]
    pub artifact_retention: UiArtifactRetention,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub preflight_backend: Option<String>,
    /// Reference design token file for token adherence scoring.
    #[serde(default)]
    pub design_tokens_path: Option<PathBuf>,
    /// Reference URLs for Design2Code-style comparison.
    #[serde(default)]
    pub reference_screenshots: Vec<PathBuf>,
    /// Golden screenshot paths for odiff regression.
    #[serde(default)]
    pub golden_screenshots: Vec<GoldenScreenshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenScreenshot {
    pub viewport: String,
    pub journey_id: String,
    pub step: String,      // "before", "final", or named screenshot
    pub path: PathBuf,
}
```

### 2.2 UiViewport

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiViewport {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub device_scale_factor: Option<f64>,
    #[serde(default)]
    pub is_mobile: bool,
    #[serde(default)]
    pub has_touch: bool,
}
```

Defaults when none specified: desktop (1440×900), mobile (390×844, is_mobile=true, has_touch=true).

### 2.3 UiJourney

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiJourney {
    pub id: String,
    pub name: String,
    pub start_url: Option<String>,
    #[serde(default)]
    pub auth_state: Option<PathBuf>,
    #[serde(default)]
    pub steps: Vec<UiStep>,
    #[serde(default)]
    pub asserts: Vec<UiAssertion>,
    #[serde(default)]
    pub screenshot: UiScreenshotPolicy,
}
```

### 2.4 UiStep (13 Variants)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiStep {
    Goto { url: String },
    Click { role: Option<String>, name: Option<String>, text: Option<String>, selector: Option<String>, test_id: Option<String> },
    Fill { label: Option<String>, selector: Option<String>, test_id: Option<String>, value: String },
    Press { key: String },
    Select { label: Option<String>, selector: Option<String>, value: String },
    Hover { selector: Option<String>, text: Option<String> },
    WaitForText { text: String, timeout_ms: Option<u64> },
    WaitForSelector { selector: String, timeout_ms: Option<u64> },
    WaitForUrl { pattern: String, timeout_ms: Option<u64> },
    Evaluate { script: String },
    Screenshot { name: Option<String>, full_page: Option<bool> },
    Scroll { selector: Option<String>, x: Option<i32>, y: Option<i32> },
    Wait { ms: u64 },
}
```

Locator resolution order: (1) role+name, (2) label, (3) test_id, (4) text, (5) CSS selector.

### 2.5 UiAssertion (22 Variants)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiAssertion {
    TextVisible { text: String },
    TextNotVisible { text: String },
    UrlMatches { pattern: String },
    LocatorVisible { selector: String },
    RoleVisible { role: String, name: String },
    NoConsoleErrors,
    NoPageErrors,
    NoFailedRequests { allow: Option<Vec<String>> },
    NoHorizontalOverflow,
    NoOverlappingText,
    NoClippedText,
    MinContrast { ratio: f64 },
    ApcaContrast { min_body_lc: f64, min_large_lc: f64 },
    AccessibilityViolationsBelow { max_critical: u32, max_serious: u32 },
    VisualScoreAtLeast { score: f64 },
    TokenAdherenceAtLeast { score: f64 },
    ElementDensityBelow { max_elements: u32 },
    CoreWebVitals { max_lcp_ms: u64, max_cls: f64, max_tbt_ms: u64 },
    NoReducedMotionViolation,
    VisualRegressionBelow { max_diff_percentage: f64 },
    CustomEvaluate { name: String, script: String, expect_json: serde_json::Value },
    ElementCount { selector: String, min: Option<u32>, max: Option<u32> },
}
```

### 2.6 Supporting Enums

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiScreenshotPolicy { Final, #[default] BeforeAndAfter, EveryStep, Manual }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiArtifactRetention { Minimal, #[default] Debug, Full }
```

---

## 3. Browser Runner Contract

### 3.1 BrowserRunSpec (spec.json)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRunSpec {
    pub schema_version: u32,     // 1
    pub run_id: String,
    pub plan_id: String,
    pub task_id: String,
    pub attempt: u32,
    pub backend: String,
    pub base_url: String,
    pub output_dir: String,
    pub timeout_ms: u64,
    pub save_trace: bool,
    pub save_har: bool,
    pub save_video: bool,
    pub viewports: Vec<UiViewport>,
    pub journeys: Vec<UiJourney>,
    pub security: RunnerSecurity,
    pub metrics: MetricsConfig,
    pub golden_screenshots: Vec<GoldenScreenshot>,
    pub design_tokens_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerSecurity {
    pub allow_external_urls: bool,
    pub allow_evaluate_steps: bool,
    pub redact_headers: Vec<String>,
    pub redact_text_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub compute_layout_metrics: bool,         // AIM, grid adherence, density
    pub compute_saliency: bool,               // DeepGaze IIE + UMSI++
    pub compute_colorfulness: bool,           // Hasler-Süsstrunk
    pub compute_apca: bool,                   // APCA per text element
    pub compute_token_adherence: bool,        // design token adherence score
    pub compute_core_web_vitals: bool,        // LHCI
    pub compute_visual_regression: bool,      // odiff + dssim
    pub compute_reduced_motion: bool,         // reduced-motion differential test
    pub lighthouse_runs: u32,                 // median-of-N, default 5
}
```

### 3.2 BrowserRunResult (result.json)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserRunResult {
    pub schema_version: u32,
    pub run_id: String,
    pub plan_id: String,
    pub task_id: String,
    pub attempt: u32,
    pub backend: String,
    pub started_at: String,
    pub duration_ms: u64,
    pub passed: bool,
    pub summary: String,
    pub failure_classes: Vec<String>,
    pub viewports: Vec<ViewportResult>,
    pub artifacts: ArtifactPaths,
    pub computational_metrics: ComputationalMetrics,
    pub hard_gate_results: HardGateResults,
}
```

### 3.3 ViewportResult and JourneyResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportResult {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub journeys: Vec<JourneyResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyResult {
    pub id: String,
    pub passed: bool,
    pub final_url: String,
    pub screenshots: Vec<String>,
    pub steps: Vec<StepResult>,
    pub assertions: Vec<AssertionResult>,
    pub console: Vec<ConsoleMessage>,
    pub page_errors: Vec<PageError>,
    pub requests: Vec<NetworkRequest>,
    pub layout: LayoutMetrics,
    pub accessibility: AccessibilityResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult { pub action: String, pub success: bool, pub duration_ms: u64, pub error: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult { pub name: String, pub passed: bool, pub severity: String, pub detail: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage { pub r#type: String, pub text: String, pub location: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageError { pub message: String, pub stack: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String, pub method: String, pub status: Option<u16>,
    pub failed: bool, pub failure_text: Option<String>,
    pub duration_ms: Option<u64>, pub response_size: Option<u64>,
}
```

### 3.4 LayoutMetrics

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutMetrics {
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub document_width: u32,
    pub document_height: u32,
    pub horizontal_overflow: bool,
    pub clipped_text_candidates: Vec<ClippedElement>,
    pub overlapping_text_candidates: Vec<OverlapPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippedElement {
    pub text: String,
    pub rect: ElementRect,
    pub scroll_width: u32,
    pub client_width: u32,
    pub scroll_height: u32,
    pub client_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapPair { pub element_a: String, pub element_b: String, pub overlap_area: f64 }
```

### 3.5 AccessibilityResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityResult {
    pub snapshot_path: Option<String>,
    pub violations_path: Option<String>,
    /// axe-core violations by impact level
    pub axe_critical: u32,
    pub axe_serious: u32,
    pub axe_moderate: u32,
    pub axe_minor: u32,
    /// IBM Equal Access violations (separate rule engine, adds 5–10% recall)
    pub ibm_violations: u32,
    /// Tab-order completeness: all interactive elements reachable, no traps
    pub tab_order_complete: bool,
    pub tab_order_traps: Vec<String>,
    /// DOM order vs visual order Levenshtein distance (lower = better)
    pub dom_visual_order_distance: Option<f64>,
    /// Focus-visible contrast change ≥ 3:1
    pub focus_visible_contrast_ok: bool,
}
```

---

## 4. Computational Metrics Types

These 15 metrics are computed on every render, before any LLM judge runs. They are the deterministic floor that ensures the visual evaluator only runs on structurally sound UIs. Based on AIM (Oulasvirta et al., UIST 2018), Koch & Oulasvirta (CHI 2016), Reinecke et al. (CHI 2013), and Miniukovich (AVI 2014).

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationalMetrics {
    // ── Hard gate metrics ──
    /// Fraction of CSS values resolving to design tokens. Target: ≥0.9.
    pub token_coverage: f64,
    /// WCAG 2.1 contrast pass rate. Target: 1.00 (hard gate).
    pub wcag_contrast_pass_rate: f64,
    /// Fraction of body text with APCA Lc ≥ 60. Target: ≥0.95.
    /// Uses Myndex/apca-w3 algorithm, polarity-corrected on composited pixel colors.
    pub apca_body_pass_rate: f64,
    /// Fraction of large text with APCA Lc ≥ 45. Target: ≥0.95.
    pub apca_large_pass_rate: f64,
    /// Per-text-element APCA details for violations.
    pub apca_violations: Vec<ApcaViolation>,
    /// Fraction of spacing values divisible by grid base (4 or 8). Target: ≥0.95.
    pub grid_adherence: f64,
    /// 1 − unique_x_coords/element_count along major axis. Target: ≥0.7.
    pub alignment_score: f64,
    /// Fraction of font sizes fitting one modular ratio. Target: ≥0.9.
    /// Ratios tested: 1.125, 1.2, 1.25, 1.333, 1.414, 1.5, 1.618.
    pub modular_scale_conformity: f64,
    /// Max ΔE2000 from any pixel to nearest palette token. Target: ≤2.3 for >0.95 of pixels.
    /// k-means k=5–8 in OKLCH, excluding images.
    pub color_palette_compactness: f64,

    // ── Soft gate metrics (contribute to Pareto frontier) ──
    /// Hasler-Süsstrunk colorfulness M. Optimal band: [15, 35].
    /// Per Reinecke et al. CHI 2013 first-impression study.
    pub colorfulness: f64,
    /// Element count in viewport. Thresholds: ≤30 mobile, ≤50 desktop (Miniukovich AVI 2014).
    pub element_density: u32,
    /// Fraction of viewport area that is text vs whitespace. Optimal: [0.15, 0.40].
    pub text_whitespace_ratio: f64,
    /// Visual balance: |W_L − W_R| / max(W_L, W_R). Target: <0.15 unless explicit asymmetry.
    pub visual_balance: f64,
    /// Primary CTA in top-3 saliency peaks. DeepGaze IIE + UMSI++ ensemble.
    pub saliency_on_cta: f64,
    /// F-pattern or layer-cake template correlation. Target: r > 0.4 on text-heavy pages.
    pub layout_pattern_correlation: f64,
    /// LPIPS vs golden reference screenshot. Target: <0.15.
    pub lpips_vs_golden: Option<f64>,

    // ── AIM metrics (Oulasvirta et al. UIST 2018) ──
    pub aim_feature_congestion: f64,
    pub aim_grid_quality: f64,

    // ── Visual regression ──
    /// odiff diff percentage against golden. 0.0 = identical.
    pub odiff_diff_percentage: Option<f64>,
    /// dssim structural similarity. <0.01 = anti-aliasing noise only.
    pub dssim_score: Option<f64>,

    // ── Design token adherence (area-weighted) ──
    pub token_adherence: Option<TokenAdherenceResult>,

    // ── Core Web Vitals (LHCI median-of-5) ──
    pub core_web_vitals: Option<CoreWebVitalsResult>,

    // ── Reduced motion compliance ──
    pub reduced_motion: Option<ReducedMotionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApcaViolation {
    pub selector: String,
    pub text_preview: String,
    pub font_size_px: f64,
    pub font_weight: u32,
    pub fg_color: String,
    pub bg_color: String,
    pub lc_value: f64,
    pub required_lc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAdherenceResult {
    /// Per-category scores (area-weighted Σ(hit·area) / Σ(area))
    pub color: f64,           // weight 0.30
    pub spacing: f64,         // weight 0.25
    pub font_size: f64,       // weight 0.15
    pub border_radius: f64,   // weight 0.10
    pub shadow: f64,          // weight 0.10
    pub font_family: f64,     // weight 0.10
    /// Weighted overall score.
    pub overall: f64,
    /// Top violations sorted by area (actionable diff for agent).
    pub violations: Vec<TokenViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenViolation {
    pub category: String,
    pub selector: String,
    pub actual_value: String,
    pub nearest_token: String,
    pub distance: f64,        // ΔE2000 for color, px for dimensions
    pub element_area: f64,    // importance weight
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreWebVitalsResult {
    pub lcp_ms: f64,          // Largest Contentful Paint. Target: ≤2500
    pub cls: f64,             // Cumulative Layout Shift. Target: ≤0.10
    pub tbt_ms: f64,          // Total Blocking Time (lab proxy for INP). Target: ≤200
    pub fcp_ms: f64,          // First Contentful Paint. Target: ≤1800
    pub inp_p99_ms: Option<f64>, // Scripted INP via web-vitals attribution build. Target: ≤200
    pub loaf_blocking_p95_ms: Option<f64>, // LoAF blockingDuration. Target: ≤100
    pub non_composited_animations_clean: bool,
    pub runs: u32,            // number of runs (median taken)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducedMotionResult {
    /// Did reduced-motion run differ from no-preference run?
    pub has_motion_difference: bool,
    /// Animations violating Material 3 duration bands.
    pub duration_violations: Vec<MotionViolation>,
    /// Animations using non-allowlisted easing.
    pub easing_violations: Vec<MotionViolation>,
    /// Flash frequency > 3Hz detected.
    pub flash_violations: u32,
    /// Autoplay > 5s detected.
    pub autoplay_violations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionViolation {
    pub selector: String,
    pub property: String,
    pub actual_duration_ms: Option<u64>,
    pub actual_easing: Option<String>,
    pub expected_range: String,
}
```

---

## 5. Hard Gate Results

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardGateResults {
    /// Tier 1: Structural integrity
    pub tier1_structural: TierResult,
    /// Tier 2: Full WCAG 2.2 AA conformance
    pub tier2_wcag: TierResult,
    /// Tier 3: Core Web Vitals
    pub tier3_performance: TierResult,
    /// Tier 4: APCA + Motion
    pub tier4_perceptual: TierResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierResult {
    pub tier: u8,
    pub name: String,
    pub passed: bool,
    pub violations: Vec<Finding>,
}

/// EARL+JSON-LD compatible finding for cross-tool aggregation.
/// Normalized from axe-core, IBM Equal Access, LHCI, and custom checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub wcag_sc: Option<String>,    // e.g., "1.4.3"
    pub impact: String,             // "critical", "serious", "moderate", "minor"
    pub selector: String,
    pub snippet: String,
    pub fix_hint: String,
    pub source_tool: String,        // "axe-core", "ibm-equal-access", "lhci", "apca", "custom"
}
```

---

## 6. Visual Evaluator Contract

### 6.1 Judge Panel Input

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgePanelInput {
    pub task_id: String,
    pub task_title: String,
    pub task_description: Option<String>,
    pub acceptance_criteria: Vec<String>,
    pub visual_goal: Option<String>,
    pub viewport: UiViewport,
    pub journey_id: String,
    /// Current candidate screenshot
    pub candidate_screenshot: PathBuf,
    /// Previous best release (fixed anchor for BT comparison)
    pub anchor_screenshot: Option<PathBuf>,
    pub hard_gate_summary: String,
    pub computational_metrics_summary: String,
    pub prior_attempts: Vec<PriorAttemptSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorAttemptSummary {
    pub attempt: u32,
    pub visual_score: Option<f64>,
    pub top_findings: Vec<String>,
}
```

### 6.2 Pairwise Judge Result (per judge)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairwiseJudgeResult {
    pub judge_model: String,         // "claude-opus-4-6", "llava-critic-72b", "prometheus-vision"
    pub judge_family: String,        // "anthropic", "llava", "kaist" — must be disjoint from generator
    pub preferred: String,           // "A" (candidate) or "B" (anchor) or "tie"
    pub position_order: String,      // "AB" or "BA" — always run both
    pub position_consistent: bool,   // did swapping change the verdict?
    pub rubric_scores: RubricScores,
    pub findings: Vec<VisualFinding>,
    pub raw_response: String,
}
```

### 6.3 Aggregated Panel Result

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelResult {
    pub schema_version: u32,
    /// Bradley-Terry MLE score (mapped to Elo scale via × 400/log(10))
    pub bt_score: f64,
    /// Bootstrap 95% CI on BT score
    pub bt_ci_lower: f64,
    pub bt_ci_upper: f64,
    /// Trimmed mean of rubric scores across panel (10–20% trim)
    pub rubric_scores: RubricScores,
    /// Whether candidate was preferred over anchor by panel majority
    pub preferred_over_anchor: bool,
    /// Number of judges, how many preferred candidate
    pub judge_count: u32,
    pub judges_preferring_candidate: u32,
    /// Position-inconsistent verdicts discarded
    pub position_inconsistent_count: u32,
    /// Individual judge results
    pub judges: Vec<PairwiseJudgeResult>,
    /// Merged findings from all judges, deduplicated
    pub findings: Vec<VisualFinding>,
    /// Overall pass/fail against threshold
    pub passed: bool,
    pub threshold: f64,
}
```

### 6.4 Rubric and Finding Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricScores {
    pub task_completion: f64,           // weight 0.25
    pub layout_integrity: f64,         // weight 0.20
    pub responsive_quality: f64,       // weight 0.15
    pub interaction_clarity: f64,      // weight 0.10
    pub visual_polish: f64,            // weight 0.10
    pub design_system_fit: f64,        // weight 0.10
    pub accessibility_affordance: f64, // weight 0.10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualFinding {
    pub severity: String,
    pub viewport: String,
    pub journey_id: String,
    pub screenshot: String,
    pub area: String,
    pub problem: String,
    pub evidence: String,
    pub suggested_fix: String,
    /// Bounding box grounding (UICrit-style)
    pub bbox: Option<BoundingBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64, pub y: f64, pub width: f64, pub height: f64,
}
```

---

## 7. Failure Taxonomy

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UiFailureClass {
    AppUnavailable, DevServerFailed, BrowserLaunchFailed, NavigationFailed,
    LocatorNotFound, ActionTimeout, AssertionFailed, ConsoleError, PageError,
    FailedRequest, AuthRequired, HydrationError, LayoutOverflow, TextClipped,
    TextOverlap, A11yCritical, A11ySerious, ApcaViolation, CoreWebVitalsViolation,
    ReducedMotionViolation, TokenAdherenceLow, VisualRegressionHigh,
    VisualScoreLow, VisualRegression, ElementDensityHigh,
    RunnerInfrastructure,
}
```

---

## 8. Design Token Schema

Following W3C Design Tokens Community Group 2025.10 spec. Three-tier taxonomy.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTokenFile {
    /// Primitive tokens: raw values
    pub primitives: BTreeMap<String, DesignToken>,
    /// Semantic tokens: reference primitives via {dot.path} aliases
    pub semantic: BTreeMap<String, DesignToken>,
    /// Component tokens: reference semantic tokens
    pub component: BTreeMap<String, DesignToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignToken {
    #[serde(rename = "$type")]
    pub token_type: String,    // "color", "dimension", "fontFamily", etc.
    #[serde(rename = "$value")]
    pub value: serde_json::Value,
    #[serde(rename = "$description", default)]
    pub description: Option<String>,
    #[serde(rename = "$extensions", default)]
    pub extensions: BTreeMap<String, serde_json::Value>,
}
```

Token adherence algorithm (from compass artifact):

1. Render UI in headless Chrome.
2. Walk every visible element via `getComputedStyle()`.
3. For each token category, compute per-occurrence minimum perceptual distance to nearest token in reference set:
   - Color: ΔE2000 in OKLCH, threshold ε=2.0
   - Dimension (spacing, radius): px difference, threshold ε=0.5
   - Shadow: weighted L2 over (offset, blur, spread, ΔE_color), threshold ε=4.0
   - Font family: exact match (weight bucket)
   - Font size: px difference, threshold ε=0.5
4. Each occurrence is area-weighted (large hero >> tiny footnote).
5. Per-category score = Σ(hit × area) / Σ(area).
6. Overall = weighted sum: color 0.30, spacing 0.25, fontSize 0.15, radius 0.10, shadow 0.10, fontFamily 0.10.
7. Skip `transparent`/`currentColor` to avoid bias.
8. Output violations list sorted by area — the actionable diff fed to the agent.

---

## 9. Artifact Layout

```
.roko/ui-runs/
  {task-id}/
    001/
      spec.json
      result.json
      panel-result.json            # Aggregated judge panel result
      feedback.md
      trace.zip
      network.har
      console.json
      requests.json
      page-errors.json
      metrics.json                 # ComputationalMetrics
      hard-gates.json              # HardGateResults
      tokens-extracted.json        # Extracted design tokens (W3C 2025.10 format)
      token-adherence.json         # TokenAdherenceResult
      regression/                  # Visual regression data
        odiff-desktop-final.png    # Diff image
        odiff-desktop-final.json   # {match, reason, diffCount, diffPercentage}
      desktop/
        create-project/
          before.png
          final.png
          dom.html
          text.txt
          a11y.json
          axe.json
          ibm-achecker.json        # IBM Equal Access results
          layout.json
          apca.json                # Per-element APCA results
          saliency.json            # DeepGaze + UMSI++ heatmap data
          aim.json                 # AIM metrics
      mobile/
        ...
      eval/
        judges/
          claude-opus-AB.json      # Judge 1, A=candidate B=anchor
          claude-opus-BA.json      # Judge 1, B=candidate A=anchor (swap)
          llava-critic-AB.json
          llava-critic-BA.json
          prometheus-AB.json
          prometheus-BA.json
        panel-aggregate.json
    002/
      ...
    verdict.json
    summary.json
    preferences.jsonl              # Pairwise preference triples for BT training
```

### 9.1 UiRunSummary

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiRunSummary {
    pub task_id: String,
    pub attempt_count: u32,
    pub passed: bool,
    pub attempts: Vec<UiAttemptRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAttemptRecord {
    pub attempt: u32,
    pub passed: bool,
    pub failure_tier: Option<u8>,
    pub failure_classes: Vec<String>,
    pub visual_score: Option<f64>,
    pub bt_score: Option<f64>,
    pub token_adherence: Option<f64>,
    pub hard_failure_count: u32,
    pub soft_finding_count: u32,
    pub computational_metrics_passed: u32,
    pub computational_metrics_total: u32,
    pub duration_ms: u64,
    pub timestamp: String,
}
```

---

## 10. Global Configuration

```toml
[gates.ui]
enabled = true
default_backend = "playwright-chromium"
target_score = 8.5
max_attempts = 3
timeout_ms = 120000
fail_on_console_error = true
fail_on_page_error = true
fail_on_failed_request = true
save_trace = true
save_har = true
vision_model = "claude-opus-4-6"
artifact_retention = "debug"

[gates.ui.judge_panel]
# Disjoint-family panel (PoLL: Verga et al. 2024)
# Never use same model family as generator
models = ["claude-opus-4-6", "llava-critic-72b", "prometheus-vision"]
families = ["anthropic", "llava", "kaist"]
aggregation = "trimmed_mean"
trim_fraction = 0.15
# Always run (A,B) and (B,A) and discard inconsistent (Wang et al. ACL 2024)
require_position_swap = true
samples_per_judgment = 5   # N=5 at T=0, captures ~80% of N=20 variance (G-Eval)

[gates.ui.metrics]
compute_layout_metrics = true
compute_saliency = false    # expensive, enable post-MVP
compute_colorfulness = true
compute_apca = true
compute_token_adherence = true
compute_core_web_vitals = true
compute_visual_regression = true
compute_reduced_motion = false  # enable post-MVP
lighthouse_runs = 5

[gates.ui.metrics.apca]
min_body_lc = 60.0          # Lc 60 minimum body text
preferred_body_lc = 75.0    # Lc 75 preferred
min_large_lc = 45.0         # Lc 45 large text
min_floor_lc = 30.0         # Lc 30 absolute floor
min_nontext_lc = 15.0       # Lc 15 non-text

[gates.ui.metrics.density]
max_mobile = 30
max_desktop = 50

[gates.ui.metrics.token_adherence]
color_weight = 0.30
spacing_weight = 0.25
font_size_weight = 0.15
radius_weight = 0.10
shadow_weight = 0.10
font_family_weight = 0.10
color_epsilon = 2.0         # ΔE2000
dimension_epsilon = 0.5     # px
shadow_epsilon = 4.0        # weighted L2

[gates.ui.metrics.regression]
odiff_threshold = 0.1       # percentage
dssim_threshold = 0.05
dssim_noise_floor = 0.01    # below this = anti-aliasing noise, pass

[gates.ui.canary]
# Frozen human-rated canary set (Krippendorff α ≥ 0.8)
canary_set_path = ".roko/learn/ui-canary-set.json"
canary_size = 200
min_alpha = 0.8
reeval_frequency = "weekly"

[gates.ui.goodhart]
# Anti-Goodhart safeguards
eval_rotation_frequency = "quarterly"
held_out_fraction = 0.20
require_orthogonal_improvements = 3
krakovna_red_team_path = ".roko/learn/specification-gaming-prompts.json"

[[gates.ui.viewport]]
name = "desktop"
width = 1440
height = 900

[[gates.ui.viewport]]
name = "mobile"
width = 390
height = 844
is_mobile = true
has_touch = true

[gates.ui.security]
allow_external_urls = false
allow_evaluate_steps = false
redact_headers = ["authorization", "cookie", "x-api-key"]
redact_text_patterns = ["sk-[A-Za-z0-9_-]+", "Bearer [A-Za-z0-9._-]+"]
```

---

## 11. Validation Rules

1. At least one journey defined.
2. Each journey needs ≥1 step or start_url + ≥1 assertion.
3. `target_score` in 0.0..=10.0.
4. Viewport dimensions positive, ≤4096.
5. Viewport and journey names/IDs unique within task.
6. Evaluate steps require security opt-in.
7. External URLs require security opt-in.
8. `artifact_retention` ∈ {minimal, debug, full}.
9. Judge panel models must be from disjoint families.
10. Judge panel must not include the generator model's family.
11. Golden screenshots must reference valid viewport/journey/step combinations.
12. Design tokens path must point to a valid .tokens or .tokens.json file.
