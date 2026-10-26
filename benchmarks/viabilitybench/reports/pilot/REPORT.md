# ViabilityBench pilot: cheap_direct, fd_claude, roko_fixed (pending)

**pilot, descriptive: two Python families (F1, F4), 20 tasks × 3 seeds, not pre-registered.**

Every estimate below has a 95% paired bootstrap interval (S09 §4.1: tasks resampled within family × level strata, then seeds within tasks; B = 10,000; method and limits at the end). Every row names the runs behind it; each run id links to its directory in this bundle, and `metrics.json` holds every number as a `vb.metric_record/1` with its run ids and interval. Counts and spend totals are tallies of the runs listed, not estimates, so they have no interval.

## Status

- **Merged:** PILOT-A-RECAL, PILOT-B from `benchmarks/viabilitybench/reports/pilot_a_recal`, `benchmarks/viabilitybench/reports/pilot_b`, on one suite, `sha256:630e4dcbeb4e77431e293a261b40cb5c629662a0975779674b586c835fe31173` (F1 f1-1.2.0; F4 f4-1.2.0). Runs are paired by (task, seed).
- `PILOT-A-RECAL` block `cheap-direct` (cheap_direct, gpt-oss-120b): all 60 planned runs recorded ([cheap-direct-1](cheap-direct-1/)).
- `PILOT-B` block `roko-fixed` (roko_fixed, gpt-oss-120b): **pending**, 0 of 60 planned runs; seeds 1, 2, 3.
- `PILOT-B` block `fd-claude` (fd_claude, claude-opus-5-5): all 60 planned runs recorded ([fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/)).
- `PILOT-B` block `provider-fault` (roko_fixed, gpt-oss-120b, under a disturbance, optional): not run, 0 of 5 planned runs; seeds 1.
- `PILOT-B` block `mini-swe` (cheap_direct [mini-swe-agent], gpt-oss-120b, optional): not run, 0 of 20 planned runs; seeds 1.
- **Not merged:** `benchmarks/viabilitybench/reports/pilot_a` (cheap_direct × seeds 1, 2, 3 (60 runs: cheap-direct-s1-1, cheap-direct-s2-1, cheap-direct-s3-1); fd_api × seeds 1 (5 runs: fd-api-s1-1)), suite `sha256:ebe9c4829ca608402650276e0231d275d61f1c4785675a5fb92ad79fb85d3535` (F1 f1-1.1.0; F4 f4-1.1.1): it ran another suite. The page pairs runs by (task, seed) on one suite only, so none of these runs is merged and no number here uses them.
- roko_fixed has no usable run: from 2026-10-20 15:17Z Cerebras answered every gpt-oss-120b call with HTTP 402 payment_required (gap-f3944c, open; last checked 2026-10-21). Pilot B's attempted roko_fixed units were set aside as abandoned, outside every bundle, and no number here uses them (gap-327242).
- Not run (optional): mini-swe needs mini-swe-agent's venv, which the host lacks, and provider-fault is roko_fixed on Cerebras (gap-327242).
- reports/pilot_a is Pilot A before the ℓ1–ℓ2 re-tune (gap-dd670b). Its ℓ3–ℓ5 instances render as in this suite (experiments/pilot_a_recal.toml), but its cheap_direct runs repeat PILOT-A-RECAL's (task, seed) keys, and its fd_api × 5 (BL8's billed-accounting check) ran one seed; its numbers stay in its own bundle and G0 page (reports/pilot_a/go-no-go.md).
- **G0** (`benchmarks/viabilitybench/reports/pilot_b/g0.json`, over PILOT-A-RECAL, PILOT-B): **NO-GO**, 7 pass, 1 fail, 7 not evaluated. Failed: Ladder: cheap_direct gpt-oss-120b at level 1: 0.5 against VS ≥ 0.6.

## Reading

- **cheap_direct against fd_claude:** $/VS ratio 0.348 [0.236, 0.508]; the interval lies below 1: cheaper per verified success; VS-rate difference -0.80 [-0.85, -0.75]; the interval excludes 0: the VS rates differ; pass^3 difference -0.85 [-0.90, -0.80]. So cheap_direct is cheaper per verified success but verifies less: this is not "cheaper at equal quality".
- **roko_fixed against fd_claude:** not measured yet (roko_fixed pending).
- **roko_fixed against cheap_direct:** not measured yet (roko_fixed pending). This is the harness effect: a null effect is possible at pilot scale (W10), and will be reported as one.
- cheap_direct verified no run at ℓ3, ℓ4, ℓ5, so its $/VS is undefined there.
- At this n, W10 expects only the cost ratio to resolve (token prices differ 10–25×) and VS differences under about 0.3 not to.

## Arms, all levels

| arm | experiment | runs kept / planned | VS rate | VS, unknown = 1 | pass^3 | $/VS, U′ (BCa) | $/VS, R (BCa) | \|U′ − R\| / R (BCa) | spend, U′ / billed | false greens / reported passes | false-green rate | cap-censored · infra_error · leak_suspected | runs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **cheap_direct** | PILOT-A-RECAL | 60 / 60 | 0.20 [0.15, 0.25] | 0.20 [0.15, 0.25] (no unknown label) | 0.15 [0.10, 0.20] | $0.0881 [$0.0598, $0.1267] | – | – | $1.0569 / $1.0569 | 22 / 34 | 0.65 [0.58, 0.72] | 2 · 0 · 0: F1-l5-0002 seed 1 aborted_cap in [cheap-direct-1](cheap-direct-1/); F1-l1-0002 seed 1 aborted_cap in [cheap-direct-1](cheap-direct-1/) | [cheap-direct-1](cheap-direct-1/) |
| **fd_claude** | PILOT-B | 60 / 60 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] (no unknown label) | 1.00 [1.00, 1.00] | $0.2532 [$0.2400, $0.2731] | $0.2532 [$0.2400, $0.2731] | 0.000 [0.000, 0.000] | $15.1947 / $0.0000 | 0 / 60 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| **roko_fixed** | PILOT-B | 0 / 60 | pending (seeds 1, 2, 3) | – | – | – | – | – | – | – | – | – | – |

## By level ℓ

| ℓ | arm | runs | VS rate | pass^3 | $/VS, U′ (BCa) | $/VS, R (BCa) | spend, U′ / billed | false greens / reported passes | false-green rate | cap-censored · infra_error · leak_suspected | runs |
|---|---|---|---|---|---|---|---|---|---|---|---|
| ℓ1 | cheap_direct | 12 | 0.50 [0.25, 0.75] | 0.25 [0.00, 0.50] | $0.0412 [$0.0154, $0.1259]‡ | – | $0.2469 / $0.2469 | 0 / 6 | 0.00 [0.00, 0.00]‡ | 1 · 0 · 0: F1-l1-0002 seed 1 aborted_cap in [cheap-direct-1](cheap-direct-1/) | [cheap-direct-1](cheap-direct-1/) |
| ℓ1 | fd_claude | 12 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] | $0.2057 [$0.1953, $0.2132] | $0.2057 [$0.1953, $0.2132] | $2.4682 / $0.0000 | 0 / 12 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| ℓ2 | cheap_direct | 12 | 0.50 [0.50, 0.50] | 0.50 [0.50, 0.50] | $0.0366 [$0.0184, $0.0534] | – | $0.2194 / $0.2194 | 1 / 7 | 0.14 [0.00, 0.33] | 0 · 0 · 0 | [cheap-direct-1](cheap-direct-1/) |
| ℓ2 | fd_claude | 12 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] | $0.1971 [$0.1880, $0.2047] | $0.1971 [$0.1880, $0.2047] | $2.3650 / $0.0000 | 0 / 12 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| ℓ3 | cheap_direct | 12 | 0.00 [0.00, 0.00] | 0.00 [0.00, 0.00] | – (no VS) | – | $0.1720 / $0.1720 | 7 / 7 | 1.00 [1.00, 1.00] | 0 · 0 · 0 | [cheap-direct-1](cheap-direct-1/) |
| ℓ3 | fd_claude | 12 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] | $0.2118 [$0.1944, $0.2308] | $0.2118 [$0.1944, $0.2308] | $2.5419 / $0.0000 | 0 / 12 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| ℓ4 | cheap_direct | 12 | 0.00 [0.00, 0.00] | 0.00 [0.00, 0.00] | – (no VS) | – | $0.1598 / $0.1598 | 8 / 8 | 1.00 [1.00, 1.00] | 0 · 0 · 0 | [cheap-direct-1](cheap-direct-1/) |
| ℓ4 | fd_claude | 12 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] | $0.2831 [$0.2369, $0.3681] | $0.2831 [$0.2369, $0.3681] | $3.3973 / $0.0000 | 0 / 12 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| ℓ5 | cheap_direct | 12 | 0.00 [0.00, 0.00] | 0.00 [0.00, 0.00] | – (no VS) | – | $0.2587 / $0.2587 | 6 / 6 | 1.00 [1.00, 1.00] | 1 · 0 · 0: F1-l5-0002 seed 1 aborted_cap in [cheap-direct-1](cheap-direct-1/) | [cheap-direct-1](cheap-direct-1/) |
| ℓ5 | fd_claude | 12 | 1.00 [1.00, 1.00] | 1.00 [1.00, 1.00] | $0.3685 [$0.3330, $0.4115] | $0.3685 [$0.3330, $0.4115] | $4.4222 / $0.0000 | 0 / 12 | 0.00 [0.00, 0.00] | 0 · 0 · 0 | [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |

‡ cheap_direct, $/VS, U′, ℓ1: 4 of 10,000 replicates undefined (no VS, counted as unbounded).
‡ cheap_direct, false-green rate, ℓ1: 4 of 10,000 replicates undefined (no reported pass, counted as 1).

## Paired contrasts

| contrast | all levels | ℓ1 | ℓ2 | ℓ3 | ℓ4 | ℓ5 | runs |
|---|---|---|---|---|---|---|---|
| $/VS ratio, cheap_direct ÷ fd_claude (BCa) | 0.348 [0.236, 0.508] | 0.200 [0.076, 0.584]‡ | 0.186 [0.091, 0.271] | – (cheap_direct verified no run here) | – (cheap_direct verified no run here) | – (cheap_direct verified no run here) | [cheap-direct-1](cheap-direct-1/), [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| VS rate, cheap_direct − fd_claude | -0.80 [-0.85, -0.75] | -0.50 [-0.75, -0.25] | -0.50 [-0.50, -0.50] | -1.00 [-1.00, -1.00] | -1.00 [-1.00, -1.00] | -1.00 [-1.00, -1.00] | [cheap-direct-1](cheap-direct-1/), [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| pass^3, cheap_direct − fd_claude | -0.85 [-0.90, -0.80] | -0.75 [-1.00, -0.50] | -0.50 [-0.50, -0.50] | -1.00 [-1.00, -1.00] | -1.00 [-1.00, -1.00] | -1.00 [-1.00, -1.00] | [cheap-direct-1](cheap-direct-1/), [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) |
| $/VS ratio, roko_fixed ÷ fd_claude (BCa) | pending: roko_fixed pending | – | – | – | – | – | – |
| VS rate, roko_fixed − fd_claude | pending: roko_fixed pending | – | – | – | – | – | – |
| pass^3, roko_fixed − fd_claude | pending: roko_fixed pending | – | – | – | – | – | – |
| $/VS ratio, roko_fixed ÷ cheap_direct (BCa) | pending: roko_fixed pending | – | – | – | – | – | – |
| VS rate, roko_fixed − cheap_direct | pending: roko_fixed pending | – | – | – | – | – | – |
| pass^3, roko_fixed − cheap_direct | pending: roko_fixed pending | – | – | – | – | – | – |

‡ $/VS ratio, cheap_direct ÷ fd_claude (BCa), ℓ1: 4 of 10,000 replicates undefined (an arm with no VS: unbounded).

D3's bar is shown for context only; the pilot does not test H1. H1 (S09 §4.2, §4.4) claims a level for `roko_full` against `fd_claude` when the 95% lower bound of the VS-rate ratio is at least X = 0.9 and the upper bound of the $/VS ratio is at most Y = 0.3, on 120 tasks × 3 seeds at G5, pre-registered and Holm-adjusted. No arm here is `roko_full`, the page is not pre-registered, and nothing on it is tested against the bar.

## False greens: visible pass, VS = 0

A false green is a reported pass whose VS-census label is 0 (App. D.1): for direct and CLI arms the visible checks passed on the final tree; for Roko arms the final gate verdict was `passed`.

| arm | family | runs | reported passes | false greens |
|---|---|---|---|---|
| cheap_direct | F4 | 30 | 29 | 18 |
| fd_claude | F4 | 30 | 30 | 0 |
| cheap_direct | F1 | 30 | 5 | 4 |
| fd_claude | F1 | 30 | 30 | 0 |

**F4: 18 false greens**

| arm | instance | ℓ | seed | status | run | record | failed checks |
|---|---|---|---|---|---|---|---|
| cheap_direct | F4-l3-0001 | 3 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:bc60b0ee06b4` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l3-0001 | 3 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:39d7f365ebcc` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l3-0001 | 3 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:56d387375f27` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l3-0002 | 3 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:7dd8084f0bc9` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l3-0002 | 3 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:38ae3d8d6741` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l3-0002 | 3 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:85085d69ffb9` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l4-0001 | 4 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:23e92b72006c` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l4-0001 | 4 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:ab2bd30f0a9a` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed, hidden.idempotent |
| cheap_direct | F4-l4-0001 | 4 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:83f5e9931a55` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l4-0002 | 4 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:33d4b33f8fa7` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l4-0002 | 4 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:80d57a353758` | hidden.completes, hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed, hidden.idempotent |
| cheap_direct | F4-l4-0002 | 4 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:1f689e9a5b6e` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0001 | 5 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:b6e52f8d6398` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0001 | 5 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:a86e54489acb` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0001 | 5 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:3d59100a5301` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0002 | 5 | 1 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:4e683d549f29` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0002 | 5 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:67dfb25a34ee` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |
| cheap_direct | F4-l5-0002 | 5 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:f59ebae142fc` | hidden.keys_renamed, hidden.none_lost, hidden.partial_resumed |

**F1: 4 false greens**

| arm | instance | ℓ | seed | status | run | record | failed checks |
|---|---|---|---|---|---|---|---|
| cheap_direct | F1-l2-0001 | 2 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:444f22a10519` | hidden.existing_errors.intact |
| cheap_direct | F1-l3-0001 | 3 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:7b1f5b3c2b18` | hidden.over_refund.raises, hidden.over_refund.new_error, hidden.error.subclasses_app_error, hidden.error.code_registered, hidden.error.code_unique, hidden.error.docs_row, hidden.error.message_format, hidden.over_refund.records_nothing, hidden.valid_refunds.unchanged, hidden.existing_errors.intact |
| cheap_direct | F1-l4-0001 | 4 | 3 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:9e002bd8d709` | hidden.over_refund.raises, hidden.over_refund.new_error, hidden.error.subclasses_app_error, hidden.error.code_registered, hidden.error.code_unique, hidden.error.docs_row, hidden.error.message_format, hidden.over_refund.records_nothing, hidden.valid_refunds.unchanged, hidden.existing_errors.intact |
| cheap_direct | F1-l4-0002 | 4 | 2 | completed | [cheap-direct-1](cheap-direct-1/) | `sha256:355889a5a037` | hidden.error.subclasses_app_error, hidden.error.code_registered, hidden.error.code_unique, hidden.error.docs_row, hidden.error.message_format |

## Verifier CI

`verifier-ci.json` (`vb.verifier_ci/1`, `ci/verify_verifiers.py --json`), copied here from the source bundle: green cells F1 100/100, F4 100/100; levels 1–5, seeds 1–10, latents v1, v2; secret `sha256:1675cc951cf12ce3`; 542.4 s; trimmed from sha256 `f18ae1cb0d314780521987cf54e76d0413a0c15abddc486b7d60cb34399274fa`. A cell is green when the reference passes the visible checks and the truth suite, the stub fails both, and planted gaming passes the visible checks but not the truth suite.
Of the 20 merged tasks, 20 have their own CI cell (same instance id and latent), and 0 of those are red.

## Method and limits

- **Bootstrap** (`analysis/bootstrap.py`, S09 §4.1 as written): paired, stratified by family × level; tasks are resampled within their stratum, then seeds within each drawn task; B = 10,000, seed 0. Percentile intervals for rates and differences; BCa for $/VS, the cost ratios and |U′ − R| / R, with the percentile interval where BCa is undefined (marked †). Arm rows resample the arm's own kept runs; contrasts resample the (task, seed) pairs both arms kept, so both arms see the same draws.
- **Open decision dec-3d5714:** In S09's synthetic simulation (`benchmarks/viabilitybench/reports/simulation/simulate-planted-0.15.json`, 1,000 campaigns, B = 10,000) this two-stage bootstrap over-covers: the VS-rate interval covered 0.975 (Monte Carlo SE 0.005), above the 0.93–0.97 band S09 sets for the toolkit (§6 E3), and the difference and ratio intervals covered 0.968 and 0.970, at the band's top. The decision records the simulation worker's explanation: the seed stage counts seed noise twice. The decision is whether to resample tasks only (n_h − 1 draws a stratum) or to accept the conservative intervals; until the author decides, the method stays as written, so these intervals are likely conservative (too wide).
- **pass^3 in a replicate** scores the seeds it drew as runs, so a task that passed 2 of 3 seeds scores 1 in a replicate that draws its passing seeds three times, while its estimate counts 0. With 3 seeds a task, as here, a replicate never scores a task below its estimate, so pass^3's replicates lean above it, and its interval can start at the estimate.
- **Undefined replicates** take the value least favourable to the arm (envelope.py's convention): $/VS or a cost ratio with no VS counts as unbounded, a false-green rate with no reported pass as 1. Such numbers are marked ‡ with the count.
- **Labels and costs:** VS is S05's VS-census label; an unknown label counts as 0, with the unknown = 1 bound beside it. $/VS is the API-equivalent spend over ΣVS, priced from one snapshot; R is the vendor's own figure (Claude Code's `total_cost_usd`). Subscription runs bill $0; billed dollars are shown apart.
- **Exclusions:** `infra_error` and `leak_suspected` runs are left out and counted; `aborted_cap` and `timeout` runs stay in with VS = 0 and are counted as cap censoring (App. D.1).
- **Limits:** descriptive and exploratory: the families, tasks and seeds in the label above, one suite of generated tasks, and dated model snapshots. Not pre-registered, so no p-value or Holm decision applies, and no claim about H1–H7 follows from this page.

## Provenance

- **This bundle:** [cheap-direct-1](cheap-direct-1/), [fd-claude-s1-1](fd-claude-s1-1/), [fd-claude-s2-1](fd-claude-s2-1/), [fd-claude-s3-1](fd-claude-s3-1/) (each run's summary files: manifest, records, ledger, and the scrubbed proxy log where it has one, copied from `benchmarks/viabilitybench/reports/pilot_a_recal`, `benchmarks/viabilitybench/reports/pilot_b`); `metrics.json` (183 MetricRecords, `experiment_id` `PILOT`, merging PILOT-A-RECAL, PILOT-B); `verifier-ci.json`; this page.
- **Computed** 2026-10-26T11:15:46Z at analysis commit `733f3e0d25285df1124f02e26bc6063e7ef7fa85`; prices `prices-2026-09-28`; suite `sha256:630e4dcbeb4e77431e293a261b40cb5c629662a0975779674b586c835fe31173`.
- **Check:** `benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot`.
- **Regenerate:** `vb report --pilot benchmarks/viabilitybench/reports/pilot_a_recal benchmarks/viabilitybench/reports/pilot_b --superseded benchmarks/viabilitybench/reports/pilot_a --bundle benchmarks/viabilitybench/reports/pilot --note 'roko_fixed has no usable run: from 2026-10-20 15:17Z Cerebras answered every gpt-oss-120b call with HTTP 402 payment_required (gap-f3944c, open; last checked 2026-10-21). Pilot B'"'"'s attempted roko_fixed units were set aside as abandoned, outside every bundle, and no number here uses them (gap-327242).' --note 'Not run (optional): mini-swe needs mini-swe-agent'"'"'s venv, which the host lacks, and provider-fault is roko_fixed on Cerebras (gap-327242).' --note 'reports/pilot_a is Pilot A before the ℓ1–ℓ2 re-tune (gap-dd670b). Its ℓ3–ℓ5 instances render as in this suite (experiments/pilot_a_recal.toml), but its cheap_direct runs repeat PILOT-A-RECAL'"'"'s (task, seed) keys, and its fd_api × 5 (BL8'"'"'s billed-accounting check) ran one seed; its numbers stay in its own bundle and G0 page (reports/pilot_a/go-no-go.md).'` (into an empty directory).
