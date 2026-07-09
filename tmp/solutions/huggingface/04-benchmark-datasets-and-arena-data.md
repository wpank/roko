# 04 - Benchmark Datasets And Arena Data

## Problem

Roko currently has SWE-bench scripts that depend on Python `datasets`. That is
fine for experiments, but the long-term arena runtime needs a typed Rust data
path:

- deterministic dataset slices;
- cacheable rows and Parquet shards;
- typed schema validation;
- no hidden Python dependency for normal arena runs;
- provenance for every benchmark instance.

Hugging Face Dataset Viewer provides the right first layer.

## First Vertical Slice

Add a small client that can load a SWE-bench Lite slice:

```sh
roko hf dataset rows princeton-nlp/SWE-bench_Lite --split test --offset 0 --length 3
```

The output should be typed JSON that can later feed an arena run:

```json
{
  "dataset": "princeton-nlp/SWE-bench_Lite",
  "config": "default",
  "split": "test",
  "offset": 0,
  "length": 3,
  "schema_fingerprint": "blake3:...",
  "rows": [
    {
      "row_idx": 0,
      "instance": {
        "instance_id": "...",
        "repo": "...",
        "base_commit": "...",
        "problem_statement": "...",
        "patch": "..."
      }
    }
  ]
}
```

## Client API

```rust
pub struct HfDatasetClient {
    http: Arc<dyn HttpClient>,
    base_url: String,
    token: Option<SecretRef>,
    cache: HfDatasetCache,
}

pub struct HfRowsRequest {
    pub dataset: String,
    pub config: Option<String>,
    pub split: String,
    pub offset: u64,
    pub length: u8, // enforce <= 100
    pub revision: Option<String>,
}

pub struct HfRowsResponse {
    pub features: Vec<HfFeature>,
    pub rows: Vec<HfRow>,
    pub source: HfDatasetSource,
}

pub struct HfParquetFile {
    pub dataset: String,
    pub config: String,
    pub split: String,
    pub url: String,
    pub filename: String,
    pub size: u64,
}
```

## Endpoint Coverage

| Endpoint | Phase | Use |
|---|---|---|
| `/rows` | P0 | Smoke slices and deterministic small runs. |
| `/splits` | P0 | Discover valid config/split values. |
| `/size` | P0 | Estimate shard counts and progress. |
| `/parquet` | P1 | Bulk ingestion through Parquet files. |
| `/search` | P2 | Find benchmark rows by text. |
| `/filter` | P2 | Select rows by metadata predicates. |

## `/rows` Rules

- Enforce `length <= 100` before making a request.
- Require `dataset` and `split`.
- Treat missing `config` as `None`, not `"default"` unless returned by HF.
- Return `ExternalSchemaChanged` if required fields are absent.
- Preserve `features` for schema fingerprinting.
- Cache rows by dataset, config, split, revision, offset, length, and auth mode.

## `/parquet` Rules

Use `/parquet` for bulk arena ingestion:

1. list Parquet files;
2. store file URL, size, dataset/config/split;
3. download via resolver URL with token when needed;
4. verify size/hash if available;
5. read with a Rust Parquet/Arrow path;
6. map rows to typed arena instances.

Do not download full dataset repos with `git clone` for arena runs unless a
specific benchmark requires repository metadata.

## Cache Design

Cache root:

```text
.roko/cache/hf/
  datasets/
    rows/
      <dataset-hash>/<config>/<split>/<revision>/<offset>-<length>.json
    parquet/
      <dataset-hash>/<config>/<split>/<filename>.parquet
  manifests/
    <dataset-hash>.json
```

Cache metadata:

```json
{
  "dataset": "princeton-nlp/SWE-bench_Lite",
  "config": null,
  "split": "test",
  "revision": null,
  "fetched_at": "2026-05-01T00:00:00Z",
  "schema_fingerprint": "blake3:...",
  "source_url": "https://datasets-server.huggingface.co/rows?...",
  "rate_limit": {
    "bucket": "api",
    "remaining": 950,
    "reset_after_s": 120
  }
}
```

## Arena Mapping

Define benchmark adapters separately from HF transport:

```rust
pub trait ArenaDatasetAdapter {
    type Instance;

    fn dataset_id(&self) -> &'static str;
    fn schema_version(&self) -> u32;
    fn from_hf_row(&self, row: &HfRow) -> Result<Self::Instance>;
}
```

SWE-bench adapter fields:

```rust
pub struct SweBenchInstance {
    pub instance_id: String,
    pub repo: String,
    pub base_commit: String,
    pub problem_statement: String,
    pub patch: Option<String>,
    pub fail_to_pass: Vec<String>,
    pub pass_to_pass: Vec<String>,
}
```

The adapter owns schema drift. The HF client only owns transport and generic
row parsing.

## Scoring Flow

Initial flow:

```text
HF Dataset Viewer rows
  -> SweBenchInstance
  -> arena workdir clone
  -> Roko run
  -> patch output
  -> official/local scorer
  -> ArenaResult
  -> learning episode
```

Do not mix dataset loading, prompt construction, and scoring in one function.

## Verification

Unit tests:

- `/rows` URL encoding for dataset/config/split/offset/length.
- `length > 100` rejects locally.
- rows response parses features and row content.
- missing required SWE-bench fields returns schema error.
- 429 with `RateLimit` header returns retry metadata.
- cache key changes with revision/config/split.

Integration tests behind env:

```sh
HF_TOKEN=... cargo test -p roko-cli hf_dataset_rows_live -- --ignored
```

No-token smoke can use a public dataset, but it should not be the only proof.

## Acceptance Criteria

- A Rust command loads at least one SWE-bench Lite row without Python
  `datasets`.
- The adapter rejects schema drift with a typed error.
- `/rows` length limit is enforced before network.
- Repeated command hits local cache unless `--refresh` is set.
- Rate-limit and auth errors are typed.
- No arena code directly builds Dataset Viewer URLs outside the HF client.

## Anti-Patterns

- Do not make the HF client return `serde_json::Value` all the way into arena
  execution.
- Do not silently skip rows with schema errors.
- Do not use Python `datasets` as the hidden fallback for the Rust path.
- Do not download whole datasets for a 10-row smoke test.
- Do not treat HF row index as stable identity; benchmark instance id is the
  stable id when provided.
