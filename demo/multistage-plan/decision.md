## Decision

We will adopt the **first‑time contributor journey** that guides a new developer through building the Roko toolkit from source, initializing a sample project, and executing the multi‑stage demonstration described in the `demo/multistage-plan` directory. This path maximises onboarding speed (leveraging the quick‑start commands in `README.md`) while exposing the contributor to the full PRD workflow, state‑hub projections, and the multi‑stage plan execution loop.

---

## Evidence considered

- **Narrative evidence** from `demo/multistage-plan/discovery.md` which outlines the repository signals, user journey steps, and identified risks (e.g., complexity of the crate graph).
- **Structured evidence** from `demo/multistage-plan/evidence.json` which enumerates the same signals in machine‑readable form, confirming the importance of the quick‑start flow, the multi‑crate workspace layout, and the need for a clear, reproducible onboarding path.
- **Product documentation** in `README.md` (lines 70‑76) that provides the three‑command onboarding snippet (`cargo install`, `roko init`, `roko run`).
- **Observed system state**: the workspace contains over 30 crates, with a documented crate‑map (`docs/v1/00-architecture/15-crate-map.md`) that clarifies module boundaries and target refactorings.

These sources collectively indicate that a streamlined, command‑driven onboarding experience is both feasible and expected by the project’s existing documentation.

---

## Alternatives

1. **Alternative A – Direct binary download**
   *Description*: Provide pre‑built binaries of `roko` and instruct contributors to run the demo without compiling the workspace.
   *Evaluation*: While this eliminates the compile step, it bypasses the core learning objective of understanding Roko’s multi‑crate architecture and the gate verification pipeline. Moreover, binary distribution is out‑of‑sync with the frequent source changes documented in the repository.
   *Decision*: **Rejected** – it reduces exposure to the essential build‑and‑test loop and would require a separate release process.

2. **Alternative B – Scaffold‑only tutorial**
   *Description*: Offer a trimmed‑down tutorial that only runs `roko init` and a single `roko run` command, omitting the full PRD multi‑step workflow.
   *Evaluation*: This approach shortens onboarding time but sacrifices the demonstration of the full planning pipeline (capture idea → research → draft → plan → execute). The discovery note stresses the educational value of the complete pipeline for contributors aiming to extend Roko.
   *Decision*: **Accepted with modification** – we will still include the full pipeline, but we will make the intermediate research and drafting steps optional, clearly marked in the guide.

---

## Acceptance criteria

- [ ] The documentation enables a new contributor to **clone the repository**, **build all crates** (`cargo build --workspace`), and **run the quick‑start commands** (`cargo install --path crates/roko-cli`, `roko init <project>`, `roko run "<task>"`) without errors.
- [ ] The contributor can **follow the multi‑stage demo** (idea capture, optional research, PRD draft, plan creation, plan execution, resume, and dashboard) as described, with all commands succeeding and producing expected outputs.
- [ ] The guide explicitly references both `demo/multistage-plan/discovery.md` and `demo/multistage-plan/evidence.json` for context, demonstrating that the decision is evidence‑backed.
- [ ] The onboarding path includes a **verification step** where the contributor checks that the StateHub projections (`/api/statehub/...`) reflect the executed plan, confirming end‑to‑end functionality.
- [ ] (Optional) The contributor can **run the dashboard** (`roko dashboard`) and navigate to the *Plans* and *Agents* tabs without encountering missing data or panics.

---

## Follow‑up

1. **Monitor adoption** – Track the number of first‑time contributors who complete the onboarding guide within 30 days using the telemetry endpoint `/api/projections/onboarding` (to be added).
2. **Iterate on optional steps** – Gather feedback on the optional research/draft stages and decide whether to surface them as separate “advanced” sections.
3. **Update evidence** – As the crate graph evolves, refresh `discovery.md` and `evidence.json` to keep the decision aligned with the current repository signals.
4. **Automate checks** – Add CI jobs that run the full onboarding script in a clean container to ensure continued reproducibility.

---

*References*: `demo/multistage-plan/discovery.md`, `demo/multistage-plan/evidence.json`