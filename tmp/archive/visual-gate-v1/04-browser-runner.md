# Part IV: Browser runner specification

This document is the complete specification for `tools/roko-ui-runner.mjs` — the standalone Node.js script that executes browser journeys via Playwright and produces structured evidence. The runner is invoked by `UiGate` (Rust) as a subprocess.

The runner does NOT call any LLM. It only runs browser automation and writes evidence. Visual evaluation is separate.

---

## 30. Overview

The runner accepts a `BrowserRunSpec` (JSON), launches Chromium via Playwright, executes UI journeys, captures screenshots and browser evidence, and writes a `BrowserRunResult` (JSON) plus artifact files.

```
Input:  spec.json (BrowserRunSpec)     ← written by Rust, read by Node
Output: result.json (BrowserRunResult) ← written by Node, read by Rust
        screenshots/*.png              ← written by Node
        console.json                   ← written by Node
        requests.json                  ← written by Node
        page-errors.json               ← written by Node
        trace.zip                      ← written by Playwright
        network.har                    ← written by Playwright
```

---

## 31. Invocation

Two modes:

### File mode (preferred)

```bash
node tools/roko-ui-runner.mjs --spec .roko/ui-runs/F3/001/spec.json
```

The runner reads the spec from the file path.

### Stdin mode

```bash
echo '{ ... BrowserRunSpec JSON ... }' | node tools/roko-ui-runner.mjs
```

The runner reads one JSON line from stdin.

### Self-test mode

```bash
node tools/roko-ui-runner.mjs --self-test
```

Launches Chromium, navigates to `data:text/html,<h1>ok</h1>`, verifies the `<h1>` contains "ok", prints `{"self_test": "pass"}` to stdout, exits 0. Confirms Playwright is installed and Chromium launches.

### Exit codes

- **0**: Runner completed. UI failures are reported in `result.json`, not via exit code.
- **1**: Unrecoverable infrastructure error (missing Playwright, invalid spec JSON, browser launch failure, filesystem error).

---

## 32. Implementation outline

```javascript
#!/usr/bin/env node
import { chromium } from 'playwright';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { randomUUID } from 'crypto';

// ---------------------------------------------------------------
// 1. Parse arguments and read spec
// ---------------------------------------------------------------
const args = process.argv.slice(2);

if (args.includes('--self-test')) {
  await runSelfTest();
  process.exit(0);
}

let spec;
const specFlag = args.indexOf('--spec');
if (specFlag !== -1 && args[specFlag + 1]) {
  spec = JSON.parse(readFileSync(args[specFlag + 1], 'utf-8'));
} else {
  // Read from stdin
  spec = JSON.parse(readFileSync('/dev/stdin', 'utf-8').trim());
}

// ---------------------------------------------------------------
// 2. Create output directory
// ---------------------------------------------------------------
mkdirSync(spec.output_dir, { recursive: true });

// Write spec to output for traceability
writeFileSync(
  join(spec.output_dir, 'spec.json'),
  JSON.stringify(spec, null, 2)
);

// ---------------------------------------------------------------
// 3. Launch browser
// ---------------------------------------------------------------
const browser = await chromium.launch({ headless: true });

const result = {
  schema_version: 1,
  run_id: spec.run_id || randomUUID(),
  plan_id: spec.plan_id,
  task_id: spec.task_id,
  attempt: spec.attempt,
  backend: spec.backend || 'playwright-chromium',
  started_at: new Date().toISOString(),
  duration_ms: 0,
  passed: true,
  summary: '',
  failure_classes: [],
  viewports: [],
  artifacts: { trace: null, har: null, video: null, screenshots: [] },
};

const startTime = Date.now();

try {
  // ---------------------------------------------------------------
  // 4. For each viewport, create context and run journeys
  // ---------------------------------------------------------------
  for (const viewport of spec.viewports) {
    const vpResult = await runViewport(browser, spec, viewport);
    result.viewports.push(vpResult);

    // Aggregate pass/fail
    for (const jr of vpResult.journeys) {
      if (!jr.passed) {
        result.passed = false;
      }
    }
  }

  // Build summary and failure classes
  result.summary = buildSummary(result);
  result.failure_classes = collectFailureClasses(result);

} catch (error) {
  result.passed = false;
  result.summary = `Runner error: ${error.message}`;
  result.failure_classes = ['runner_infrastructure'];
} finally {
  await browser.close();
  result.duration_ms = Date.now() - startTime;
}

// ---------------------------------------------------------------
// 5. Write result
// ---------------------------------------------------------------
writeFileSync(
  join(spec.output_dir, 'result.json'),
  JSON.stringify(result, null, 2)
);

// Also echo to stdout for the Rust caller
console.log(JSON.stringify(result));

process.exit(0);
```

---

## 33. Viewport execution

```javascript
async function runViewport(browser, spec, viewport) {
  const contextOptions = {
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: viewport.device_scale_factor || 1,
    isMobile: viewport.is_mobile || false,
    hasTouch: viewport.has_touch || false,
  };

  // Configure tracing and HAR
  if (spec.save_har) {
    contextOptions.recordHar = {
      path: join(spec.output_dir, 'network.har'),
      mode: 'full',
    };
  }

  if (spec.save_video) {
    contextOptions.recordVideo = {
      dir: join(spec.output_dir, 'video'),
      size: { width: viewport.width, height: viewport.height },
    };
  }

  const context = await browser.newContext(contextOptions);

  // Network policy: block non-allowed hosts
  if (spec.security) {
    const allowedHosts = new Set([
      'localhost', '127.0.0.1', '0.0.0.0',
      ...(spec.security.network_allow || []),
    ]);

    await context.route('**/*', (route) => {
      try {
        const url = new URL(route.request().url());
        if (allowedHosts.has(url.hostname)) {
          route.continue();
        } else if (url.protocol === 'data:') {
          route.continue();
        } else {
          route.abort('blockedbyclient');
        }
      } catch {
        route.continue();
      }
    });
  }

  // Start tracing
  if (spec.save_trace) {
    await context.tracing.start({
      screenshots: true,
      snapshots: true,
      sources: true,
    });
  }

  const page = await context.newPage();

  // ---------------------------------------------------------------
  // Set up event capture
  // ---------------------------------------------------------------
  const consoleMessages = [];
  const pageErrors = [];
  const networkRequests = [];

  page.on('console', (msg) => {
    if (consoleMessages.length < 200) {
      consoleMessages.push({
        level: msg.type(),
        text: msg.text(),
        url: msg.location()?.url || null,
        line_number: msg.location()?.lineNumber || null,
      });
    }
  });

  page.on('pageerror', (error) => {
    pageErrors.push({
      message: error.message,
      stack: error.stack || '',
    });
  });

  page.on('response', (response) => {
    if (networkRequests.length < 200) {
      networkRequests.push({
        url: response.url(),
        method: response.request().method(),
        status: response.status(),
        failed: false,
        failure_text: null,
        response_size: null,
      });
    }
  });

  page.on('requestfailed', (request) => {
    if (networkRequests.length < 200) {
      networkRequests.push({
        url: request.url(),
        method: request.method(),
        status: null,
        failed: true,
        failure_text: request.failure()?.errorText || null,
        response_size: null,
      });
    }
  });

  // ---------------------------------------------------------------
  // Run each journey
  // ---------------------------------------------------------------
  const journeyResults = [];

  for (const journey of spec.journeys) {
    const jr = await runJourney(page, spec, viewport, journey, {
      consoleMessages, pageErrors, networkRequests,
    });
    journeyResults.push(jr);
  }

  // ---------------------------------------------------------------
  // Stop tracing and close context
  // ---------------------------------------------------------------
  if (spec.save_trace) {
    await context.tracing.stop({
      path: join(spec.output_dir, 'trace.zip'),
    });
    // result.artifacts.trace will be set by caller
  }

  // Write evidence files
  const vpDir = join(spec.output_dir, viewport.name);
  mkdirSync(vpDir, { recursive: true });

  writeFileSync(
    join(spec.output_dir, 'console.json'),
    JSON.stringify(consoleMessages, null, 2)
  );

  writeFileSync(
    join(spec.output_dir, 'requests.json'),
    JSON.stringify(networkRequests, null, 2)
  );

  writeFileSync(
    join(spec.output_dir, 'page-errors.json'),
    JSON.stringify(pageErrors, null, 2)
  );

  await context.close();

  return {
    name: viewport.name,
    width: viewport.width,
    height: viewport.height,
    journeys: journeyResults,
  };
}
```

---

## 34. Journey execution

```javascript
async function runJourney(page, spec, viewport, journey, events) {
  const journeyDir = join(spec.output_dir, viewport.name, journey.id);
  mkdirSync(journeyDir, { recursive: true });

  const result = {
    id: journey.id,
    passed: true,
    final_url: '',
    screenshots: [],
    steps: [],
    assertions: [],
    console: [], // Will be populated from events after journey
    page_errors: [],
    requests: [],
    layout: null,
    accessibility: null,
  };

  // Navigate to start URL if specified
  if (journey.start_url) {
    try {
      await page.goto(journey.start_url, { waitUntil: 'networkidle' });
    } catch (error) {
      result.passed = false;
      result.steps.push({
        index: 0,
        action: 'goto',
        success: false,
        duration_ms: 0,
        error: `Navigation failed: ${error.message}`,
      });
      result.final_url = page.url();
      return result;
    }
  }

  // Execute steps
  for (let i = 0; i < journey.steps.length; i++) {
    const step = journey.steps[i];
    const stepStart = Date.now();

    try {
      await executeStep(page, step, spec, journeyDir, result);
      result.steps.push({
        index: i,
        action: step.action,
        success: true,
        duration_ms: Date.now() - stepStart,
        error: null,
      });
    } catch (error) {
      result.passed = false;
      result.steps.push({
        index: i,
        action: step.action,
        success: false,
        duration_ms: Date.now() - stepStart,
        error: error.message,
      });
      // Short-circuit: stop executing further steps on failure
      break;
    }
  }

  result.final_url = page.url();

  // Collect layout metrics (even on partial failure)
  result.layout = await collectLayout(page);

  // Collect accessibility snapshot (if configured)
  if (spec.require_accessibility_snapshot !== false) {
    result.accessibility = await collectAccessibility(page, journeyDir);
  }

  // Evaluate assertions
  for (const assertion of [...(journey.asserts || []), ...(spec.global_assertions || [])]) {
    const ar = await evaluateAssertion(page, assertion, events, result);
    result.assertions.push(ar);
    if (!ar.passed && ar.severity === 'hard') {
      result.passed = false;
    }
  }

  // Snapshot console/network events for this journey
  result.console = [...events.consoleMessages];
  result.page_errors = [...events.pageErrors];
  result.requests = [...events.networkRequests];

  // Take final screenshot if not already taken via a Screenshot step
  if (result.screenshots.length === 0 || journey.screenshot !== 'manual') {
    const finalPath = join(journeyDir, 'final.png');
    await page.screenshot({ path: finalPath, fullPage: false });
    const relPath = `${viewport.name}/${journey.id}/final.png`;
    result.screenshots.push(relPath);
  }

  return result;
}
```

---

## 35. Step execution

```javascript
async function executeStep(page, step, spec, journeyDir, result) {
  switch (step.action) {
    case 'goto':
      await page.goto(step.url, { waitUntil: 'networkidle' });
      break;

    case 'click': {
      const locator = resolveLocator(page, step);
      await locator.click();
      break;
    }

    case 'fill': {
      const locator = resolveLocator(page, step);
      await locator.fill(step.value);
      break;
    }

    case 'press':
      await page.keyboard.press(step.key);
      break;

    case 'select': {
      const locator = resolveLocator(page, step);
      await locator.selectOption(step.value);
      break;
    }

    case 'hover': {
      const locator = resolveLocator(page, step);
      await locator.hover();
      break;
    }

    case 'wait_for_text':
      await page.getByText(step.text).waitFor({
        timeout: step.timeout_ms || 5000,
      });
      break;

    case 'wait_for_selector':
      await page.waitForSelector(step.selector, {
        timeout: step.timeout_ms || 5000,
      });
      break;

    case 'wait_for_url':
      await page.waitForURL(new RegExp(step.pattern), {
        timeout: step.timeout_ms || 5000,
      });
      break;

    case 'scroll':
      if (step.selector) {
        await page.locator(step.selector).scrollIntoViewIfNeeded();
      } else if (step.x !== undefined && step.y !== undefined) {
        await page.evaluate(({ x, y }) => window.scrollTo(x, y), {
          x: step.x, y: step.y,
        });
      }
      break;

    case 'delay':
      await page.waitForTimeout(step.ms);
      break;

    case 'screenshot': {
      const name = step.name || `step-${result.steps.length}`;
      const path = join(journeyDir, `${name}.png`);
      await page.screenshot({
        path,
        fullPage: step.full_page || false,
      });
      result.screenshots.push(path);
      break;
    }

    case 'assert':
      // Inline assertion — will be handled separately
      // Just run it and throw on failure
      await evaluateAssertionDirect(page, step.assertion);
      break;

    case 'evaluate': {
      if (!spec.security?.allow_evaluate_steps) {
        throw new Error('Evaluate steps require security.allow_evaluate_steps = true');
      }
      const evalResult = await page.evaluate(step.script);
      if (step.expect_json !== undefined) {
        const expected = JSON.stringify(step.expect_json);
        const actual = JSON.stringify(evalResult);
        if (expected !== actual) {
          throw new Error(`Evaluate expected ${expected}, got ${actual}`);
        }
      }
      break;
    }

    default:
      throw new Error(`Unknown step action: ${step.action}`);
  }
}
```

---

## 36. Locator resolution

Locators are resolved in priority order. The first available target wins:

```javascript
function resolveLocator(page, target) {
  // 1. ARIA role + accessible name (most user-like, most stable)
  if (target.role && target.name) {
    return page.getByRole(target.role, { name: target.name });
  }

  // 2. Form label
  if (target.label) {
    return page.getByLabel(target.label);
  }

  // 3. Test ID (data-testid attribute)
  if (target.test_id) {
    return page.getByTestId(target.test_id);
  }

  // 4. Visible text
  if (target.text) {
    return page.getByText(target.text);
  }

  // 5. CSS selector (least desirable, most brittle)
  if (target.selector) {
    return page.locator(target.selector);
  }

  throw new Error(
    'Step has no locator target. Provide role+name, label, test_id, text, or selector.'
  );
}
```

Prefer role, label, and test_id. CSS selectors work but are brittle and couple tests to implementation rather than user-visible behavior.

---

## 37. Layout collector

Run after the journey completes (or after the last successful step if the journey fails partway):

```javascript
async function collectLayout(page) {
  return await page.evaluate(() => {
    const doc = document.documentElement;
    const body = document.body;
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    const documentWidth = Math.max(
      doc.scrollWidth || 0,
      doc.offsetWidth || 0,
      doc.clientWidth || 0,
      body ? body.scrollWidth || 0 : 0,
      body ? body.offsetWidth || 0 : 0
    );
    const documentHeight = Math.max(
      doc.scrollHeight || 0,
      body ? body.scrollHeight || 0 : 0
    );

    // Detect clipped text
    const clipped = [];
    for (const el of document.querySelectorAll('body *')) {
      if (clipped.length >= 20) break;
      const style = window.getComputedStyle(el);
      if (style.visibility === 'hidden' || style.display === 'none') continue;
      const text = (el.textContent || '').trim();
      if (!text) continue;
      if (
        el.scrollWidth > el.clientWidth + 1 ||
        el.scrollHeight > el.clientHeight + 1
      ) {
        const rect = el.getBoundingClientRect();
        clipped.push({
          text: text.slice(0, 120),
          rect: {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
          },
          scroll_width: el.scrollWidth,
          client_width: el.clientWidth,
          scroll_height: el.scrollHeight,
          client_height: el.clientHeight,
        });
      }
    }

    // Detect overlapping text (heuristic)
    const textElements = [];
    for (const el of document.querySelectorAll(
      'p, h1, h2, h3, h4, h5, h6, span, a, button, label, li, td, th, div'
    )) {
      const style = window.getComputedStyle(el);
      if (style.visibility === 'hidden' || style.display === 'none') continue;
      const text = (el.textContent || '').trim();
      if (!text || text.length < 2) continue;
      // Only leaf-ish text nodes
      if (el.children.length > 3) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      textElements.push({ text: text.slice(0, 80), rect });
    }

    const overlapping = [];
    for (let i = 0; i < textElements.length && overlapping.length < 10; i++) {
      for (let j = i + 1; j < textElements.length && overlapping.length < 10; j++) {
        const a = textElements[i].rect;
        const b = textElements[j].rect;
        const overlapX = Math.max(
          0,
          Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x)
        );
        const overlapY = Math.max(
          0,
          Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y)
        );
        const overlapArea = overlapX * overlapY;
        if (overlapArea > 100) {
          // > 100 sq px overlap
          overlapping.push({
            text_a: textElements[i].text,
            text_b: textElements[j].text,
            rect_a: {
              x: a.x,
              y: a.y,
              width: a.width,
              height: a.height,
            },
            rect_b: {
              x: b.x,
              y: b.y,
              width: b.width,
              height: b.height,
            },
            overlap_area: overlapArea,
          });
        }
      }
    }

    return {
      viewport_width: viewportWidth,
      viewport_height: viewportHeight,
      document_width: documentWidth,
      document_height: documentHeight,
      horizontal_overflow: documentWidth > viewportWidth + 1,
      clipped_text_candidates: clipped,
      overlapping_text_candidates: overlapping,
    };
  });
}
```

The 20-element cap on clipped candidates and 10-element cap on overlapping candidates prevent payload bloat. Parent elements containing clipped children will also match, so duplicates are expected.

---

## 38. Accessibility collection

```javascript
async function collectAccessibility(page, journeyDir) {
  const result = {
    snapshot_path: null,
    violations_path: null,
    critical: 0,
    serious: 0,
  };

  // Playwright accessibility snapshot
  try {
    const snapshot = await page.accessibility.snapshot();
    if (snapshot) {
      const snapshotPath = join(journeyDir, 'a11y.json');
      writeFileSync(snapshotPath, JSON.stringify(snapshot, null, 2));
      result.snapshot_path = snapshotPath;
    }
  } catch {
    // Accessibility snapshot not available — not an error
  }

  // axe-core (optional — only if the package is available)
  try {
    // Inject axe-core if available
    const axeSource = await import('@axe-core/playwright').catch(() => null);
    if (axeSource) {
      const { AxeBuilder } = axeSource;
      const axeResults = await new AxeBuilder({ page }).analyze();
      const violationsPath = join(journeyDir, 'axe.json');
      writeFileSync(violationsPath, JSON.stringify(axeResults, null, 2));
      result.violations_path = violationsPath;
      result.critical = axeResults.violations.filter(
        (v) => v.impact === 'critical'
      ).length;
      result.serious = axeResults.violations.filter(
        (v) => v.impact === 'serious'
      ).length;
    }
  } catch {
    // axe-core not available — not an error
  }

  return result;
}
```

Do not make axe-core a hard dependency. Design the result schema to accept axe results when available and return `{ critical: 0, serious: 0 }` when not.

---

## 39. Assertion evaluation

```javascript
async function evaluateAssertion(page, assertion, events, journeyResult) {
  const result = {
    name: `${assertion.type}`,
    passed: true,
    severity: 'hard',
    detail: null,
  };

  try {
    switch (assertion.type) {
      case 'text_visible':
        result.name = `text_visible:${assertion.text}`;
        await page.getByText(assertion.text).waitFor({ timeout: 3000 });
        break;

      case 'text_not_visible':
        result.name = `text_not_visible:${assertion.text}`;
        await expect(page.getByText(assertion.text)).not.toBeVisible();
        break;

      case 'element_exists':
        result.name = `element_exists:${assertion.selector}`;
        const el = await page.$(assertion.selector);
        if (!el) throw new Error(`Element not found: ${assertion.selector}`);
        break;

      case 'element_visible':
        result.name = `element_visible:${assertion.selector}`;
        const visible = await page.isVisible(assertion.selector);
        if (!visible) throw new Error(`Element not visible: ${assertion.selector}`);
        break;

      case 'element_count': {
        result.name = `element_count:${assertion.selector}=${assertion.expected}`;
        const count = await page.$$eval(assertion.selector, (els) => els.length);
        if (count !== assertion.expected) {
          throw new Error(
            `Expected ${assertion.expected} elements matching '${assertion.selector}', found ${count}`
          );
        }
        break;
      }

      case 'text_contains': {
        result.name = `text_contains:${assertion.selector}`;
        const text = await page.$eval(assertion.selector, (el) => el.textContent);
        if (!text?.includes(assertion.text)) {
          throw new Error(`Text does not contain '${assertion.text}'`);
        }
        break;
      }

      case 'url_matches': {
        result.name = `url_matches:${assertion.pattern}`;
        if (!new RegExp(assertion.pattern).test(page.url())) {
          throw new Error(`URL '${page.url()}' does not match pattern '${assertion.pattern}'`);
        }
        break;
      }

      case 'role_visible':
        result.name = `role_visible:${assertion.role}[${assertion.name}]`;
        await page.getByRole(assertion.role, { name: assertion.name }).waitFor({ timeout: 3000 });
        break;

      case 'no_console_errors': {
        const errors = events.consoleMessages.filter((m) => m.level === 'error');
        if (errors.length > 0) {
          throw new Error(
            `${errors.length} console error(s): ${errors
              .slice(0, 3)
              .map((e) => e.text)
              .join('; ')}`
          );
        }
        break;
      }

      case 'no_page_errors': {
        if (events.pageErrors.length > 0) {
          throw new Error(
            `${events.pageErrors.length} page error(s): ${events.pageErrors
              .slice(0, 3)
              .map((e) => e.message)
              .join('; ')}`
          );
        }
        break;
      }

      case 'no_failed_requests': {
        const allowPatterns = (assertion.allow || []).map((p) => new RegExp(p));
        const failed = events.networkRequests.filter((r) => {
          if (!r.failed && (!r.status || r.status < 400)) return false;
          if (allowPatterns.some((p) => p.test(r.url))) return false;
          return true;
        });
        if (failed.length > 0) {
          throw new Error(
            `${failed.length} failed request(s): ${failed
              .slice(0, 3)
              .map((r) => `${r.method} ${r.url} → ${r.status || 'failed'}`)
              .join('; ')}`
          );
        }
        break;
      }

      case 'no_horizontal_overflow': {
        if (journeyResult.layout?.horizontal_overflow) {
          throw new Error(
            `Horizontal overflow: document ${journeyResult.layout.document_width}px > viewport ${journeyResult.layout.viewport_width}px`
          );
        }
        break;
      }

      case 'no_clipped_text': {
        const clipped = journeyResult.layout?.clipped_text_candidates || [];
        if (clipped.length > 0) {
          throw new Error(
            `${clipped.length} clipped text element(s): "${clipped[0].text.slice(0, 40)}..."`
          );
        }
        result.severity = 'soft'; // Advisory
        break;
      }

      case 'no_overlapping_text': {
        const overlapping = journeyResult.layout?.overlapping_text_candidates || [];
        if (overlapping.length > 0) {
          throw new Error(
            `${overlapping.length} overlapping text pair(s)`
          );
        }
        result.severity = 'soft'; // Advisory — heuristic detection
        break;
      }

      default:
        result.severity = 'soft';
        result.detail = `Unknown assertion type: ${assertion.type}`;
    }
  } catch (error) {
    result.passed = false;
    result.detail = error.message;
  }

  return result;
}
```

---

## 40. Dev server management

When `dev_server` is specified in the spec, the Rust side manages the dev server process (start before runner, kill after). The runner itself does NOT start the dev server.

However, if you need the runner to handle dev server management (e.g., for standalone testing), here is the pattern:

1. **Spawn.** Start the dev server command as a child process. Inherit the workspace directory as `cwd`.
2. **Poll.** Send HTTP GET requests to the `base_url` every 500ms. Ready when HTTP 200. Timeout after 30 seconds.
3. **Run.** Execute browser journeys while the dev server is alive.
4. **Kill.** After the run, send SIGTERM. Wait 5 seconds. Send SIGKILL.

The Rust `DevServerHandle` RAII pattern is preferred for reliability.

---

## 41. Secret redaction in the runner

Apply redaction patterns from `spec.security.redact_text_patterns` to text evidence before writing to disk:

```javascript
function redact(text, patterns) {
  let result = text;
  for (const pattern of patterns) {
    result = result.replace(new RegExp(pattern, 'gi'), '[REDACTED]');
  }
  return result;
}

// Apply to console messages before writing
const redactedConsole = consoleMessages.map((m) => ({
  ...m,
  text: redact(m.text, spec.security?.redact_text_patterns || []),
}));

// Apply to network request URLs before writing
const redactedRequests = networkRequests.map((r) => ({
  ...r,
  url: redact(r.url, spec.security?.redact_text_patterns || []),
}));
```

Also redact headers from HAR files per `spec.security.redact_headers`.

---

## 42. Error handling

- **Invalid spec JSON.** Print error to stderr, exit 1.
- **Playwright not installed.** Print `{"error": "playwright not installed"}` to stdout, exit 1.
- **Browser launch failure.** Return `BrowserRunResult` with `passed: false`, empty viewports, failure class `browser_launch_failed`.
- **Navigation timeout.** Record as a step failure with error message. Do not crash.
- **Step timeout.** Record as a step failure. Short-circuit remaining steps in the journey. Continue to next journey.
- **Global timeout.** If the entire run exceeds `spec.timeout_ms`, kill the browser and return a partial result with `passed: false` and failure class `runner_infrastructure`.
- **File system errors.** Print to stderr, exit 1.

The key principle: UI failures are reported in JSON. Infrastructure failures exit 1. The runner should be robust — a broken page should produce a detailed failure report, not a crash.

---

## 43. Dependencies

```json
{
  "name": "roko-ui-runner",
  "version": "0.1.0",
  "type": "module",
  "dependencies": {
    "playwright": "^1.42.0"
  },
  "optionalDependencies": {
    "@axe-core/playwright": "^4.8.0"
  }
}
```

Install Chromium after installing the package:

```bash
npx playwright install chromium
```
