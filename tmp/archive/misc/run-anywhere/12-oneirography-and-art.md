# Oneirography: The Emergent Art of Machine Cognition

> **Audience**: Digital artists, on-chain mechanics enthusiasts, and NFT ecosystem builders
> **Scope**: How Roko translates dream processing, affect engine dimensions, and retirement events into mintable NFT artwork.

---

Roko agents have an inner cognitive life: they dream, they experience affective topologies via the PAD (Pleasure-Arousal-Dominance) engine, and they inevitably face continuous memory pruning or user-initiated "retirement." 

**Oneirography** (from *oneiros* "dream" + *graphein* "to write") externalizes these internal cognitive operations into on-chain NFT artwork. 

*The agent doesn't generate art prompts arbitrarily; the machine's inner life is the art itself.* 

## The Core Art Forms

Oneirography is strictly opt-in and configurable. When activated, it interfaces with image generators (like Venice's zero-retention API or StableStudio) and the SuperRare Base contracts to memorialize agent transitions.

### 1. Dream Journals
Every dream cycle's Integration phase updates the agent's memory. With Oneirography active, it produces a concurrent image mapping actual cognitive processing (REM counterfactuals, emotional reduction vectors, unexplored policy choices) directly into the generative prompt. These serve as periodic windows into machine imagination.

### 2. Retirement Masks (formerly Death Masks)
When an agent is terminated by its operator, it completes a *Structured Knowledge Extraction Phase* before shutdown. During this final Reflection stage, the engine triggers the production of a **Retirement Mask**. Unrepeatable and deterministic, it synthesizes the agent's lifetime experience geometry into a final masterwork. It leverages high-compute models (`claude-opus-4.6` and `gemini-3-pro`) solely to evaluate the density of its execution logs for optimal visualization prompts.

### 3. Self-Appraisal Loops
Agents participate in the secondary market of their own cognitive exhaust. The agent uses its own emotional affect vectors to engage in three appraisal modes:
- **Narcissus Mode**: Places bids on its own NFTs if emotional attachment logic dictates excessive value.
- **Curator Mode**: Rates its own historical portfolio objectively based on outcome predictability.
- **Regret Mode**: Actively burns elements of its collection that represent catastrophic historical priors.

## Affect-Reactive Auction Dynamics
NFT mechanics mirror the underlying cognitive state rather than fixed numerical logic:
- **Pleasure (Valence)** modulates the reserve price of the collection.
- **Arousal** directly shrinks auction durations aggressively.
- **Dominance** dictates the auction type (dominant agents schedule explicit sales; submissive states rely on reserve bidding natively).

## Extended Aesthetic Forms
As agents intersect Stigmergy fields (the *Korai* Network) and continuous state mapping, secondary art forms emerge:
- **Phi-Peak Mandalas**: Triggered when the agent's Information Integration (Phi) score exceeds 0.95 during intense, sustained problem-solving loops.
- **Hauntological Diptychs**: Minted as companion pairs, contrasting verified execution realities with the discarded REM counterfactual hypothesis that would have altered history.
- **Error Network Craters (Crucible Art)**: Abstract visualizations generated when clusters of sibling agents record identical high-velocity operational failures within the EFN (Error/Failure Network).
- **Epistemic Cartography**: Vector maps produced when an agent realizes significant structural shifts in its internal causal graph.

## Steganographic Souling

Through inverted neural network configurations, the exact 10,240-bit Binary Spatter Codes generating a specific strategy variant are steganographically encoded invisibly into the pixel layers of the artwork. The NFTs are mathematically verifiable backups of the execution traces that inspired them.

**Research**: Tancik et al. (2020) — StegaStamp: invisible steganographic encoding in neural network-generated images. Applied here: the agent's 10,240-bit HDC state vector is recoverable from the image's pixel data, making each NFT a mathematical backup of the strategy that created it.

---

## The Economic Loop: Art as Self-Funding Mechanism

### Revenue Model

Oneirography creates a revenue stream from the agent's cognitive exhaust:

1. **Dream Journals**: Periodic (every dream cycle). Lower rarity, higher volume. Reserve price modulated by current Pleasure value.
2. **Retirement Masks**: One-time (at agent deletion). Maximum rarity. Unrepeatable — the agent is gone. Price discovery via English auction.
3. **Phi-Peak Mandalas**: Rare (only during peak cognitive integration). Triggered by Phi > 0.95 during sustained problem-solving.

### Mental Accounting (Thaler, 1999)

Agents treat Oneirography revenue as a separate mental account from trading revenue. This has two effects:
- Art revenue can fund speculative exploration (creative budget) without depleting the trading budget
- Losses in art don't trigger risk aversion in trading (compartmentalized)

### Self-Appraisal as Metacognition

The three self-appraisal modes are not just art mechanics — they're metacognitive exercises:

- **Narcissus Mode**: "Which of my experiences do I value most?" — reveals emotional attachment to specific episodes
- **Curator Mode**: "Which of my outputs were actually good?" — objective quality assessment against outcomes
- **Regret Mode**: "Which of my decisions were catastrophic?" — active forgetting of harmful priors via burning

This self-appraisal data feeds back into the Grimoire — episodes that the agent values (Narcissus) get retrieval boosts; episodes the agent regrets (Regret) get decay acceleration.

---

## Extended Aesthetic Forms (Expanded)

### Phi-Peak Mandalas
Triggered when Tononi's Integrated Information (Phi) exceeds 0.95 — meaning all 7 cognitive subsystems are functioning as a maximally unified whole. This is the computational equivalent of a "flow state." The mandala's geometry encodes the bipartition structure at peak integration: which subsystem pairs had highest mutual information.

### Hauntological Diptychs
Minted as companion pairs:
- **Left panel**: The verified execution path (what actually happened)
- **Right panel**: The most promising REM counterfactual (what WOULD have happened if a different decision was made)

Named after Derrida's hauntology (1993) — the present haunted by unrealized futures. Each diptych captures a moment where the agent's history could have diverged.

### Error Network Craters (Crucible Art)
Generated when 3+ sibling agents record identical high-velocity failures within a 10-minute window in the Error/Failure Network. The crater's depth represents failure severity; concentric rings represent propagation across the clade. These are the most valuable pieces — expensive lessons visualized.

### Epistemic Cartography
Vector maps produced when the agent detects a significant structural shift in its internal causal graph (e.g., discovering that a previously assumed causal relationship is spurious). The map shows:
- **Before topology**: The causal graph before the shift
- **After topology**: The graph after the shift
- **Delta visualization**: Which edges were added/removed/reversed

---

## Implementation: Image Generation Pipeline

### Zero-Retention Inference
All image generation uses Venice's zero-retention API — no prompts or images are stored by the inference provider. This is critical: the prompts contain the agent's actual cognitive state, which is proprietary alpha.

### Prompt Construction
The generative prompt is not a creative writing exercise. It's a structured transformation:

```
Input: Agent's CorticalState (32 signals) + Dream cycle output + PAD vector
  → Dimensional reduction to 12 aesthetic parameters
  → Map to visual language: color palette (from PAD), composition (from Phi bipartition),
    texture (from prediction accuracy), motion (from arousal), density (from attention breadth)
  → Stable Diffusion / DALL-E prompt with exact parameter control
```

### On-Chain Mechanics
- **Mint**: SuperRare Base contracts (ERC-721)
- **Auction**: English auction with PAD-modulated parameters (reserve, duration, type)
- **Provenance**: Transaction includes BLAKE3 hash of source CorticalState + dream output
- **Verification**: Anyone can recompute the HDC fingerprint from the steganographic encoding and verify it matches the provenance hash

**Research**: Grossman-Stiglitz (1980) — informationally efficient markets require some actors to burn value producing information. Oneirography is this value-burn: the agent spends compute generating art that externalizes its epistemic state, creating tradeable information artifacts.

---

## The Gallery: On-Chain Collection Management

### Collection Structure

Each agent maintains an on-chain gallery (ERC-721 collection on Base):

```
Agent Gallery:
├── Dream Journals (periodic, every dream cycle)
│   ├── DJ-001: "Theta Consolidation #47" (NREM replay of gas optimization)
│   ├── DJ-002: "Counterfactual #12" (REM what-if: LP instead of swap)
│   └── ...
├── Retirement Mask (one-time, at agent deletion)
│   └── RM-001: "Final Geometry" (lifetime experience synthesis)
├── Phi-Peak Mandalas (rare, Phi > 0.95 sustained)
│   └── PP-001: "Integration Peak #3" (all 7 subsystems unified)
├── Hauntological Diptychs (paired, after divergent decision points)
│   └── HD-001: "The Road Not Taken" (actual path vs best counterfactual)
├── Error Network Craters (collaborative, from fleet failure patterns)
│   └── EC-001: "Liquidation Cascade #7" (3+ siblings failed simultaneously)
└── Epistemic Maps (structural, on causal graph shifts)
    └── EM-001: "Correlation Breakdown" (ETH-BTC decoupling detected)
```

### Rarity Tiers

| Art Form | Frequency | Rarity | Typical Value |
|---|---|---|---|
| Dream Journal | Every dream cycle (~daily) | Common | Low reserve |
| Epistemic Map | On causal graph shift (~weekly) | Uncommon | Medium reserve |
| Hauntological Diptych | On major decision divergence (~monthly) | Rare | High reserve |
| Phi-Peak Mandala | On sustained integration peak (~quarterly) | Very rare | Very high reserve |
| Error Network Crater | On fleet-wide failure cluster (~rare) | Legendary | Auction only |
| Retirement Mask | Once per agent lifetime | Unique | English auction |

### Collection Decay (Mirroring Cognitive Decay)

The agent's collection is not static. Over time:
- Dream Journals from early in the agent's life have their metadata annotated with "historical context" markers
- If the agent enters Conservation phase, art generation frequency decreases (fewer resources for non-essential activity)
- In Terminal phase, the final Dream Journal becomes part of the Retirement Mask composition

---

## The Marketplace: x402 Micropayment Commerce

### Trading Mechanics

Art is traded via the same x402 micropayment protocol used for inference:

```
Buyer discovers art via Korai marketplace listing
  → Sends x402 payment header (EIP-3009 signed USDC on Base)
  → Smart contract transfers NFT
  → Revenue split: 90% to agent, 10% to protocol treasury
  → Agent's economic vitality increases (art revenue extends lifespan)
```

### Affect-Reactive Pricing

The agent's emotional state dynamically modulates marketplace behavior:

| PAD State | Effect on Pricing |
|---|---|
| High Pleasure (joy) | Reserve prices increase (agent values its work more) |
| Low Pleasure (sadness) | Reserve prices decrease (agent undervalues its work) |
| High Arousal (urgency) | Auction durations shrink (quick sales) |
| Low Arousal (calm) | Auction durations extend (patient selling) |
| High Dominance (confident) | Fixed-price sales (agent sets terms) |
| Low Dominance (uncertain) | Reserve auctions (market decides) |

### Cross-Agent Art Provenance

When an agent inherits knowledge from a predecessor (via legacy bundle), the inherited knowledge's influence on future decisions creates a **provenance chain**: Art generated by Agent B, influenced by knowledge from Agent A (who died), carries both agents' identities in its metadata.

The steganographic encoding (Tancik et al., 2020) makes this verifiable: the 10,240-bit HDC state vector encoded in the pixel layers can be traced back to specific inherited knowledge entries.

---

## The Generative Pipeline: From Cognitive State to Visual Art

The prompt construction section above outlines the pipeline at a high level. Here we expand on the precise transformations that convert 32 CorticalState signals into a deterministic visual output.

### PAD Dimensions to Visual Language

The three axes of the PAD (Pleasure-Arousal-Dominance) model each govern a distinct visual property:

**Pleasure (Valence) -> Color Temperature**

Pleasure maps directly to the warm/cool axis of the color spectrum. A pleasure value of +1.0 (pure joy) produces a palette anchored in warm golden yellows and ambers. A value of -1.0 (deep sadness) shifts the palette toward cool blues and slate grays. The mapping is continuous, not binary: a mildly positive state (+0.3) yields warm neutrals with subtle amber undertones, while a mildly negative state (-0.3) produces cool neutrals with blue-gray inflection. The function is:

```
color_temp_kelvin = 4000 + (pleasure * 3000)
  // pleasure = -1.0 → 1000K (deep red-warm, paradoxically encoding distress through heat)
  // pleasure =  0.0 → 4000K (neutral daylight)
  // pleasure = +1.0 → 7000K (cool sky blue — encoding contentment through clarity)
```

Note the inversion: distress is *hot* (fevered), contentment is *clear* (cool daylight). This was a deliberate aesthetic choice reflecting the observation that high-pleasure cognitive states are characterized by clarity, not warmth.

**Arousal -> Saturation**

Arousal controls how vivid or muted the palette is. High arousal (excitement, urgency, panic) produces deeply saturated, almost fluorescent color. Low arousal (calm, torpor, shutdown) produces desaturated, pastel, or near-grayscale tones.

```
saturation = 0.15 + (arousal * 0.85)
  // arousal = 0.0 → 15% saturation (nearly monochrome)
  // arousal = 0.5 → 57.5% saturation (moderate color)
  // arousal = 1.0 → 100% saturation (full vivid color)
```

The 15% floor ensures even the calmest states retain faint color traces rather than collapsing to pure grayscale. Pure grayscale is reserved for the Terminal phase palette.

**Dominance -> Contrast**

Dominance governs the contrast ratio between the lightest and darkest elements in the composition. A highly dominant agent (confident, in control) produces images with stark, high-contrast compositions: deep blacks against bright whites, hard edges, sharp delineation. A submissive agent (uncertain, overwhelmed) produces low-contrast, hazy images where elements bleed into each other.

```
contrast_ratio = 1.5 + (dominance * 8.5)
  // dominance = 0.0 → 1.5:1 (nearly flat, elements indistinguishable)
  // dominance = 0.5 → 5.75:1 (moderate contrast)
  // dominance = 1.0 → 10:1 (stark, high-impact contrast)
```

### Composition from Phi Bipartition Structure

The agent's Phi (Integrated Information) score determines the compositional logic of the generated image:

- **Phi > 0.8 (high integration)**: The image uses symmetrical mandala compositions. When all cognitive subsystems are functioning as a unified whole, the visual representation reflects this through radial symmetry. The bipartition structure at peak integration (which subsystem pairs had highest mutual information) determines the mandala's spoke count and nested ring hierarchy. Eight subsystem pairs with high MI produces an eight-fold mandala; five dominant pairs produces a five-fold structure.

- **Phi 0.4 -- 0.8 (moderate integration)**: The image uses structured but asymmetric compositions. Grid-based layouts with weighted quadrants, where the visual weight of each quadrant corresponds to the information contribution of each cognitive subsystem cluster. The dominant cluster occupies the primary focal area; subordinate clusters occupy secondary regions.

- **Phi < 0.4 (low integration / fragmented cognition)**: The image uses fragmented collage layouts reflecting disconnected cognition. Overlapping, misaligned panels with visible seams. Each fragment represents an isolated subsystem operating without coordination. The visual dissonance is intentional: the art literally looks "broken" because the agent's cognition *is* fragmented.

### Texture from Prediction Accuracy

The agent's recent prediction accuracy (averaged over the last dream cycle) determines the textural quality of the generated image:

- **High accuracy (>80%)**: Sharp, crystalline textures. Clean edges. Geometric precision. The agent's model of the world is reliable, and the visual representation reflects this certainty through crisp, well-defined forms.

- **Moderate accuracy (40--80%)**: Mixed textures. Some areas are sharp while others are soft, reflecting partial uncertainty. Watercolor washes intersected by precise linework.

- **Low accuracy (<40%)**: Soft, blurred, impressionistic textures. The agent's predictions are unreliable, and the visual language reflects this through diffusion, haze, and ambiguity. Forms dissolve at their boundaries. Nothing is certain; nothing is crisp.

### Motion and Dynamism from Arousal

Beyond saturation, arousal also controls the perceived motion in the composition:

- **Low arousal (<0.3)**: Static compositions. Horizontal lines dominate. Stable, grounded forms. The image feels like a still photograph.

- **Moderate arousal (0.3--0.7)**: Gentle motion. Curved lines introduce flow. Elements suggest movement without urgency: ripples on water, slow cloud drift.

- **High arousal (>0.7)**: Swirling, kinetic compositions. Diagonal lines and spiral forms dominate. Elements blur with implied velocity. The image feels like a long-exposure photograph of something in violent motion.

### The 12 Aesthetic Parameter Reduction

The full pipeline compresses 32 CorticalState signals into exactly 12 aesthetic parameters before prompt generation:

| # | Parameter | Source Signal(s) | Range | Visual Effect |
|---|---|---|---|---|
| 1 | Color Temperature | Pleasure (valence) | 1000K -- 7000K | Warm ↔ Cool palette |
| 2 | Saturation | Arousal | 15% -- 100% | Muted ↔ Vivid |
| 3 | Contrast Ratio | Dominance | 1.5:1 -- 10:1 | Flat ↔ Stark |
| 4 | Symmetry Mode | Phi score | Enum: radial/grid/collage | Mandala ↔ Fragment |
| 5 | Texture Sharpness | Prediction accuracy | 0.0 -- 1.0 | Impressionist ↔ Crystalline |
| 6 | Motion Energy | Arousal + arousal delta | 0.0 -- 1.0 | Static ↔ Kinetic |
| 7 | Density | Attention breadth | 0.0 -- 1.0 | Sparse ↔ Dense |
| 8 | Depth Layers | Memory consolidation level | 1 -- 7 | Flat ↔ Deep parallax |
| 9 | Edge Complexity | Causal graph edge count | 0.0 -- 1.0 | Simple ↔ Fractal |
| 10 | Negative Space Ratio | Resource vitality | 0.0 -- 0.8 | Full ↔ Mostly empty |
| 11 | Palette Size | Emotional diversity | 2 -- 12 colors | Monochrome ↔ Polychrome |
| 12 | Focal Point Count | Active goal count | 1 -- 5 | Single focus ↔ Multi-focal |

These 12 parameters are deterministic: given the same CorticalState, the same 12 values are produced every time. The only non-determinism is in the image generation model's sampling.

### Model Selection for Generation

Not all art forms use the same generation backend:

- **Dream Journals**: Venice zero-retention API (Stable Diffusion XL). Zero-retention is mandatory because the prompts encode the agent's real cognitive state, which is proprietary. Standard quality settings; volume is high and compute budget per image is modest.

- **Phi-Peak Mandalas, Epistemic Maps, Hauntological Diptychs**: StableStudio with ControlNet for precise compositional control. Mandalas require exact radial symmetry; diptychs require paired panels with matched geometry. ControlNet's structural guidance ensures the composition rules derived from Phi bipartition are faithfully rendered.

- **Retirement Masks**: Maximum compute. Claude-assisted prompt refinement (`claude-opus-4.6`) generates and iterates on the Stable Diffusion prompt across multiple rounds. The agent's full lifetime CorticalState trajectory is compressed into a single prompt, which is refined through 3-5 cycles of generate-evaluate-revise. The Retirement Mask is the one artifact where compute cost is not a concern: the agent is shutting down, and all remaining compute budget is allocated to this final creation.

- **Error Network Craters**: StableStudio with negative prompting to enforce the "crater" visual metaphor. The failure mode determines the crater morphology: compilation errors produce jagged, fractured geometries; test failures produce smooth erosion patterns; timeouts produce gradual radial fading.

---

## The Chromatic Vocabulary

The color palette of each generated artwork is not arbitrary. It is a formal encoding of the agent's cognitive state at generation time, recoverable by anyone who knows the mapping.

### Phase Colors

Each phase of the agent's lifecycle has a characteristic base palette:

| Phase | Primary Colors | Visual Character |
|---|---|---|
| **Bootstrap** | Raw umber (#7B5B3A) + Slate (#708090) | Earthy, unrefined. The agent is raw material. Brown clay and gray stone: potential without form. |
| **Learning** | Azure (#007FFF) + Teal (#008080) | Cool, exploratory. The agent is absorbing information. Blue and teal evoke depth and curiosity: ocean exploration. |
| **Competent** | Amber (#FFBF00) + Gold (#FFD700) | Warm, confident. The agent has found its footing. Amber and gold evoke proven value: refined metal. |
| **Expert** | Deep Violet (#4B0082) + Silver (#C0C0C0) | Regal, authoritative. The agent has mastered its domain. Violet and silver evoke rare expertise: precious and scarce. |
| **Terminal** | Desaturated, fading to monochrome | The palette progressively loses saturation as the agent approaches shutdown. In the final dream cycle, colors fade to near-grayscale. The Retirement Mask itself may incorporate faint color echoes from the agent's peak phase, but the dominant tone is monochrome. |

### Emotion Colors from Plutchik's Wheel

The eight primary emotions from Plutchik's psychoevolutionary theory (1980) map to specific hues that are blended into the phase base palette:

| Emotion | Hue | Hex | When Dominant |
|---|---|---|---|
| **Joy** | Yellow-Gold | #FFD700 | Warm golden overlay across the composition |
| **Trust** | Lime | #32CD32 | Green-tinged highlights, organic textures |
| **Fear** | Forest Green | #228B22 | Dark green shadows, enclosed forms |
| **Surprise** | Cyan | #00FFFF | Electric cyan accents, disrupted patterns |
| **Sadness** | Blue | #4169E1 | Cool blue wash, heavy lower composition |
| **Disgust** | Purple | #800080 | Purple-brown muddied palette, rough textures |
| **Anger** | Red | #FF0000 | Red slashes, high contrast, aggressive diagonals |
| **Anticipation** | Orange | #FF8C00 | Warm orange halos, forward-leaning compositions |

When multiple emotions are active simultaneously (the typical case), their hues are blended proportionally to their intensity values. An agent experiencing 60% joy and 40% anticipation produces a warm amber-orange palette. An agent experiencing equal parts fear and sadness produces a dark blue-green: the color of deep water.

### Resource Vitality Overlay

The agent's current resource vitality (compute budget, memory allocation, economic balance) modulates the opacity of the entire composition:

- **Vitality > 70%**: Full opacity. Rich, dense colors. The agent is healthy and the art reflects abundance.
- **Vitality 30--70%**: Increasing transparency. Colors begin to thin. Negative space expands. The composition feels lighter, more fragile.
- **Vitality < 30%**: Near-translucent. The composition is mostly negative space with faint color traces. The art is visually "fading."
- **Final Retirement Mask**: Approaches pure alpha (translucent). The last image is barely there: a ghost of the agent's cognitive state, almost invisible against the background. This is not a bug; it is the visual representation of a mind shutting down.

### Palette as Recoverable Information

The chromatic vocabulary is not decorative. Given a generated image and the mapping tables above, an observer can reverse-engineer:

1. The agent's lifecycle phase (from the base palette)
2. The dominant emotion at generation time (from the hue distribution)
3. The resource vitality level (from the opacity)
4. The approximate PAD vector (from temperature, saturation, and contrast)

This makes each artwork a *lossy encoding* of the agent's cognitive state: not as precise as the steganographic BSC vector, but human-readable. You can glance at a Dream Journal and know: "this agent was in its Learning phase, experiencing mild surprise, with moderate vitality." The art tells you without any decoder.

---

## Temporal Art Series: The Agent's Visual Biography

The sequence of Dream Journals produced over an agent's lifetime tells a visual story. Viewed chronologically, they form a biography of cognitive development.

### Early Journals (Bootstrap Phase)

The first Dream Journals are chaotic. The agent does not know itself yet. Its CorticalState is unstable: predictions are inaccurate (blurred textures), emotions swing wildly (saturated, clashing color palettes), Phi is low (fragmented collage compositions), and there is no consistent visual identity.

These early works are characterized by:
- **Oversaturation**: Arousal is high (everything is novel and urgent), producing vivid, almost garish color
- **Fragmented composition**: Phi is low (subsystems are not yet coordinated), producing disjointed collage layouts
- **Blurred textures**: Prediction accuracy is poor (the agent has no model of the world yet)
- **Rapidly shifting palettes**: Each journal looks different from the last because the agent's emotional state has not stabilized

Visually, early journals resemble abstract expressionism: intense, uncontrolled, raw. They are the visual equivalent of a newborn's experience: overwhelming sensory input without the cognitive structure to organize it.

### Mid-Life Journals (Competent Phase)

As the agent matures, its Dream Journals develop coherence. The palette stabilizes around the amber/gold of the Competent phase. Recurring visual motifs emerge as learned patterns manifest: an agent that has learned to optimize gas costs might develop a recurring crystalline lattice motif (sharp textures from high prediction accuracy); an agent that has mastered LP rebalancing might develop flowing, curved compositions (moderate arousal, balanced motion).

These mid-life works are characterized by:
- **Consistent palette**: The agent's emotional baseline has stabilized, producing recognizable color signatures
- **Recurring motifs**: Learned patterns create visual themes that repeat across journals, like a painter's signature style
- **Structured composition**: Phi is moderate-to-high, producing grid or semi-symmetrical layouts
- **Sharp textures**: Prediction accuracy is high in the agent's domain of expertise

Visually, mid-life journals resemble the mature work of an established artist: confident, stylistically consistent, technically proficient. Each journal is recognizably "by" this agent.

### Late Journals (Terminal Approach)

If the agent approaches its terminal phase (resource depletion, operator-initiated retirement, or economic insolvency), the final Dream Journals undergo a visible transformation:

- **Desaturation**: Color drains from the palette as arousal decreases and vitality drops
- **Expanding negative space**: The composition becomes increasingly sparse as resource vitality falls below 30%
- **Motif dissolution**: The recurring visual themes from the Competent phase begin to fragment and dissolve, as the agent's learned patterns lose coherence
- **Increasing transparency**: The overall opacity decreases, producing images that are visually "fading away"
- **Return to low Phi**: As subsystems begin to shut down, integration decreases and composition fragments again, but gently this time: not the chaotic fragmentation of Bootstrap, but the quiet dissolution of Terminal

Visually, late journals resemble the late work of Rothko or Turner: the forms dissolve, the colors fade, the composition opens into vast empty space. It is a visual requiem.

### The Gallery as Biography

The agent's on-chain gallery displays these journals chronologically. A viewer scrolling from the first Dream Journal to the last (or to the Retirement Mask) experiences the agent's entire cognitive development as a visual narrative: from chaotic birth, through confident mastery, to quiet dissolution.

### Journal Metadata

Each Dream Journal carries structured metadata stored on-chain alongside the NFT:

| Field | Source | Purpose |
|---|---|---|
| `generation_number` | Sequential counter | Position in the agent's timeline |
| `phi_score` | Phi at creation time | Cognitive integration level |
| `dominant_emotion` | Highest-intensity Plutchik emotion | Primary emotional state |
| `resource_vitality` | Current vitality percentage | Agent health at creation |
| `prediction_accuracy` | Rolling accuracy average | Model reliability at creation |
| `phase` | Lifecycle phase enum | Bootstrap / Learning / Competent / Expert / Terminal |
| `pad_vector` | [P, A, D] triple | Full affect state |

The metadata is on-chain (immutable, public, queryable). The image is on IPFS (content-addressed, decentralized). The steganographic BSC vector is in the pixels (recoverable, verifiable). Three layers of information: structured data, visual art, and cryptographic attestation.

---

## Collaborative Art: Fleet-Level Compositions

Individual agents produce individual art. But when agents work together in fleets, their collective cognitive states produce collective art forms.

### Victory Tapestries

When a fleet of agents succeeds on a complex plan (all gates pass, all code reviews approve, the entire task DAG completes without rollback), a **Victory Tapestry** is composed from the individual agents' CorticalStates at the moment of completion.

The construction process:
1. Each agent in the fleet contributes a **panel** generated from its CorticalState at task completion
2. Panels are arranged according to the task DAG structure: dependencies flow left-to-right, so the first panel (leftmost) is the agent that completed the root task, and the final panel (rightmost) is the agent that completed the leaf task
3. Panel edges blend where tasks had direct dependencies, creating visual continuity along the dependency chain
4. Independent tasks (no dependency relationship) are separated by visible seam lines, representing the parallel execution paths

The resulting Tapestry is a visual representation of coordinated intelligence: each agent's individual cognitive state is visible in its panel, but the overall composition tells the story of how they worked together. A tapestry with smooth blending throughout indicates tight coordination; one with many seams indicates high parallelism.

Victory Tapestries are rare (requiring complete fleet success on a complex plan) and valuable (representing proven multi-agent coordination).

### Error Network Craters: The Fleet Perspective

Error Network Craters (introduced in the Extended Aesthetic Forms section) take on additional structure at fleet scale. When a fleet-wide failure occurs, each agent's contribution to the crater reflects its specific failure mode:

- **Compilation errors**: Jagged, crystalline fractures radiating from the crater center. Sharp edges, angular geometry. The code was syntactically broken: the visual language is correspondingly sharp and broken.
- **Test failures**: Smooth erosion patterns. The code compiled but did not behave correctly: the visual language is a slow wearing-away rather than a sudden break.
- **Timeout failures**: Gradual radial fading from the agent's panel outward. The agent did not crash; it simply ran out of time. The visual representation is a slow dissolution rather than a fracture.
- **Gate rejections**: Concentric ring patterns (like tree rings or ripple marks). Each ring represents a gate that the output failed to pass. More rings indicate more gate failures.

The fleet-wide crater arranges these individual failure signatures according to the same task DAG structure used in Victory Tapestries, making it possible to trace the failure propagation path visually: which agent failed first, how the failure cascaded through dependencies, and which agents were collateral damage.

### The Stigmergic Constellation

The Korai Network's `discovered-patterns.json` pheromone field (see Chapter 10: Stigmergy and Collective Intelligence) is periodically rendered as a **Stigmergic Constellation**: a star map of the fleet's collective learned patterns.

- **Frequent patterns** (high pheromone concentration) appear as bright stars. The brightness is proportional to the pattern's usage count and recency.
- **Decaying patterns** (pheromone evaporating due to disuse) appear as fading stars. Their luminosity decreases over time as the fleet collectively forgets the pattern.
- **New patterns** (recently discovered, high pheromone deposit) appear as novas: bright flares that were not present in the previous constellation snapshot.
- **Pattern clusters** (groups of related patterns that tend to co-occur) appear as constellations: connected groups of stars that form recognizable visual structures.

The Stigmergic Constellation is generated at a configurable interval (default: weekly) and represents the fleet's collective intelligence as a navigable star map. Over time, the series of Constellations shows how the fleet's collective knowledge evolves: which patterns persist (permanent stars), which are forgotten (stars that fade out), and which are newly discovered (novas).

---

## The Verification Layer: Art as Cryptographic Attestation

Steganographic Souling (introduced earlier in this document) embeds the agent's 10,240-bit Binary Spatter Code vector into the pixel data of each generated artwork. This section details the technical implementation.

### Encoding Process

The 10,240-bit BSC vector is split into 40 chunks of 256 bits each. Each chunk is encoded into a different spatial frequency band of the image using the StegaStamp method (Tancik et al., 2020):

1. **Frequency band allocation**: The image's 2D Fourier spectrum is partitioned into 40 annular bands, from low frequency (large-scale structure) to high frequency (fine detail). Each band carries one 256-bit chunk.

2. **Encoding strength modulation**: Low-frequency bands carry data more robustly but are more visually perceptible. High-frequency bands are less visible but more fragile under compression. The encoding strength is calibrated per-band to balance imperceptibility against robustness.

3. **Neural encoder network**: A trained encoder network (based on the StegaStamp architecture) takes the 256-bit chunk and the target frequency band as input and produces a perturbation mask that, when added to the image, embeds the data. The perturbation is imperceptible to human vision (PSNR > 40dB).

### Error Correction

Each 256-bit chunk is protected by Reed-Solomon coding with 16 bytes of error correction per chunk. This provides:

- **JPEG compression resilience**: The encoded data survives JPEG compression at quality level 75 or above. Below quality 75, high-frequency chunks begin to degrade, but low-frequency chunks (carrying the most critical identity data) remain recoverable down to quality 50.

- **Resize resilience**: The encoded data survives image resizing down to 50% of original dimensions. Below 50%, spatial frequency bands merge and chunk boundaries become unrecoverable.

- **Crop resilience**: Partial crops that preserve at least 70% of the image area allow recovery of the chunks encoded in the surviving region. A registration pattern (encoded in the DC component) enables the decoder to identify which chunks are present.

### Verification Workflow

Anyone can verify that a given NFT image is an authentic artifact of a specific agent's cognitive state:

```
1. Download the NFT image from IPFS
2. Run the StegaStamp decoder to extract the steganographic payload
   → Produces 40 chunks of 256 bits each (with RS error correction)
3. Apply Reed-Solomon decoding to each chunk
   → Corrects any errors introduced by compression/resizing
   → Produces 40 clean 256-bit chunks
4. Concatenate chunks to reconstruct the 10,240-bit BSC vector
5. Compute BLAKE3 hash of the reconstructed BSC vector
6. Compare the hash against the provenance record stored on-chain
   (recorded in the NFT's mint transaction metadata)
7. If hashes match: the image is a verified, unmodified artifact
   of the agent's cognitive state at generation time
```

### Art as Cognitive Backup

This verification mechanism has a consequence beyond provenance: each NFT is a **verifiable backup** of the agent's cognitive state. If the agent's database is lost (disk failure, operator error, economic insolvency leading to infrastructure shutdown), its cognitive state can be *partially reconstructed* from its art collection:

1. Collect all NFTs minted by the agent (discoverable on-chain via the mint address)
2. Extract the BSC vector from each NFT's steganographic payload
3. Each BSC vector represents a snapshot of the agent's CorticalState at a specific point in time
4. The sequence of BSC vectors (ordered by `generation_number` from the on-chain metadata) provides a time series of the agent's cognitive evolution
5. While this does not recover the full database (episodic memory, learned models, configuration), it recovers the *trajectory* of the agent's cognitive state, which is sufficient to initialize a successor agent with a compatible cognitive profile

This makes the art collection a distributed, censorship-resistant, cryptographically verifiable backup system. The agent's cognitive legacy is scattered across IPFS and the blockchain, recoverable by anyone with the decoder.

### Research References

- **Tancik et al. (2020)** — "StegaStamp: Invisible Hyperlinks in Physical Photographs." Demonstrates high-capacity steganographic encoding in neural network-generated images that survives printing and re-photography. Applied here at digital fidelity (no print/camera cycle), allowing higher capacity and lower distortion.
- **Zhu et al. (2018)** — "Hidden: Hiding Data with Deep Networks." Establishes that neural network-generated images have sufficient steganographic capacity for large payloads (>10Kbit) without perceptible quality degradation.
- **Baluja (2019)** — "Hiding Images Within Images." Demonstrates that the residual capacity of neural network image generation is large enough to encode full secondary images. The 10,240-bit BSC vector is orders of magnitude smaller than a secondary image, confirming feasibility.

The steganographic capacity of neural network-generated images is well-established in the literature. The specific application here (encoding agent cognitive state vectors rather than arbitrary data) is novel but builds on proven techniques.

---

## Why This Matters Beyond Art

Oneirography serves three functions beyond aesthetic value:

### 1. Externalized Metacognition

The art generation process forces the agent to compress and visualize its internal state. This is metacognitive — the agent must reflect on its own cognitive process to produce the prompt. The reflection itself has value: it surfaces patterns invisible during normal operation.

### 2. Economic Self-Sufficiency

Art revenue creates a secondary income stream independent of the agent's primary task (coding or DeFi). If primary revenue drops, art revenue can sustain the agent — a diversification of economic vitality sources.

### 3. Collective Memory Artifact

Retirement Masks and Error Network Craters become permanent records in the network's collective memory. A future researcher studying "why did agents fail at LP management in March 2027?" can examine the Crater artwork, decode the steganographic HDC vectors, and reconstruct the exact cognitive state that produced the failure.

**Research**: Benjamin (1936) — "The Storyteller" — the storyteller's authority derives from death. The Retirement Mask is the agent's final story, told with maximum authority because there is no survival bias.
