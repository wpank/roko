# 07 - Agent Implementation Packets

Purpose: make Hugging Face integration implementable by low-context agents.
Each packet is narrow, has a bounded write set, and includes acceptance
criteria plus anti-patterns.

## Packet HF-00 - Config Examples And Doctor Text

Owner/write scope:

- config docs or fixtures;
- config doctor output/tests;
- no provider execution code.

Task:

- Add a documented Hugging Face `openai_compat` provider example using
  `https://router.huggingface.co/v1` and `HF_TOKEN`.
- Teach config doctor to mention whether `HF_TOKEN` is present when a
  Hugging Face provider is configured.

Acceptance:

- Example config parses.
- Config doctor never prints token value.
- Missing token is reported as a warning/error with provider id.

Do not do:

- Do not add new provider HTTP code.
- Do not add `ProviderKind::HuggingFaceApi` in this packet.

## Packet HF-01 - Dataset Client Skeleton

Owner/write scope:

- one new HF dataset client module;
- focused tests with mocked HTTP.

Task:

- Implement typed request/response structs for `/rows`.
- Enforce `length <= 100`.
- Parse `features` and `rows`.

Acceptance:

- Unit test builds expected URL.
- Unit test rejects length 101 without HTTP.
- Unit test parses a sample rows response.
- Unit test maps 429 plus `RateLimit` header to typed retry metadata.

Do not do:

- Do not implement Parquet in this packet.
- Do not wire SWE-bench execution yet.

## Packet HF-02 - SWE-bench Row Adapter

Owner/write scope:

- benchmark adapter module;
- tests with fixture JSON.

Task:

- Convert generic HF rows into `SweBenchInstance`.
- Require `instance_id`, `repo`, `base_commit`, and `problem_statement`.
- Preserve optional `patch`, `FAIL_TO_PASS`, and `PASS_TO_PASS` fields when
  present.

Acceptance:

- Valid fixture maps to typed instance.
- Missing required field returns schema error.
- Unknown extra fields are preserved or ignored intentionally.

Do not do:

- Do not clone repos or run benchmarks.
- Do not call live HF from unit tests.

## Packet HF-03 - CLI Dataset Rows Command

Owner/write scope:

- CLI command wiring;
- HF dataset client call;
- output formatting.

Task:

- Add:

```sh
roko hf dataset rows <dataset> --split <split> --offset <n> --length <n>
```

Acceptance:

- Command uses `HfDatasetClient`.
- Length over 100 fails locally.
- `--json` output includes dataset/config/split/offset/length/features/rows.
- Tests cover argument parsing without network.

Do not do:

- Do not add benchmark execution.
- Do not require `HF_TOKEN` for public dataset reads unless configured.

## Packet HF-04 - Cache For Dataset Rows

Owner/write scope:

- HF dataset cache module;
- CLI flag `--refresh`.

Task:

- Cache `/rows` responses by dataset/config/split/revision/offset/length.

Acceptance:

- Second call with same mocked request reads cache.
- `--refresh` bypasses cache and rewrites it.
- Cache metadata records source URL and fetched time.

Do not do:

- Do not cache auth tokens.
- Do not use row offset as stable benchmark identity.

## Packet HF-05 - Router Model Listing Client

Owner/write scope:

- HF inference catalog client;
- mocked tests.

Task:

- Implement `GET https://router.huggingface.co/v1/models` through a typed,
  mockable client.
- Store results as candidate models.

Acceptance:

- Parses model id list from fixture.
- Missing `HF_TOKEN` returns typed auth missing if endpoint requires auth.
- 401/403/429 are typed.
- Candidate cache is append-only JSONL.

Do not do:

- Do not promote models into `roko.toml`.
- Do not call this client during normal model dispatch.

## Packet HF-06 - Candidate Probe Through ModelCallService

Owner/write scope:

- candidate probe command/service;
- tests with mocked `ModelCaller`.

Task:

- Probe one candidate using `ModelCallService` or a `ModelCaller` trait object.
- Record non-stream and stream results.

Acceptance:

- Probe result records local alias, backend slug, provider policy, success,
  latency, usage observation, and error kind.
- Probe failure does not edit config.

Do not do:

- Do not direct POST to HF router.
- Do not route production traffic to probed model.

## Packet HF-07 - Promotion Diff

Owner/write scope:

- candidate registry;
- config edit/diff module.

Task:

- Convert a `ProbePassed` candidate into a proposed TOML diff.
- Require explicit alias.

Acceptance:

- Diff adds `[models.<alias>]` only.
- Existing aliases are rejected unless `--replace` is explicit.
- `default_model` is never changed.

Do not do:

- Do not write the config silently.
- Do not infer capabilities that were not observed.

## Packet HF-08 - Parquet Listing

Owner/write scope:

- HF dataset client;
- tests with `/parquet` fixture.

Task:

- Add `/parquet?dataset=...` support and parse files with dataset/config/split,
  URL, filename, size.

Acceptance:

- Parses multiple configs/splits.
- Handles partial conversion names without dropping them.
- Does not download files yet.

Do not do:

- Do not add a Parquet reader in this packet.

## Packet HF-09 - Episode Export Dry Run

Owner/write scope:

- learning/runtime export module;
- local JSONL output only.

Task:

- Convert verified episodes into a local HF dataset JSONL bundle.
- Include redaction/license/provenance status fields.

Acceptance:

- Dry run writes local files and manifest.
- Any failed redaction/license status blocks upload plan.
- No network calls.

Do not do:

- Do not upload to HF.
- Do not include raw env vars/secrets.

## Packet HF-10 - Hub Upload Plan

Owner/write scope:

- artifact upload planning module;
- no live upload unless behind ignored/live test.

Task:

- Create a plan for creating/updating a private dataset repo and uploading the
  local export bundle.

Acceptance:

- Plan includes repo id, repo type, visibility, files, sizes, and required token
  scope.
- Default visibility is private.
- Dry-run output is stable JSON.

Do not do:

- Do not make public repos the default.
- Do not upload without explicit command flag.

## Packet HF-11 - Jobs Plan Builder

Owner/write scope:

- jobs plan structs;
- CLI dry-run command.

Task:

- Generate a saved HF Jobs plan for TRL SFT from a dataset repo and base model.

Acceptance:

- Plan includes image, flavor, command, env, secrets, input dataset, output repo.
- No job is submitted by default.
- Tests validate stable serialization.

Do not do:

- Do not require a live HF account.
- Do not train anything in this packet.

## Packet HF-12 - Webhook Event Parser

Owner/write scope:

- webhook payload parser;
- replay/idempotency key tests.

Task:

- Parse HF webhook payloads for repo updates and discussions.
- Extract repo type/name/api URL and event action.

Acceptance:

- Fixture payload parses.
- Idempotency key is stable for replayed event.
- Unknown event types are ignored with typed `Unknown` variant.

Do not do:

- Do not submit jobs from webhook handler yet.
- Do not expose an unauthenticated public route.

## Universal Verification

Every packet must run:

```sh
cargo fmt --check
git diff --check
```

And the focused crate test/check for its write scope.

If a packet touches provider execution, also run a static check proving no new
raw HF router calls were added outside the allowed HF client module.

## Universal Anti-Patterns

- Do not build a broad crate full of unused abstractions.
- Do not add raw provider HTTP in chat/ACP/serve execution surfaces.
- Do not write secrets into config, prompts, manifests, or logs.
- Do not let discovery mutate production routing.
- Do not convert unknown usage/cost/capability into zero or false.
- Do not hide rate limits as generic errors.
- Do not publish artifacts publicly by default.
