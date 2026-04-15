# Pitch deck design for an a16z infrastructure Series A

The honest answer to your most important question: **Stripe's "original pitch deck" is a myth that didn't exist as a polished artifact, and Cloudflare's Series A deck never existed at all.** Patrick Collison and Matthew Prince both pitched on live product, not slides — every "Stripe Series A deck" circulating online (Slidebean, Zlides, Upmetrics, Karaf.ai) is a third-party reconstruction using post-2018 brand assets. This matters for your strategy: the decks people are imitating aren't real, and the most-revered infrastructure pitches were **distinguished by product credibility, not deck craft**. Your job is to use the deck as a *vehicle for proof*, not as the proof itself. The strongest evidence-backed model in your research targets is **Temporal's $18.75M Series A deck (Sequoia, Oct 2020)**, which Sequoia now uses to train scouts on infrastructure pitches. Below, decision-ready recommendations for each of your 12 sections, organized to match your slide-by-slide build.

## What slide one should actually do

Both Sequoia and YC prescribe the same thing for the title slide: **company name plus a one-sentence declarative purpose**. Sequoia's exact language is *"define your company in a single declarative sentence — this is harder than it looks."* YC's Aaron Harris template says *"Your company name and a succinct description."* Kevin Hale at YC adds the visual rule: large type, bold, top-aligned, legible from the back of a 500-person room.

Then the actual debate: **a dramatic number ("$44.86 → $1.42") versus a thesis statement ("The model is the same. The system is the variable.")**. The 4th & King framework that Stripe officially endorses on stripe.com/atlas catalogs five proven "pitch plots," and a counterintuitive-statistic opener is one of them — but they explicitly warn it's *"a challenging plot to pull off because it requires presenting both a fact your audience doesn't already know AND an insight that feels fresh."* The thesis-statement opener (their "starting over" plot) is more forgiving and is the format Sequoia, YC, Temporal, and Vercel all used.

**Recommendation: lead with the thesis line, not the number.** "The model is the same. The system is the variable" is **a Marc Andreessen-grade aphorism** that Casado will pattern-match to category-creation pitches; "$44.86 → $1.42" is a punchline that lands harder on slide 4 (cost comparison) than on slide 1, where investors haven't yet earned the context to find it shocking. Kevin Hale explicitly **warns against animations and screencasts on the opening slide** ("the frustration I have for screenshots in pitch decks goes for screencasts and videos as well"). A live cost animation on slide 1 violates Hale's rule and pattern-matches to demo-day theatrics. Save the number for the cost-comparison slide where it does maximum work.

## Designing the problem slide

The cross-source consensus is unambiguous: **one giant number beats a chart**. Anthony Miller's Swipefile review puts it bluntly — *"'$100M' and '10X' really stand out and make an impact."* Focused Chaos's review of 50 decks: *"One great stat is better than five forgettable ones."* a16z's own published deck guidance (per InkNarrates' synthesis) says *"VCs invest in problems, not products… use data if you have it, but don't overload."*

For the "AI projects fail" stat specifically, **the most defensible citation is RAND's 80% number** (research report RRA2680-1) over MIT's 95% figure (which is more contested). For chart selection if you decompose the failure: **horizontal bar chart beats donut beats waterfall**. A waterfall is mathematically wrong here — waterfalls show *additive* cumulative change, not category breakdown.

**The cybersecurity playbook is instructive.** CrowdStrike's annual threat reports use a remarkably consistent visceral pattern: a stopwatch graphic with a single dramatic number ("**29 minutes average breakout, fastest 27 seconds**"), triple-digit YoY percentages ("442% increase in vishing"), and named adversaries with mascot icons (FAMOUS CHOLLIMA, CURLY SPIDER). The technique is **time-as-fear plus named villains**. Palo Alto Networks instead pushes "platformization" — counting platformized customers (1,550, +35% YoY) — a category-leader rather than fear-based frame. SentinelOne mimics Gartner Magic Quadrant placement directly. For a Series A pitch, **CrowdStrike's hero-stat-plus-stopwatch is the right model**, not Palo Alto's platform metrics (which require enterprise scale you don't have yet).

## The solution slide as code

The "7 lines of code" mythology deserves clarification: **the snippet was on stripe.com's 2011 landing page, not in a pitch deck**. Stripe's own engineer Michelle Bu wrote in a 2020 retrospective: *"To this day, it's not entirely clear which seven lines the article referenced. The prevailing theory is that it's the roughly seven lines of curl it took to create a Charge."* The Bloomberg Businessweek 2017 cover story — *"How Two Brothers Turned Seven Lines of Code Into a $9.2 Billion Startup"* — created the legend. Patrick Collison himself has called it *"about the feeling of magical simplicity, not a literal claim."*

The actual snippet, reconstructed from the Wayback Machine:

```
curl https://api.stripe.com/v1/charges \
   -u sk_test_REDACTED: \
   -d amount=400 \
   -d currency=usd \
   -d card[number]=4242424242424242 \
   -d card[exp_month]=12 \
   -d card[exp_year]=2013
```

What made it work as a *pitch mechanism* was that **investors and customers could literally run it in a terminal**. The artifact was identical for both audiences. **Recommendation: yes, use a code snippet as your solution slide.** It's a Stripe-canonical move, it telegraphs developer-first design, and it lets Casado mentally execute the call — which is far more memorable than an architecture diagram. Keep it under 10 lines, use a real callable command (not pseudocode), and **show output below it**, not just input.

## Cost comparison and the $44.86 → $1.42 slide

The Snowflake S-1 deliberately avoids "we're cheaper" claims; it leads with **decoupled storage and compute as architecture**, with cost framed as a *consequence* of architecture. AWS case studies follow a tight template: **single hero percentage, before/after architecture diagram, three supporting stats**. They never use waterfalls.

**On the multiplicative waterfall (5x × 3x × 2x = 30x): don't use a literal Excel waterfall — they're additive by design.** What works visually is a **stepped horizontal cascade** where each step labels both the multiplier and the absolute dollar value, with footnoted citations for each factor. This matters because Hunter Walk's heuristic applies: investors mentally discount each multiplier by ~30%, so 30x in your slide becomes ~10x in their head. Two defenses against that discount: **(1) cite each factor independently with a primary-source benchmark** in a footer, and **(2) anchor the headline in the absolute number, not the multiplier**. "$44.86 → $1.42" is more defensible than "30x cheaper" because the reader can mentally check the math.

The Forrester TEI study you mentioned (**201% three-year ROI, 14-month payback, $2.8M NPV**) is **from May 2025, not Temporal's 2020 Series A** — it post-dates the round by 4.5 years and is now sales-enablement, not a Series A artifact. The TEI visual conventions that work: hero ROI percentage at the top in massive type, then 3–5 quantified benefit stat-cards below, with the Forrester logo lockup for credibility transfer. If you have your own TEI-equivalent analysis, format it identically — but flag that **Forrester TEI studies are commissioned**, and sophisticated investors discount them somewhat.

## Competitive positioning without the credibility tax

Hunter Walk (Homebrew) wrote the canonical takedown of vanity 2x2s: *"I've never been presented one of these slides where the startup pitching isn't in the 'upper right' square. If you show me a competitive 2x2, I'm going to ask you what the competitors on the slide would say about it."* Underscore VC's refinement: *"The best 2x2s use the axes to convey what competitors could never do because of their specific advantages — business model, data advantage, product structure."*

The strong cross-source consensus is that **honestly showing competitor strengths builds credibility** — this is one of the few topics in pitch literature with near-universal agreement. Already.dev: *"A 2x2 with you in the top-right doesn't just fail to impress, it actively undermines your credibility."* InkNarrates: *"Ignoring the giant in the room is a sure-fire way to lose investor trust."* The pattern that works: explicitly name each competitor's strength in a small callout, then explain how you route around it. Snowflake's positioning against Redshift and BigQuery did exactly this — **acknowledged AWS/GCP as both partners and competitors** in the S-1, then differentiated on multi-cloud, decoupled storage/compute, and per-second billing.

**Format choice for infrastructure: Harvey-Ball feature table beats 2x2 for technical SaaS, but a 2x2 with non-obvious axes wins if you can defend them.** Mimic Gartner Magic Quadrant when entering an existing category (enterprise buyers think in MQ terms); subvert with petal/Venn diagrams when creating a new category. For Casado specifically, who has spent a decade thinking about infrastructure category structure, **subverting MQ is the right move only if your axes name something genuinely orthogonal to the existing frame** — otherwise a clean Harvey-Ball table reads as more honest.

## Presenting the dual-asset structure

None of the canonical dual-asset companies have public decks, but their public framing language is well-documented and convergent. **The dominant pattern: lead with the legacy industry being disrupted; crypto is the *mechanism*, never the headline.** Story Protocol pitched a16z as "AI × IP law modernization" tokenizing a "$2T IP asset class," not as a crypto play. Helium pitched **telecom disruption first**, then deliberately rebranded to Nova Labs to disambiguate the equity entity from the protocol — Frank Mong told Decrypt the $200M Series D was *"entirely focused on equity, with no tokens involved."* Worldcoin / Tools for Humanity leans on AI inevitability ("proof of personhood for the AGI era"), not crypto merit.

The **three-entity structure is now standard**: for-profit Labs entity (equity) + nonprofit Foundation (governance) + token (separate sale). This lets generalists underwrite the equity layer without taking token risk. The Block reports the institutional default since 2022 is **SAFE plus token warrants**, used by Mysten Labs, Story ($80M Series B), and dYdX. Sequence the rounds the way Helium did: token sale to crypto-natives first (a16z crypto, Multicoin, Polychain), then equity round to generalists (Tiger, Deutsche Telekom). Don't ask one investor to underwrite both forms of risk simultaneously.

**Anti-pattern vocabulary to avoid: "Web3 platform," "tokenomics," "blockchain company," "DeFi."** Replacement vocabulary: "infrastructure for [specific industry]," "incentive design," "[industry] company that uses blockchain," "verifiable compute," "programmable trust." EigenLayer renamed its product to "EigenCloud" — deliberately AWS-style — to frame the addressable market as *all* software. Anchorage leads with its OCC federal bank charter; Fireblocks writes long-form content for BNP Paribas and BNY Mellon about wallet infrastructure. **Customer-logo selection does the heaviest lifting**: regulated/enterprise logos (Goodyear, Deutsche Telekom, BNY Mellon) defeat crypto-native pattern-matching faster than any framing language.

On the **token graveyard slide: appendix-only, titled by the objection**. NextView's published guidance is to title appendix slides literally as the objection they answer ("Why this isn't Terra/FTX/Helium-revenue"). Reference its existence in the main deck with one line ("we've designed around the failure modes that broke prior attempts — see appendix"), then hyperlink. Putting it in the main flow signals defensiveness and burns 90 seconds reminding the investor of the bear case. Qubit's framing is correct: *"A tight 12-slide deck that sparks good questions beats a 25-slide deck that preempts every objection."*

## Compliance as the GTM, not the checkbox

This is the **single most important framing move** in your deck if regulation is a tailwind. Christina Cacioppo's exact words on the Vanta thesis: *"If you want to start a security company, you should think about starting a compliance company. Because compliance and SOC 2 is often a purchase driver, it opens up new markets. So it ends up being this growth accelerant, this driver of revenue in a way that just most security tools aren't."* The reframe is: **compliance ≠ cost center; compliance = revenue unlock**. Vanta sold SOC 2 as the *gate* to enterprise sales, with Vanta as the gate-opener.

OneTrust's exact playbook deserves study because it's the closest precedent for a regulation-driven infrastructure pitch. Kabir Barday's 2019 Series A press-release language: *"This investment will help us bring scale and support, **coming at a timely juncture with just six months before California's CCPA is set to be enforced**."* Insight Partners' Richard Wells reframed the regulation itself: *"Privacy regulations like CCPA and GDPR are a direct **market reaction to consumer demand**."* That inversion — regulation as *symptom* of demand, not exogenous shock — makes the TAM look durable rather than policy-dependent.

The repeating compliance-as-distribution formula across GDPR (OneTrust), SOC 2 (Vanta), and now the EU AI Act (Openlayer, Holistic AI):

| Regulation | Buyer the regulation created | Software winner |
|---|---|---|
| GDPR (May 2018) | Chief Privacy Officer | OneTrust ($4.5B) |
| SOC 2 (de facto) | Compliance lead at startups | Vanta ($4B) |
| EU AI Act (Aug 2025/2026) | AI Governance Lead | Openlayer, Holistic AI |

**On countdown timers: use the *content*, not the widget.** OneTrust's "six months before CCPA enforcement" is the model — specific date, specific penalty (€20M/4% global revenue under GDPR; €35M/7% under EU AI Act), specific buyer role created. Animated countdown UI pattern-matches to ICO/affiliate-marketing and is universally absent from credible VC decks. Specificity equals credibility; a ticking clock graphic equals gimmicky.

## The team slide for a solo founder

Patrick Collison and John raised Stripe's seed from Thiel/Musk on **two people and a YC alumni credential** — the famous "4-person team slide" circulating online with Claire Hughes Johnson is a later reconstruction; she joined years afterward. Guillermo Rauch's Vercel Series A team narrative was effectively *"Guillermo + the Next.js community"*: the team slide leaned on his open-source pedigree (Socket.IO, Mongoose, Next.js, React contributor) plus a customer-logo wall doing double-duty as team validation. Paul Copplestone at Supabase emphasized **OSS-maintainer hires + ex-employer logos** (AWS, Google, Palantir, Stripe).

For a solo technical founder pitching Casado, the convergent best-practice across VIP Graphics, Stage VP, Funding Blueprint, and Waveup is the **"1 founder = 3 proof points" rule**: domain expertise + shipping track record + hiring/community pull. Concrete structure for the slide:

- **Founder block (60% of slide)**: photo, one-line title, three logos of prior companies with role and concrete outcome ("Led X-engineer team that shipped Y to Z scale"), one line on technical credibility (commits, OSS, prior infra shipped).
- **Advisors (25%)**: 2–4 named advisors with **specific contribution** ("Joined seed, intros to 3 design partners" beats "Strategic advisor"). CTOs/VPEng at recognizable companies outweigh other VCs.
- **Hiring plan (15%)**: 2–3 roles tied to milestones ("Founding Engineer — distributed systems, Q1"), never C-suite titles ("CFO, CMO"). Name committed hires by LinkedIn.

**Move this slide to position 4 — right after problem/solution — not slide 11.** Waveup's argument is that for solo founders the team is the primary investible asset, so deferring it 11 slides cedes the most important narrative leverage.

## GTM that reads as repeatable, not aspirational

The infrastructure-VC consensus is overwhelming: at Series A, **logos of design partners beat the playbook**. Sequoia's Bogomil Balkansky praised Temporal's deck precisely for its **Snap, Box, Coinbase, Checkr logo wall** plus open-source community traction — no SDR/AE ratios, no sales-cycle math. LangChain announced its Sequoia Series A with **Rakuten, Elastic, Moody's, Retool plus 70K LangSmith signups in 7 months**. Vercel: Airbnb, Uber, GitHub, Nike, Ticketmaster, TikTok plus Next.js community. Supabase scaled this into **"55–59% of YC's W25 batch use Supabase, 1,000+ YC companies total, 4M+ developers"** — but those metrics are post-Series A artifacts; at the A, Supabase used logo-plus-community.

The principle: at Series A you're not yet selling a repeatable enterprise sales playbook (that's Series B). You're proving **(a) technically sophisticated buyers chose you voluntarily** (logos do this), and **(b) the adoption motion is bottom-up and repeatable** (signups, GitHub stars, OSS contributors). The detailed sales playbook crystallizes at Series B. For your deck, **lead with 6–8 logos with one-line use-case captions**, then a community-traction chart (GitHub stars, signups, monthly actives), then a one-line pricing model — no funnel math.

## The ask slide and closing line

The DocSend research on 200 raises totaling $360M+ recommends **omitting valuation and deal terms** from the ask slide entirely (deliver in person — they vary per investor). What stays on the slide:

The clean format synthesized from Headline VC's 2025 Series A template, OpenVC, and Storypitchdecks: **headline ask (rounded number, never a range), use-of-funds split into 3–4 buckets with percentages, and 3–5 milestones tied to the next round**. For an infrastructure A, the canonical bucket split is **Engineering/R&D 50–65%, GTM (sales + DevRel + marketing) 20–30%, G&A 10–15%**. Milestones should include ARR target, customer count, product GA, SOC 2, headcount — anchored to "this gets us to the metrics needed for a Series B at $X scale."

**Closing line: thesis, not network effect.** The dominant pattern across Temporal, Vercel, LangChain, and Headline VC's template is closing with **the inevitability statement** — "this is the inevitable architecture for [workload]." Temporal closed with durable execution as the inevitable primitive for microservices. The optional epilogue slide is just the company logo plus founder name, photo, contact info — Headline's specific recommendation. **Network-effect framing belongs in the business model slide; the closing line is vision.**

## Visual language and slide count

The honest finding here: **no VC has publicly stated a preference for "designed" decks over Sequoia-template decks.** All published evidence runs the other direction — Alexander Jarvis (the most-cited deck collector) explicitly warns *"DON'T DO ANYTHING FANCY,"* citing failed website-as-deck experiments. Casado's own published fundraising advice on a16z.com is content-first, not design-first, and warns against buzzwords, posturing, and fake urgency. **Use the Sequoia narrative spine; differentiate on craft within slides, not by abandoning structure.**

The dark vs light question has zero quantitative data — every result is template-marketplace marketing. The infra-canonical aesthetic is actually **near-white background (#FAFAFA) with subtle grid plus monospace accents** — Stripe seed deck, Vercel Series A, Linear marketing pages all run light. Reserve dark for embedded product screenshots where the contrast does work. Your ROSEDUST dark theme is fine *if* the product brand is genuinely dark-native (security, terminals, observability) and you're consistent — what kills decks is *inconsistency* between deck and product, not the choice itself. For an a16z partner reviewing on a laptop in good light or printing to PDF, light renders more reliably across viewers and phone preview.

**Words per slide: target 15–30 on content slides, under 10 on transitions, 30pt minimum body / 60pt+ headers** (Kawasaki's mechanical floor). Total deck word count 300–600. The DocSend benchmark is a 3-minute-44-second average reading time — your deck should fit in roughly four minutes of skimming.

**Slide count for infrastructure Series A in 2026: 13 slides plus appendix is the right number** — Kawasaki's 10 is too thin for technical infrastructure; Jarvis's 17–22 is too long for a meeting deck. Recommended order:

1. Title with thesis line
2. Problem (one hero stat)
3. Solution (code snippet)
4. Founder
5. Why now (platform shift)
6. Product / architecture
7. How it works (real screenshots)
8. Traction (logos + community)
9. Cost comparison ($44.86 → $1.42 lands here)
10. Competition (honest, with named strengths)
11. Business model + dual-asset structure
12. Use of funds + milestones
13. Ask + thesis close

Appendix: token graveyard / failure-mode slide titled by objection, technical deep-dive, financials, hiring plan, security/compliance posture, full customer logos.

## Landing page as deck — don't, but borrow heavily

**No primary source confirms Linear, Vercel, or Stripe ever used their landing page as the investor pitch.** Karri Saarinen's only published statement on this distinguishes the *investor pitch* from the *customer pitch* and warns that polished facades can set "the wrong expectation for what the company actually is." Guillermo Rauch's most-cited recent quote (Sequoia Training Data podcast, 2025) is forward-looking: *"v0 prototypes are actually replacing pitch decks… by the time you get to your pitch, it'd be rare these days to not have a working front end because the cost has gone so low."* That's prototypes-as-decks, not landing-pages-as-decks.

**What transfers from landing page to deck**: hero positioning line (becomes title slide), product screenshots full-bleed, customer logo grid, quote testimonials, and the visual system (typography, color, spacing). **What does *not* transfer**: feature lists (decks need one wedge), hero animations (PDFs don't animate), CTA buttons (the deck CTA is "we're raising $X"), SEO copy, blog snippets, careers callouts. And critically, **landing pages almost never carry why-now, TAM, business model, competitive matrix, or financials** — those have to be authored fresh.

## What to actually do on Monday

The defensible thesis from this research: **Casado is not a designer-aesthete; he's a category-architecture thinker who has publicly criticized buzzword-driven pitches.** The deck that wins him is the deck that reads as *technically credible, narratively inevitable, and quietly well-crafted* — Temporal, not Linear-marketing. Use the thesis-statement opener, the code-snippet solution slide, the honest competitive table with named competitor strengths, the logo-led GTM, and the appendix-only token graveyard. The "$44.86 → $1.42" number is your strongest weapon — deploy it on slide 9 where context makes it shocking, not on slide 1 where it has none. Frame compliance the way Vanta did: regulation creates your buyer, you are the gate-opener to a market that didn't exist eighteen months ago. Close with thesis, not network effect. Keep the deck to 13 slides. And remember the most important finding from this entire research: **the canonical decks people are imitating mostly didn't exist.** Stripe pitched on a curl command. Cloudflare pitched on Disrupt Battlefield. Your equivalent is product proof embedded in slides that don't pretend to substitute for it.