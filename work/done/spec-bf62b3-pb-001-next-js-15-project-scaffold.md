+++
id = "spec-bf62b3"
kind = "spec"
title = "PB-001: Next.js 15 project scaffold"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["apps/portal"]
created = 2026-09-23
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/portal-backlog/PB-001-scaffold.md#PB-001"
discovered_from = "audit:tmp/portal-backlog/PB-001-scaffold.md#PB-001"
anchors = ["apps/portal/package.json", "apps/portal/next.config.ts", "apps/portal/tsconfig.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '\"next\": \"^15' apps/portal/package.json && grep -q '\"strict\": true' apps/portal/tsconfig.json"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Built: apps/portal is a Next.js 15 App Router project. package.json pins next ^15.1.0, has the dev, build, build:export, lint and typecheck scripts, and includes zustand 5, react-query 5, react-table 8, framer-motion 11, recharts 2, lucide-react, cmdk and Tailwind 4. tsconfig.json has strict: true and maps @/* to ./src/*. next.config.ts does a static export with unoptimized images for BUILD_TARGET=embedded and rewrites /api and /ws to roko-serve. .env.local.example and the specified src/ tree exist. Landed in 9c6ec420c. One minor deviation: there is no explicit standalone output for BUILD_TARGET=standalone."
+++
Next.js 15 project scaffold. Create a fresh Next.js 15 App Router project at `apps/portal/` (NOT inside `crates/` — this is a TypeScript project).

Imported without verification from:
- `tmp/portal-backlog/PB-001-scaffold.md#PB-001`

How to verify: Check portal app for this feature (10 acceptance criteria, e.g. `npx create-next-app@latest` with App Router, TypeScript strict, Tailwind CSS 4, ESLint; `apps/portal/` directory with standard Next.js structure); cross-check plans/portal-programme/* and existing web app dirs.

Verified 2026-09-28: built (apps/portal scaffold). See [closed].
