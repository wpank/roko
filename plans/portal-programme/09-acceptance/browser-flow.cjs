'use strict';
/**
 * browser-flow.cjs — drive the Roko portal in a real browser.
 *
 * Usage: node browser-flow.cjs <mode> <url> <evidence-dir> [<workspace-name>]
 *
 * Modes:
 *   flow      — full two-action flow (generate + run) with fake agent timings.
 *   real      — same steps, longer timeouts for the live model.
 *   parallel  — click run-all and confirm two plans run concurrently.
 *
 * Each step prints:  BROWSER <step>: PASS|FAIL <detail>
 * The first FAIL saves fail-<step>.png and exits 1.
 * If no browser can be launched, prints BROWSER-UNAVAILABLE <reason> and exits 3.
 * Console/page errors are recorded to <evidence-dir>/browser-<mode>.json.
 */

const path = require('path');
const fs   = require('fs');
const os   = require('os');

const [,, MODE, URL_ARG, EVIDENCE_DIR, WS_NAME] = process.argv;

// ── Locate playwright from the demo app's node_modules ──────────────────────
const REPO_ROOT = path.resolve(__dirname, '../../..');
let chromium;
try {
    ({ chromium } = require(path.join(REPO_ROOT, 'demo/demo-app/node_modules/playwright')));
} catch (e) {
    console.log(`BROWSER-UNAVAILABLE playwright not found at demo/demo-app/node_modules: ${e.message}`);
    process.exit(3);
}

// ── Find installed headless-shell executables ────────────────────────────────
function findHeadlessShells() {
    const cacheDir = path.join(os.homedir(), 'Library/Caches/ms-playwright');
    const shells = [];
    let dirs;
    try { dirs = fs.readdirSync(cacheDir); } catch { return shells; }
    for (const d1 of dirs) {
        if (!d1.startsWith('chromium_headless_shell-')) continue;
        let subs;
        try { subs = fs.readdirSync(path.join(cacheDir, d1)); } catch { continue; }
        for (const d2 of subs) {
            if (!d2.startsWith('chrome-headless-shell-')) continue;
            const bin = path.join(cacheDir, d1, d2, 'chrome-headless-shell');
            if (fs.existsSync(bin)) shells.push(bin);
        }
    }
    return shells;
}

// ── Launch browser with fallbacks ────────────────────────────────────────────
async function launchBrowser() {
    // Try 1: default chromium
    try {
        return await chromium.launch({ headless: true });
    } catch (e1) {
        // Try 2: system Chrome
        try {
            return await chromium.launch({ channel: 'chrome', headless: true });
        } catch (e2) {
            // Try 3: each installed headless-shell
            for (const executablePath of findHeadlessShells()) {
                try {
                    return await chromium.launch({ executablePath, headless: true });
                } catch {}
            }
            const reason = `chromium: ${e1.message.split('\n')[0]}; chrome: ${e2.message.split('\n')[0]}`;
            console.log(`BROWSER-UNAVAILABLE ${reason}`);
            process.exit(3);
        }
    }
}

// ── Step helpers ─────────────────────────────────────────────────────────────
function passStep(name, detail) {
    console.log(`BROWSER ${name}: PASS ${detail || ''}`);
}

async function failStep(page, name, detail, evidenceDir) {
    console.log(`BROWSER ${name}: FAIL ${detail || ''}`);
    try {
        await page.screenshot({ path: path.join(evidenceDir, `fail-${name}.png`) });
    } catch {}
    process.exit(1);
}

// Require a step: fn() must resolve; on throw → FAIL + exit 1
async function requireStep(page, name, evidenceDir, fn) {
    try {
        const detail = await fn();
        passStep(name, detail || '');
    } catch (e) {
        await failStep(page, name, e.message, evidenceDir);
    }
}

// ── flow / real ──────────────────────────────────────────────────────────────
async function runFlow(page, url, wsName, evidenceDir, isReal) {
    const T = {
        rail:      30_000,
        signedIn:  15_000,
        generate:  isReal ? 660_000 : 120_000,
        runBtn:    30_000,
        running:   isReal ? 600_000 : 60_000,
        done:      isReal ? 1_800_000 : 180_000,
        revise:    120_000,
        runAgain:  30_000,
        runDone:   180_000,
    };

    let slug = null;
    let actionCount = 0;

    // 1. open
    await requireStep(page, 'open', evidenceDir, async () => {
        await page.goto(url);
        await page.locator('[data-region="rail"]').waitFor({ state: 'visible', timeout: T.rail });
        return 'rail visible';
    });

    // 2. signed-in
    await requireStep(page, 'signed-in', evidenceDir, async () => {
        await page.waitForFunction(() => !location.hash.includes('token='), { timeout: T.signedIn });
        const slot = page.locator('[data-region="header"] [data-slot="workspace"]');
        await slot.waitFor({ state: 'visible', timeout: 5_000 });
        if (wsName) {
            const text = await slot.textContent();
            if (!text.includes(wsName)) throw new Error(`workspace slot shows "${text}", want "${wsName}"`);
        }
        const alertCount = await page.locator('[data-region="alert"]').count();
        for (let i = 0; i < alertCount; i++) {
            const t = await page.locator('[data-region="alert"]').nth(i).textContent();
            if (t.includes('Not signed in')) throw new Error('alert says "Not signed in"');
        }
        return 'hash cleared, workspace correct';
    });

    // 3. first-run
    await requireStep(page, 'first-run', evidenceDir, async () => {
        await page.locator('[data-prompt]').waitFor({ state: 'visible', timeout: 5_000 });
        const n = await page.locator('[data-plan-row]').count();
        if (n !== 0) throw new Error(`expected no plan rows, found ${n}`);
        return 'prompt visible, no plans';
    });

    // 4. generate (action 1)
    await requireStep(page, 'generate', evidenceDir, async () => {
        await page.locator('[data-prompt]').fill('a rust app that prints hello world');
        await page.locator('[data-prompt]').press('Control+Enter');
        actionCount++;
        const row = page.locator('[data-plan-row][aria-current="true"]');
        await row.waitFor({ state: 'visible', timeout: T.generate });
        slug = await row.getAttribute('data-plan-row');
        const search = await page.evaluate(() => location.search);
        if (!search.includes(`plan=${slug}`)) throw new Error(`search "${search}" missing plan=${slug}`);
        const taskCount = await page.locator('[data-task-row]').count();
        if (taskCount < 1) throw new Error('no task rows after generate');
        await page.screenshot({ path: path.join(evidenceDir, '1-generated.png') });
        return `slug=${slug} tasks=${taskCount}`;
    });

    // 5. run (action 2)
    await requireStep(page, 'run', evidenceDir, async () => {
        const btn = page.locator('[data-action="run"]');
        await btn.waitFor({ state: 'visible', timeout: T.runBtn });
        await page.waitForFunction(
            () => !document.querySelector('[data-action="run"]')?.disabled,
            { timeout: T.runBtn }
        );
        await btn.click();
        actionCount++;
        return 'clicked run';
    });

    // 6. running
    await requireStep(page, 'running', evidenceDir, async () => {
        await page.locator('[data-task-row][data-state="active"]').waitFor({ state: 'visible', timeout: T.running });
        await page.locator('[data-region="transcript"]').waitFor({ state: 'visible', timeout: 10_000 });
        // Agent row contains slug + ' · '
        await page.waitForFunction(
            (s) => Array.from(document.querySelectorAll('[data-region="run-band"] [data-cell="agents"] [data-agent]'))
                .some(r => r.textContent.includes(s + ' · ')),
            slug,
            { timeout: 15_000 }
        );
        // At least one check rung
        await page.locator('[data-cell="checks"] [data-rung]').first().waitFor({ state: 'attached', timeout: 15_000 });
        await page.screenshot({ path: path.join(evidenceDir, '2-running.png') });
        return 'active task, transcript, agent row, rung';
    });

    // 7. live-step (flow only)
    if (!isReal) {
        await requireStep(page, 'live-step', evidenceDir, async () => {
            await page.waitForFunction(() => {
                for (const s of document.querySelectorAll('[data-step]')) {
                    if (!s.textContent.includes('Write')) continue;
                    const t = s.querySelector('[data-target]');
                    if (t && t.textContent === 'hello/main.rs' && s.querySelector('[data-live]')) return true;
                }
                return false;
            }, { timeout: T.running });
            return 'Write hello/main.rs with data-live';
        });
    }

    // 8. reload
    await requireStep(page, 'reload', evidenceDir, async () => {
        const baseUrl = url.split('#')[0].split('?')[0];
        await page.goto(baseUrl);
        await page.locator('[data-region="rail"]').waitFor({ state: 'visible', timeout: 30_000 });
        await page.locator(`[data-plan-row="${slug}"][aria-current="true"]`).waitFor({ state: 'visible', timeout: 10_000 });
        const alertCount = await page.locator('[data-region="alert"]').count();
        for (let i = 0; i < alertCount; i++) {
            const t = await page.locator('[data-region="alert"]').nth(i).textContent();
            if (t.includes('Not signed in')) throw new Error('alert says "Not signed in" after reload');
        }
        return 'rail visible, plan current, session persisted';
    });

    // 9. done
    await requireStep(page, 'done', evidenceDir, async () => {
        await page.locator(`[data-plan-row="${slug}"][data-state="done"]`).waitFor({ state: 'attached', timeout: T.done });
        await page.waitForFunction(() => {
            const rows = document.querySelectorAll('[data-task-row]');
            return rows.length > 0 && Array.from(rows).every(r => r.getAttribute('data-state') === 'done');
        }, { timeout: 15_000 });
        await page.locator('[data-region="run-band"]').waitFor({ state: 'hidden', timeout: 10_000 });
        await page.screenshot({ path: path.join(evidenceDir, '3-done.png') });
        return 'plan done, all tasks done, run-band gone';
    });

    // Steps 10–11: flow only
    if (!isReal) {
        // 10. revise
        await requireStep(page, 'revise', evidenceDir, async () => {
            await page.locator('[data-action="revise"]').click();
            await page.locator('[data-prompt]').fill('add a closing task');
            await page.locator('[data-prompt]').press('Control+Enter');
            await page.locator('[data-task-row="T99"]').waitFor({ state: 'visible', timeout: T.revise });
            await page.screenshot({ path: path.join(evidenceDir, '4-revised.png') });
            return 'T99 task row visible';
        });

        // 11. run-again
        await requireStep(page, 'run-again', evidenceDir, async () => {
            await page.locator('[data-action="run-again"]').click();
            // Plan leaves done within 30s
            await page.waitForFunction(
                (s) => document.querySelector(`[data-plan-row="${s}"]`)?.getAttribute('data-state') !== 'done',
                slug,
                { timeout: T.runAgain }
            );
            // Done again within 180s, T99 done
            await page.locator(`[data-plan-row="${slug}"][data-state="done"]`).waitFor({ state: 'attached', timeout: T.runDone });
            await page.locator('[data-task-row="T99"][data-state="done"]').waitFor({ state: 'attached', timeout: 10_000 });
            return 'plan ran again, T99 done';
        });
    }

    return { slug, actionCount };
}

// ── parallel ─────────────────────────────────────────────────────────────────
async function runParallel(page, url, evidenceDir) {
    // open
    await requireStep(page, 'open', evidenceDir, async () => {
        await page.goto(url);
        await page.locator('[data-region="rail"]').waitFor({ state: 'visible', timeout: 30_000 });
        return 'rail visible';
    });

    // click run-all
    await requireStep(page, 'run-all', evidenceDir, async () => {
        const btn = page.locator('[data-action="run-all"]');
        await btn.waitFor({ state: 'visible', timeout: 15_000 });
        await btn.click();
        return 'clicked run-all';
    });

    // two plans running concurrently within 60s
    await requireStep(page, 'parallel-running', evidenceDir, async () => {
        await page.waitForFunction(() => {
            const slot = document.querySelector('[data-region="header"] [data-slot="run"]');
            if (!slot || !slot.textContent.includes('2 running')) return false;
            const agents = document.querySelectorAll('[data-cell="agents"] [data-agent]');
            const plans = new Set(Array.from(agents).map(a => {
                const m = a.textContent.match(/^([^\s]+)\s+·\s+/);
                return m ? m[1] : null;
            }).filter(Boolean));
            return plans.size >= 2;
        }, { timeout: 60_000 });
        await page.screenshot({ path: path.join(evidenceDir, '5-parallel.png') });
        return '2 running in header, 2 different plans in agent list';
    });

    // all plan rows done within 300s
    await requireStep(page, 'parallel-done', evidenceDir, async () => {
        await page.waitForFunction(() => {
            const rows = document.querySelectorAll('[data-plan-row]');
            return rows.length > 0 && Array.from(rows).every(r => r.getAttribute('data-state') === 'done');
        }, { timeout: 300_000 });
        return 'all plan rows done';
    });
}

// ── main ─────────────────────────────────────────────────────────────────────
async function main() {
    if (!MODE || !URL_ARG || !EVIDENCE_DIR) {
        console.error('Usage: node browser-flow.cjs <mode> <url> <evidence-dir> [<ws-name>]');
        process.exit(1);
    }
    fs.mkdirSync(EVIDENCE_DIR, { recursive: true });

    const browser = await launchBrowser();
    const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
    });

    const consoleErrors = [];
    const pageErrors    = [];

    const page = await context.newPage();
    page.on('dialog', async d => { try { await d.accept(); } catch {} });
    page.on('console', msg => { if (msg.type() === 'error') consoleErrors.push(msg.text()); });
    page.on('pageerror', err => pageErrors.push(err.message));

    try {
        if (MODE === 'flow' || MODE === 'real') {
            const { slug, actionCount } = await runFlow(page, URL_ARG, WS_NAME || '', EVIDENCE_DIR, MODE === 'real');
            if (slug) console.log(`BROWSER-SLUG ${slug}`);
            console.log(`BROWSER-ACTIONS ${actionCount}`);
        } else if (MODE === 'parallel') {
            await runParallel(page, URL_ARG, EVIDENCE_DIR);
        } else {
            console.error(`Unknown mode: ${MODE}`);
            process.exit(1);
        }
    } finally {
        fs.writeFileSync(
            path.join(EVIDENCE_DIR, `browser-${MODE}.json`),
            JSON.stringify({ consoleErrors, pageErrors }, null, 2)
        );
        await browser.close().catch(() => {});
    }
}

main().catch(err => {
    console.error(err.stack || err.message);
    process.exit(1);
});
