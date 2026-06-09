# Ecosystem Patterns: What Makes Plugin Systems Succeed

Research across 12 successful extensible platforms to identify the patterns that drive
exponential adoption. Applied to roko's adapter architecture in the April 2026 market.

Last updated: 2026-04-29.

---

## Platform Comparison Matrix (Updated April 2026)

| Ecosystem | Core Interface | Methods | Language | Time to First | Ecosystem Size | Status |
|---|---|---|---|---|---|---|
| MCP | Tool/Resource/Prompt | 1-3 | Any (JSON-RPC) | ~30 min | 17,468+ servers | 97M monthly SDK downloads |
| LangChain | Runnable | 4 | Python | ~5 min | 30K+ stars | Enterprise LangGraph growing |
| CrewAI | BaseTool | 1 (`_run`) | Python | ~5 min | 80+ tools | Role-based, accessible |
| OpenAI Agents SDK | Function tool | 1 | Python | ~2 min | Native | Integrated with ChatGPT |
| n8n | INodeType | desc+execute | TypeScript | ~30 min | 6,234 nodes | 13.6 nodes/day growth |
| Terraform | Provider (gRPC) | Schema+CRUD | Go (any gRPC) | ~2 hours | 3,500+ providers | $6.4B acquisition |
| K8s Operators | CRD+Reconciler | 1 | Go (any gRPC) | ~1 hour | 300+ certified | CNCF governance |
| VS Code | Extension | activate+contrib | TypeScript | ~15 min | Tens of thousands | Dominant IDE |
| Zed | WASM extension | trait impl | Rust->WASM | ~30 min | Growing | Rust-native |
| Airbyte | Source/Destination | 4 commands | Any (Docker) | ~30 min | 400+ connectors | $1.5B valuation |
| OTel | Exporter | 1-2 functions | Any (gRPC) | ~30 min | 200+ components | CNCF, gen_ai.* |
| Backstage | Plugin | Ext. Points | TypeScript | ~1 hour | 200+ plugins | 3,400+ orgs, 89% share |
| **Roko (current)** | **6 traits** | **2-10 each** | **Rust only** | **~2-4 hours** | **Internal** | 18 crates, 177K LOC |
| **Roko (target)** | **<=5 methods** | **1-5 each** | **Rust + process** | **~30 min** | **50+ by end 2026** | Adapter-trait |

---

## The 7 Patterns (Empirically Validated)

### Pattern 1: Minimal Interface, Maximum Composability

Every successful ecosystem defines the smallest possible interface. The interface must fit
in one developer's head in under 10 minutes.

**Empirical data**:
- MCP: 3 primitives. From 2M to 97M monthly SDK downloads in 16 months.
- OTel: 3 pipeline stages. 200+ community components.
- Terraform: Schema + CRUD. 3,500+ providers.
- Airbyte: 4 commands. 400+ connectors.
- Bevy: 1 required method. Growing game ecosystem.
- LangChain: Replaced complex Chain (8+ methods) with Runnable (4 methods) -> adoption exploded.

**Anti-pattern**: Tower's `Service` trait (two methods, one associated future type) is
famously easy to mis-implement. Async-fn-in-trait initiative explicitly cites Tower as the
case study. Skip Tower's complexity unless roko needs backpressure on plugins.

**Roko application**: Each adapter trait <=5 required methods. The current 6 core traits
are internally sound but externally opaque. External adapters need simpler interfaces.

---

### Pattern 2: Process Boundary > Code Boundary

Every platform that achieved massive ecosystem growth defined its plugin interface at the
process boundary, not at the code boundary.

| Platform | Original | Migration | Result |
|---|---|---|---|
| K8s | In-tree plugins | CSI/CNI over gRPC | 300+ plugins |
| Terraform | Built-in | gRPC `go-plugin` | 3,500+ providers |
| VS Code | In-process | Extension Host | Tens of thousands |
| Airbyte | Python SDK | Docker containers | 400+ in any language |
| MCP | Started with stdio | stdio + Streamable HTTP | 17,468+ servers |

**Why process boundaries win**:
1. Language independence: anyone can write a plugin
2. Crash isolation: plugin crash does not take down the host
3. Independent versioning: plugins release on their own schedule
4. Security sandboxing: plugins cannot access arbitrary host resources

**Roko application**: Define Roko Connector Protocol over stdio (like MCP) or gRPC (like
Terraform). Adapters are either Rust trait impls (in-process, fast) OR external processes
(any language, isolated).

---

### Pattern 3: Declarative for 80%, Programmatic for 20%

Most integrations are "call this REST API with these headers and parse this JSON." Making
the common case declarative expands the contributor base dramatically.

**Empirical data**:
- n8n: Declarative nodes define HTTP requests via YAML. No code for REST API wrappers.
- Airbyte Low-Code CDK: YAML manifest maps API endpoints to streams. <10 min for new source.
- VS Code Contribution Points: Many extensions are pure JSON -- zero TypeScript.
- Terraform HCL: Declarative infrastructure.

**Roko application**: `connector.toml` manifest format. This generates a `WorkSource`
implementation without writing Rust. Power users implement the trait directly.

---

### Pattern 4: Registry + Auto-Discovery

The registry creates a discoverability flywheel.

**Empirical data**:
- Terraform Registry: `terraform init` auto-downloads. JFrog +800%, Okta +350%, Heroku +500%
  growth in single years after registry launch.
- VS Code Marketplace: In-editor search with ratings, downloads, categories.
- npm (for MCP): 34,700+ dependents on `@modelcontextprotocol/sdk`.
- crates.io: Trusted publishing shipped July 11, 2025 (RFC 3691).

**Roko application**:
1. `roko-contrib` monorepo (OTel-contrib model)
2. `roko.toml` declares adapters -> `roko init` downloads them
3. Ship with categories on day one (crates.io took ~3 years to add categories)
4. Adopt OIDC trusted publishing from launch

---

### Pattern 5: The Escape Hatch

When no specific integration exists, a generic fallback makes the platform useful immediately.

- n8n: HTTP Request node works with any REST API
- LangChain: `@tool` decorator turns any function into a tool
- MCP: Raw JSON-RPC means any HTTP endpoint is a potential resource

**Roko application**: Generic HTTP adapter, webhook adapter, MCP adapter, CLI adapter.
These exist "for free" and make roko useful before the ecosystem exists.

---

### Pattern 6: Catalog as Gravity Well

Once entities are in a catalog, every plugin becomes more valuable because it can discover
and act on those entities.

- Backstage Software Catalog: 3,400+ orgs, 89% market share in developer portals.
- Terraform State: Every resource in state. Data sources cross-reference providers.
- K8s etcd: Every resource. CRDs extend with new entity types.

**Roko application**: `.roko/` is the catalog. Formalize it:

| Entity Kind | Location | Catalog Path |
|---|---|---|
| Signal | `.roko/signals.jsonl` | `catalog://signals/{hash}` |
| Episode | `.roko/episodes.jsonl` | `catalog://episodes/{id}` |
| Plan | `.roko/plans/{name}/` | `catalog://plans/{name}` |
| Task | tasks.toml | `catalog://tasks/{plan}/{id}` |
| PRD | `.roko/prd/{slug}/` | `catalog://prds/{slug}` |
| Knowledge | `.roko/neuro/` | `catalog://knowledge/{tier}/{id}` |
| Agent | runtime state | `catalog://agents/{role}/{instance}` |
| Route | `.roko/learn/cascade-router.json` | `catalog://routing/{context}` |

Every adapter that puts data IN benefits every adapter that takes data OUT.

---

### Pattern 7: Lazy Activation

Extensions only load when needed. Startup cost does not scale with installed adapters.

- VS Code: 50,000+ extensions, sub-second startup
- Zed: WASM extensions only instantiated when their language is requested
- K8s Controllers: Only watch their own CRD type

**Roko application**: `activate_on = ["gate:security-scan"]` in adapter config.

---

## Empirical Lock-In Thresholds (Updated April 2026)

### How many integrations build a real moat?

| Platform | Lock-In Threshold | Current | Outcome |
|---|---|---|---|
| **Terraform** | ~500 providers (2019-2020) | 3,500+ | $6.4B IBM acquisition (Feb 2025) |
| **Zapier** | ~3,000 integrations | 7,000+ | N^2 potential workflows |
| **Airbyte** | Building (~400) | 400+ | $1.5B valuation, Docker-as-boundary was key |
| **n8n** | Building (~1,000) | 6,234 | 13.6 nodes/day growth |
| **MCP** | Building fast | 17,468+ | 52% abandonment quality crisis |

### Roko targets

| Count | Moat Level | Migration Cost | Target |
|---|---|---|---|
| Sub-50 | None | Hours | Today |
| 50-200 | Tactical | 1-2 weeks/workflow | End 2026 |
| 200-500 | Meaningful | Months | End 2027 |
| 500+ | Prohibitive | Multi-quarter | End 2028 |

### Acceleration strategies

1. **Declarative connector.toml**: Airbyte's low-code CDK proved 80% need zero imperative code
2. **Process boundary**: Terraform/Airbyte unlocked non-Go/non-Java contributions
3. **Registry as gravity well**: `roko init` auto-downloads, like `terraform init`
4. **Bounty program**: Airbyte grew catalog 110->150 in 1.5 months at $150-300/connector
5. **Verification badge**: Terraform's 42 verified modules = >95% of all downloads

---

## Contributor Funnel (Empirical Data)

### Time-to-first-response is #1 retention lever

Calefato et al. (Empirical Software Engineering 27.3, 2022): ~45% of core developers
completely disengage for >=1 year. Return probability drops from 35-55% to 21-26% once
break crosses one year. Lower first-response latency directly correlates with quicker
contributor responses and future contribution likelihood.

**Commitment**: <72 hour first response on PRs, instrumented via CHAOSS metric.

### Good First Issue nuance

- GFI starters: 61.2% made 2+ contributions vs 47-48% of mentored-bug starters
- But: expert involvement negatively correlates with retention when experts complete work
- Pair every GFI with explicit mentor assignment; **mentor must coach, not finish**
- Bevy's nuanced label taxonomy (`X-Needs-SME`, `A-Cross-Cutting`, `C-Docs`) is better
  than single "good first issue" label

### Macro funnel

- 1.4M first-time OSS contributors per year (GitHub Octoverse 2025)
- TypeScript overtook Python as #1 GitHub language by contributors (Aug 2025)
- 180M+ developers on GitHub total
- 43.2M PRs merged/month (+23% YoY in 2025)
- The typed-language tailwind favors Rust toolkits for next 24-36 months

---

## Verification Badge Economics

Terraform February 2018 RedMonk audit: 42 verified modules out of 376 total = >95% of all
downloads. AWS modules alone >94% of total.

**Inversion of conventional wisdom**: The data says curation > quantity at early stage.
10-20 verified reference adapters do 95% of discovery work.

**Roko v1**: Plan around 10-20 "Roko Verified" adapters with real review process. Skip
composite quality scoring until ~1,000 adapters. Binary `verified: true` is sufficient.

---

## First-Party vs Community Ratio Collapse

Once a platform passes ~50 functional components, first-party share drops predictably:

| Platform | Official | Community | Ratio |
|---|---|---|---|
| n8n (April 2026) | ~400 | ~5,834 | 1:14 |
| OTel Collector Contrib | N/A | 200+ | Virtually all community |
| Terraform (at acquisition) | <2% | >3,500 | <2% first-party |

**Pattern**: Past ~50 components, first-party drops below 20% and continues to ~5% by 500.
Verification, scaffolding, and bounty machinery must be in place before crate #25.

---

## MCP Quality Crisis (April 2026)

MCP servers crossed 10,000+ in registries with a 52% abandonment rate (Rapid Claw audit,
April 2026). Enterprises deploying MCP run into: audit trails, SSO-integrated auth, gateway
behavior, configuration portability.

**Roko opportunity**: Position as the quality layer for MCP. The verification badge +
conformance crate addresses exactly what the MCP ecosystem lacks. MCP support is table stakes;
MCP quality is the differentiator.

---

## Activation, Retention & Distribution

### Activation keystone

Supabase's keystone is "create a database," not signup. All funnel metrics re-oriented around
this single event. Supabase: 4.5M developers, $5B Series E (Oct 2025).

**Roko keystone**: "First successful agent invocation that hit a real model API and returned
a trace." Not `cargo install`, not signup, not first adapter scaffolded.

### Event-driven distribution

Supabase uses Postgres webhooks on product events, not calendar drips.

**Roko events**:

| Event | Triggered Action |
|---|---|
| `agent_first_run` | Trace tutorial |
| `second_adapter_added` | Recipe library |
| `7d_inactive_after_install` | Recovery email |
| `first_gate_pass` | Showcase CTA |

### Personal outreach

OpenView 2022: "companies that performed outreach retained 2-3x better." For first 100-500
signups, Will should personally email anyone whose first run succeeded but who has not
returned by Day 3.

### k-Factor target: 0.2 in Year 1

SaaS typical k = 0.2; consumer 0.45 median; k > 1.0 incredibly rare. Required:
- Shareable artifacts (trace URLs)
- Public showcase ("Made with Roko")
- Visible status badges

### Roko Week quarterly cadence

Modeled on Supabase Launch Week (15+ since 2020). Ship features daily for one week.
Shareable virtual tickets boost swag/prize odds when posted to social. For solo founder,
quarterly is sufficient.

---

## crates.io Discovery (Rust-Specific)

Discovery channels ranked by traffic impact:

1. **Transitive dependencies** -- becoming a dep of one popular crate compounds traffic
2. **lib.rs / blessed.rs** -- curated Rust crate lists
3. **crates.io search** -- direct discovery
4. **awesome-rust** -- community curated
5. **This Week in Rust** -- weekly newsletter, Crate of the Week

**Download counts are misleading**: 2.2-3.4x higher on weekdays (CI traffic). 249 of top
1,000 crates are abandoned but still pulled billions of times. Roko is a binary/runtime,
not a library -- won't compete on raw downloads.

---

## AAIF Governance (April 2026)

The AAIF surpassed CNCF in membership at same stage: 170+ organizations in <4 months.

**Platinum members**: AWS, Anthropic, Block, Bloomberg, Cloudflare, Google, Microsoft, OpenAI.

**No European Platinum member** visible. SEP authorship + Technical Committee participation
as under-represented EU voice is higher leverage than any A2A working group.

**Key events**:
- MCPCon Europe: Amsterdam, Sep 17-18
- MCPCon NA: Oct 22-23

---

## Marketplace Economics

| Platform | Revenue Share | Notes |
|---|---|---|
| GitHub Marketplace | 5% (from 25%, Jan 2021) | Developer-friendly |
| GitHub Actions | 0% | Pure distribution-as-marketing |
| Figma | 15% | Seller program closed |
| Slack | 0% | Curate only |
| VS Code | 0% | No payment processing |

GitHub Actions: 5M workflow runs/day, zero revenue share, enormous impression count.
Vendors publish adapters for marketing alone if publishing is trivial. Roko should run
the same play.

---

## Scaffolding Benchmarks

| Platform | No-code | Low-code | Full-code |
|---|---|---|---|
| Airbyte CDK | <10 min | <30 min | ~3 hours |
| Backstage | N/A | `yarn backstage-cli new` | ~1 hour |
| **Roko target** | connector.toml (<10 min) | `cargo generate` (<15 min) | Full trait (<1 hour) |

---

## Sources

- MCP: 97M monthly SDK downloads (Apr 2026), 17,468 servers (Nerq census), AAIF 170+ orgs
- n8n: 6,234 nodes, 9,487 templates, 13.6 nodes/day (April 2026)
- Terraform: 3,500+ providers, $6.4B IBM acquisition (Feb 2025), 42 verified = >95% downloads
- Airbyte: 400+ connectors, $1.5B valuation, bounty program (110->150 in 1.5 months)
- OTel: 200+ community components, gen_ai.* semconv >=1.37
- VS Code: Tens of thousands extensions, Extension Host isolation
- Backstage: 3,400+ orgs, 89% developer portal market share
- Bevy: Plugin trait, function-as-plugin blanket impl
- Supabase: 4.5M developers, $5B Series E (Oct 2025), Launch Week cadence
- GitHub Octoverse 2025: 1.4M first-time contributors, 180M+ developers
- JetBrains survey 2026: Claude Code most-used, 91% CSAT
- OpenView 2022: 2-3x retention from personal outreach
- Calefato et al. 2022: time-to-first-response as #1 contributor retention lever
- crates.io: Trusted publishing July 2025 (RFC 3691)
