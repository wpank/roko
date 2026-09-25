## Repository signals

- **Product purpose** – The repository defines *Roko* as a Rust toolkit for building self‑modifying agents that automate code changes (see `README.md` lines 3‑5).  This signals a high‑level domain of AI‑driven development.
- **Quick‑start entry point** – The README provides a three‑command onboarding flow (`cargo install`, `roko init`, `roko run`) that is intended to get a new user from zero to a running task in a single terminal session (see `README.md` 70‑76).  This is the primary signal for first‑time users.
- **Workspace composition** – The workspace manifest (`Cargo.toml`) declares a large multi‑crate workspace with a clear separation between core, runtime, gate, plugin, agent, and application crates (see `Cargo.toml` 4‑50).  The presence of many domain‑specific apps (e.g., `mirage-rs`, `roko-chain-watcher`) signals a broad ecosystem.
- **Architectural map** – The crate‑map document (`docs/v1/00-architecture/15-crate-map.md`) enumerates current crate responsibilities and a target modularisation plan (see sections *Current Workspace Shape* and *Target Kernel and Fabric Crates*).  It signals both the current state and an intended future boundary discipline.
- **Evolutionary intent** – The crate‑map explicitly calls out “target” crates that are not yet present (e.g., `roko‑bus`, `roko‑hdc`, `roko‑spi`) and recommends splits of existing crates (`roko‑std` → `roko‑defaults` + `roko‑tools`).  This is a design‑level signal that the codebase is in transition.

## User journey

1. **Installation** – A contributor follows the quick‑start snippet in the README (`cargo install --path crates/roko-cli`) to obtain the `roko` binary.  This step relies on the workspace being buildable with the default toolchain.
2. **Project initialization** – Running `roko init <my‑project>` auto‑detects the target language, creates a `roko.toml` configuration, and sets up gate commands for compilation/linting (see README line 76).  This provides an immediate, opinionated scaffold.
3. **Task execution** – The user invokes `roko run "<natural‑language instruction>"`.  Under the hood the command composes a prompt, dispatches an LLM agent, and passes the generated code through the gate layer before persisting the result (README lines 84‑89).  The user sees a single‑command feedback loop.
4. **Exploring the workspace** – After the initial task, a contributor may explore the multi‑crate workspace (via `Cargo.toml` and the crate‑map) to understand where core contracts (`roko‑core`), runtime (`roko‑runtime`), and higher‑level applications (`roko‑cli`, `roko‑serve`) live.  The crate‑map serves as the navigation guide for deeper contributions.
5. **Advanced workflows** – For larger efforts, the README outlines a multi‑step PRD workflow (capture idea, research, draft, plan, execute, resume, watch) (lines 94‑115).  This pathway guides contributors from high‑level feature ideation to concrete implementation plans.

## Risks

1. **Complexity of the crate graph** – The workspace contains over 30 members, many of which are interdependent (see `Cargo.toml` 4‑50).  New contributors may be overwhelmed by the sheer number of crates and lack a concise map of runtime vs. kernel responsibilities, leading to onboarding friction.
2. **Mismatch between current and target architecture** – The crate‑map describes *target* crates that are not yet present (e.g., `roko‑bus`, `roko‑hdc`).  Contributors reading the map may assume those crates exist, causing confusion when build failures occur because the dependencies are missing.
3. **Insufficient documentation of internal workflows** – While the README provides the quick‑start commands, deeper processes such as the gate system, signal persistence, and multi‑stage PRD pipeline are only briefly mentioned.  Without detailed guidance, newcomers may struggle to understand how the verification gates interact with the agent loop, increasing the risk of misuse or broken pipelines.
4. **Tooling and environment assumptions** – The quick‑start assumes a working Rust toolchain, cargo, and optional language‑specific tools (e.g., `tsc` for TypeScript).  New contributors on non‑Rust platforms may encounter hidden prerequisites that are not explicitly documented, raising the likelihood of early failures.

*All observations are directly drawn from the repository sources (`README.md`, `Cargo.toml`, and `docs/v1/00-architecture/15-crate-map.md`).  Recommendations are distinguished by the “Risk” bullet points, which highlight areas needing further clarification or mitigation.*
