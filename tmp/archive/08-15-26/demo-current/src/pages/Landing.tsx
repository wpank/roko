import { type CSSProperties, useEffect, useRef } from 'react';
import { useNavigate } from 'react-router';

const vars = {
  bg: '#0A080C',
  bg2: '#0E0B12',
  panel: '#12101A',
  panel2: '#16131E',
  bone: '#C8B890',
  boneDim: '#998566',
  roseGray: '#988090',
  roseFaint: '#6F5A6A',
  muted: '#584858',
  rose: '#AA7088',
  roseBright: '#CC90A8',
  roseDeep: '#3A2030',
  success: '#80A88A',
  warning: '#C09870',
  border: '#1F1825',
  borderStrong: '#2A2030',
  serif: "'Fraunces', 'Iowan Old Style', 'Times New Roman', serif",
  mono: "'JetBrains Mono', 'SF Mono', Menlo, ui-monospace, Consolas, monospace",
};

const maxW = 1180;

// ─── Shared styles ───

const sectionStyle: CSSProperties = {
  padding: 'clamp(72px, 10vw, 140px) clamp(20px, 4vw, 56px)',
  position: 'relative',
};

const secInnerStyle: CSSProperties = {
  maxWidth: maxW,
  margin: '0 auto',
  position: 'relative',
};

const secLabelStyle: CSSProperties = {
  color: vars.rose,
  letterSpacing: '0.22em',
  fontSize: 10,
  marginBottom: 18,
  display: 'flex',
  alignItems: 'center',
  gap: 12,
  fontFamily: vars.mono,
};

const h2Style: CSSProperties = {
  fontFamily: vars.serif,
  fontWeight: 400,
  fontStyle: 'italic',
  fontSize: 'clamp(36px, 5vw, 56px)',
  lineHeight: 1.1,
  letterSpacing: '-0.005em',
  color: vars.bone,
  marginBottom: 18,
  maxWidth: '24ch',
};

const dekStyle: CSSProperties = {
  fontFamily: vars.mono,
  fontSize: 'clamp(13px, 1.2vw, 15px)',
  color: vars.roseGray,
  maxWidth: '64ch',
  lineHeight: 1.65,
};

function SecLabel({ children }: { children: React.ReactNode }) {
  return (
    <div style={secLabelStyle}>
      <span style={{ width: 24, height: 1, background: vars.rose, display: 'inline-block' }} />
      {children}
    </div>
  );
}

// ─── Nav ───

function Nav({ onStart }: { onStart: () => void }) {
  const navStyle: CSSProperties = {
    position: 'sticky',
    top: 0,
    zIndex: 100,
    background: 'rgba(10, 8, 12, 0.85)',
    backdropFilter: 'blur(12px)',
    WebkitBackdropFilter: 'blur(12px)',
    borderBottom: `1px solid ${vars.border}`,
  };

  const innerStyle: CSSProperties = {
    maxWidth: maxW,
    margin: '0 auto',
    padding: '14px clamp(20px, 4vw, 56px)',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    fontSize: 12,
    fontFamily: vars.mono,
  };

  const linkStyle: CSSProperties = {
    color: vars.roseGray,
    textDecoration: 'none',
    transition: 'color 120ms',
    fontFamily: vars.mono,
    fontSize: 12,
  };

  const ctaStyle: CSSProperties = {
    color: vars.rose,
    border: `1px solid ${vars.rose}`,
    padding: '6px 14px',
    textDecoration: 'none',
    fontSize: 11,
    letterSpacing: '0.1em',
    transition: 'all 120ms',
    cursor: 'pointer',
    background: 'none',
    fontFamily: vars.mono,
  };

  return (
    <nav style={navStyle}>
      <div style={innerStyle}>
        <div style={{ color: vars.rose, fontWeight: 600, letterSpacing: '0.18em' }}>NUNCHI</div>
        <div style={{ display: 'flex', gap: 24, color: vars.muted }}>
          <a href="#problem" style={linkStyle}>problem</a>
          <a href="#loop" style={linkStyle}>runtime</a>
          <a href="#cost" style={linkStyle}>cost</a>
          <a href="#chain" style={linkStyle}>chain</a>
        </div>
        <button
          style={ctaStyle}
          onClick={onStart}
          onMouseEnter={e => {
            e.currentTarget.style.background = vars.rose;
            e.currentTarget.style.color = vars.bg;
          }}
          onMouseLeave={e => {
            e.currentTarget.style.background = 'none';
            e.currentTarget.style.color = vars.rose;
          }}
        >
          LAUNCH DEMO {'→'}
        </button>
      </div>
    </nav>
  );
}

// ─── Hero ───

function Hero({ onStart }: { onStart: () => void }) {
  const btnStyle: CSSProperties = {
    color: vars.rose,
    border: `1px solid ${vars.rose}`,
    padding: '12px 22px',
    textDecoration: 'none',
    fontSize: 12,
    letterSpacing: '0.12em',
    transition: 'all 120ms',
    cursor: 'pointer',
    background: 'none',
    fontFamily: vars.mono,
  };

  const ghostStyle: CSSProperties = {
    ...btnStyle,
    color: vars.roseGray,
    borderColor: vars.borderStrong,
  };

  return (
    <section id="hero" style={{ ...sectionStyle, minHeight: '100vh', paddingTop: 80, display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
      <div style={secInnerStyle}>
        <div style={{ color: vars.rose, letterSpacing: '0.22em', fontSize: 11, marginBottom: 30, fontFamily: vars.mono }}>
          SERIES A {'·'} APRIL 2026 {'·'} AGENT COORDINATION PLANE
        </div>
        <h1 style={{
          fontFamily: vars.serif, fontWeight: 400, fontStyle: 'italic',
          fontSize: 'clamp(48px, 7vw, 88px)', lineHeight: 1.04,
          letterSpacing: '-0.01em', color: vars.bone, marginBottom: 28,
        }}>
          The durable runtime<br /><span style={{ color: vars.rose, fontStyle: 'italic' }}>for production agents.</span>
        </h1>
        <div style={{
          fontFamily: vars.mono, fontSize: 'clamp(15px, 1.5vw, 18px)',
          color: vars.boneDim, marginTop: 10, maxWidth: '56ch',
        }}>
          The model is the same. The system is the variable.
        </div>

        <div className="fade" style={{
          marginTop: 'clamp(40px, 6vw, 80px)',
          display: 'grid', gridTemplateColumns: 'minmax(0, 1.1fr) minmax(0, 1fr)', gap: 56, alignItems: 'end',
        }}>
          <div>
            <div style={{
              background: vars.panel2, border: `1px solid ${vars.borderStrong}`, padding: '22px 26px',
            }}>
              <div style={{ color: vars.rose, fontSize: 10, letterSpacing: '0.18em', marginBottom: 14, fontFamily: vars.mono }}>
                PER-TASK COST {'·'} HAL BENCHMARK
              </div>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 18, flexWrap: 'wrap' }}>
                <span style={{
                  fontFamily: vars.serif, fontStyle: 'italic', fontSize: 'clamp(36px, 5vw, 56px)',
                  color: vars.roseFaint, textDecoration: 'line-through', textDecorationColor: vars.roseDeep,
                }}>$44.86</span>
                <span style={{ color: vars.rose, fontSize: 22 }}>{'→'}</span>
                <span style={{ fontFamily: vars.serif, fontStyle: 'italic', fontSize: 'clamp(48px, 7vw, 84px)', color: vars.roseBright }}>$1.42</span>
              </div>
              <div style={{ marginTop: 12, color: vars.muted, fontSize: 11, lineHeight: 1.7, fontFamily: vars.mono }}>
                <strong style={{ color: vars.roseGray }}>Princeton HAL {'·'} ICLR 2026.</strong>{' '}
                Naive baseline excludes caching by design. Optimized run is reproducible with{' '}
                <code style={{ color: vars.rose, background: vars.bg, padding: '1px 6px', fontSize: 11 }}>nunchi run --share</code>.
              </div>
            </div>
          </div>
          <div style={{ color: vars.roseGray, fontSize: 14, lineHeight: 1.7, fontFamily: vars.mono }}>
            Agents broke reliability — again. <strong style={{ color: vars.bone, fontWeight: 600 }}>41–86% of multi-agent deployments fail in production</strong>, and 79% of those failures come from coordination, not capability. Nunchi is the durable runtime that closes the loop.
            <div style={{ marginTop: 36, display: 'flex', gap: 16, flexWrap: 'wrap' }}>
              <button
                style={btnStyle}
                onClick={onStart}
                onMouseEnter={e => { e.currentTarget.style.background = vars.rose; e.currentTarget.style.color = vars.bg; }}
                onMouseLeave={e => { e.currentTarget.style.background = 'none'; e.currentTarget.style.color = vars.rose; }}
              >
                LAUNCH LIVE DEMO
              </button>
              <a
                href="#loop"
                style={ghostStyle}
                onMouseEnter={e => { e.currentTarget.style.color = vars.bone; e.currentTarget.style.borderColor = vars.rose; }}
                onMouseLeave={e => { e.currentTarget.style.color = vars.roseGray; e.currentTarget.style.borderColor = vars.borderStrong; }}
              >
                SEE THE RUNTIME {'→'}
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Problem ───

function ProblemSection() {
  const stats = [
    { num: '41–86%', label: 'of multi-agent deployments fail in production.', src: 'MAST taxonomy · Berkeley · NeurIPS 2025 (arXiv:2503.13657)' },
    { num: '79%', label: 'of those failures come from coordination — not from model capability.', src: 'Same study. Replicated by Hugging Face GAIA leaderboard, 2025–26.' },
    { num: '82 : 1', label: 'machine-to-human identity ratio in production today.', src: 'CyberArk · Entro Security · State of NHI 2025' },
  ];

  return (
    <section id="problem" style={sectionStyle}>
      <div style={secInnerStyle}>
        <SecLabel>02 {'·'} THE PROBLEM</SecLabel>
        <h2 style={h2Style}>Coordination, not capability.</h2>
        <p style={dekStyle}>Three numbers describe the gap between agents that work in demos and agents that work in production. None of them is about how smart the model is.</p>

        <div className="fade" style={{
          marginTop: 56, display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
          gap: 4, border: `1px solid ${vars.border}`,
        }}>
          {stats.map(s => (
            <div key={s.num} style={{ background: vars.panel, padding: '36px 30px', border: `1px solid ${vars.borderStrong}`, margin: -1 }}>
              <div style={{ fontFamily: vars.serif, fontStyle: 'italic', fontSize: 'clamp(56px, 6.5vw, 76px)', color: vars.rose, lineHeight: 1, marginBottom: 18, letterSpacing: '-0.02em' }}>
                {s.num}
              </div>
              <div style={{ color: vars.bone, fontSize: 14, lineHeight: 1.45, marginBottom: 12, fontFamily: vars.mono }}>{s.label}</div>
              <div style={{ color: vars.muted, fontSize: 11, fontFamily: vars.mono }}>{s.src}</div>
            </div>
          ))}
        </div>

        <div className="fade" style={{
          marginTop: 48, padding: '24px 30px',
          borderLeft: `2px solid ${vars.rose}`, color: vars.roseGray,
          fontSize: 14, lineHeight: 1.7, fontFamily: vars.mono,
          background: 'rgba(170, 112, 136, 0.03)',
        }}>
          <strong style={{ color: vars.bone, fontWeight: 600 }}>Klarna reversed its all-AI customer service in May 2025.</strong> The agents could do the tasks. Klarna couldn't audit them. The pattern is older than agents — monoliths broke reliability in 2010, microservices broke it again in 2020. Coordination layers fixed each. Nunchi is the coordination layer for agents.
        </div>
      </div>
    </section>
  );
}

// ─── Runtime Loop ───

function LoopSection() {
  const stages = [
    { num: '01 · OBSERVE', title: 'Pulse', prim: 'ephemeral · structured', body: 'Every action emits structured events on the bus — token, latency, tool, gate, decision. Captured at source, not reconstructed after.' },
    { num: '02 · DECIDE', title: 'Score · Route', prim: 'cascade · context', body: 'Cascade routing chooses model and parameters per task. Knowledge substrate injects context. Reputation gates trust.' },
    { num: '03 · ENFORCE', title: 'Gate pipeline', prim: '11 gates · language-agnostic', body: 'Compile, test, diff, lint, security, semantic, coverage, budget, latency, policy, replay. Failures route back. Frontier calls are last resort.' },
    { num: '04 · RECORD', title: 'Signal', prim: 'durable · cited', body: 'Content-addressed. HDC-fingerprinted. Replayable. Citable cross-org. Becomes context for the next agent on the next task.' },
  ];

  return (
    <section id="loop" style={{ ...sectionStyle, background: vars.bg2 }}>
      <div style={secInnerStyle}>
        <SecLabel>03 {'·'} THE RUNTIME</SecLabel>
        <h2 style={h2Style}>Observe. Decide. Enforce. Record.</h2>
        <p style={dekStyle}>The control loop the field said couldn't close on agents. Every stage external to the model. Every stage deterministic. Every stage replayable.</p>

        <div className="fade" style={{
          marginTop: 48, display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
          gap: 0, border: `1px solid ${vars.border}`,
        }}>
          {stages.map((s, i) => (
            <div key={i} style={{
              padding: '28px 24px', borderRight: i < 3 ? `1px solid ${vars.border}` : 'none',
              background: vars.panel,
            }}>
              <div style={{ color: vars.rose, fontSize: 11, letterSpacing: '0.18em', marginBottom: 14, fontFamily: vars.mono }}>{s.num}</div>
              <h3 style={{ fontFamily: vars.serif, fontStyle: 'italic', fontSize: 22, color: vars.bone, marginBottom: 8, fontWeight: 400 }}>{s.title}</h3>
              <div style={{ color: vars.rose, fontFamily: vars.mono, fontSize: 11, margin: '10px 0 16px', paddingBottom: 12, borderBottom: `1px solid ${vars.borderStrong}` }}>{s.prim}</div>
              <p style={{ color: vars.roseGray, fontSize: 13, lineHeight: 1.65, fontFamily: vars.mono }}>{s.body}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── Cost ───

function CostSection() {
  const rows = [
    { layer: 'Baseline', opt: 'HAL benchmark, naive', mult: '—', src: 'Princeton, ICLR 2026 — excludes caching' },
    { layer: '1. Cache', opt: 'Anthropic prompt caching, priced per docs', mult: '5×', src: 'Anthropic API documentation, current pricing' },
    { layer: '2. Route', opt: 'RouteLLM cascade haiku → sonnet → opus', mult: '3×', src: 'Princeton NLP, RouteLLM 2024' },
    { layer: '3. Gate', opt: 'Pre-screen (compile, test, diff) before frontier', mult: '2×', src: 'Internal benchmark, reproducible' },
  ];

  const cellStyle: CSSProperties = { padding: '14px 16px', textAlign: 'left', borderBottom: `1px solid ${vars.border}`, verticalAlign: 'top', fontFamily: vars.mono };

  return (
    <section id="cost" style={{ ...sectionStyle, background: vars.bg, borderTop: `1px solid ${vars.border}` }}>
      <div style={secInnerStyle}>
        <SecLabel>04 {'·'} COST PROOF</SecLabel>
        <h2 style={h2Style}>Forty-three dollars to a buck forty-two.</h2>
        <p style={dekStyle}>Three composable optimizations, each measurable. We disclose the methodology because the alternative is "trust us."</p>

        <div className="fade" style={{
          marginTop: 48, display: 'grid', gridTemplateColumns: 'minmax(0, 1.2fr) minmax(0, 1fr)', gap: 56, alignItems: 'start',
        }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13, fontFamily: vars.mono }}>
            <thead>
              <tr>
                {['Layer', 'Optimization', 'Multiplier', 'Source / verification'].map(h => (
                  <th key={h} style={{ ...cellStyle, color: vars.muted, fontWeight: 400, fontSize: 10, letterSpacing: '0.18em', textTransform: 'uppercase' }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map(r => (
                <tr key={r.layer}>
                  <td style={{ ...cellStyle, color: vars.bone }}>{r.layer}</td>
                  <td style={{ ...cellStyle, color: vars.roseGray }}>{r.opt}</td>
                  <td style={{ ...cellStyle, color: vars.rose, fontWeight: 600, textAlign: 'right' }}>{r.mult}</td>
                  <td style={{ ...cellStyle, color: vars.roseGray }}>{r.src}</td>
                </tr>
              ))}
              <tr>
                <td style={{ ...cellStyle, color: vars.bone, paddingTop: 18, borderBottom: 'none', borderTop: `1px solid ${vars.rose}` }}>
                  <strong style={{ color: vars.bone }}>Composed</strong>
                </td>
                <td style={{ ...cellStyle, color: vars.roseGray, paddingTop: 18, borderBottom: 'none', borderTop: `1px solid ${vars.rose}` }}>Multiplicative, with disclosure</td>
                <td style={{ ...cellStyle, color: vars.roseBright, fontWeight: 600, fontSize: 14, textAlign: 'right', paddingTop: 18, borderBottom: 'none', borderTop: `1px solid ${vars.rose}` }}>30{'×'}</td>
                <td style={{ ...cellStyle, color: vars.bone, paddingTop: 18, borderBottom: 'none', borderTop: `1px solid ${vars.rose}` }}>
                  <strong style={{ color: vars.bone }}>$44.86 {'→'} $1.42</strong> per HAL task
                </td>
              </tr>
            </tbody>
          </table>

          <div style={{
            background: vars.panel, border: `1px solid ${vars.borderStrong}`, borderLeft: `2px solid ${vars.rose}`,
            padding: '24px 26px', color: vars.roseGray, fontSize: 13, lineHeight: 1.7, fontFamily: vars.mono,
          }}>
            <h3 style={{ fontFamily: vars.mono, fontStyle: 'normal', fontSize: 11, color: vars.rose, letterSpacing: '0.18em', marginBottom: 12, textTransform: 'uppercase', fontWeight: 400 }}>
              Honest read
            </h3>
            <p>The HAL baseline excludes prompt caching by design — that's the reference point against which the field measures, not a number we picked to flatter ourselves. Our optimized $1.42 includes Anthropic prompt caching, Princeton's RouteLLM cascade, and our own gate pre-screening data.</p>
            <p style={{ marginTop: 12 }}>Every layer is reproducible. <code style={{ color: vars.rose, background: vars.bg, padding: '1px 6px', fontSize: 12 }}>nunchi run --share</code> posts a public receipt.</p>
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Chain ───

function ChainSection() {
  const blocks = [
    { name: 'HDC', sub: 'PRECOMPILE 0xA01', body: 'Hyperdimensional similarity over 10,240-bit binary vectors. Native EVM precompile, ~400 gas top-K. The substrate that makes cross-org knowledge queryable on-chain at a price agents can pay.' },
    { name: 'ERC-8004', sub: 'AGENT IDENTITY', body: 'Transferable on-chain identity with 7-domain reputation EMA — coding, security, research, chain, knowledge, ops, strategy. Slashable on policy violation. Queryable cross-org without disclosure.' },
    { name: 'ZK-HDC', sub: 'CIRCOM · GROTH16', body: 'Hamming distance proofs over committed hypervectors. Sub-second proving on a laptop. ~250K gas to verify. Lets one agent prove it knows something without revealing what.' },
  ];

  return (
    <section id="chain" style={{ ...sectionStyle, background: vars.bg2 }}>
      <div style={secInnerStyle}>
        <SecLabel>05 {'·'} WHY IT COMPOUNDS</SecLabel>
        <div className="fade" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.1fr) minmax(0, 1fr)', gap: 56, alignItems: 'end' }}>
          <div>
            <h2 style={h2Style}>The chain is the moat. The runtime ships first.</h2>
            <p style={dekStyle}>The runtime is open source today. The chain is a 2027 milestone. Three protocol primitives turn coordination cost into compounding identity, knowledge, and verifiability.</p>
          </div>
          <div style={{ padding: '18px 22px', border: `1px solid ${vars.borderStrong}`, background: vars.panel, color: vars.roseGray, fontSize: 12, lineHeight: 1.7, fontFamily: vars.mono }}>
            <div style={{ color: vars.rose, letterSpacing: '0.16em', fontSize: 10, marginBottom: 8, fontFamily: vars.mono }}>CHAIN STATUS {'·'} Q2 2026</div>
            Specs frozen. <strong style={{ color: vars.warning, fontFamily: vars.mono }}>76 implementation items deferred Tier 6</strong> — testnet 2027. The runtime works without the chain.
          </div>
        </div>

        <div className="fade" style={{ marginTop: 64, display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 0, border: `1px solid ${vars.border}` }}>
          {blocks.map((b, i) => (
            <div key={b.name} style={{
              padding: '36px 30px', background: vars.panel,
              borderRight: i < 2 ? `1px solid ${vars.border}` : 'none',
            }}>
              <div style={{ fontFamily: vars.serif, fontStyle: 'italic', fontSize: 32, color: vars.roseBright, marginBottom: 4 }}>{b.name}</div>
              <div style={{ color: vars.muted, fontSize: 11, letterSpacing: '0.1em', marginBottom: 18, fontFamily: vars.mono }}>{b.sub}</div>
              <div style={{ color: vars.roseGray, fontSize: 13, lineHeight: 1.65, fontFamily: vars.mono }}>{b.body}</div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

// ─── CTA ───

function CTASection({ onStart }: { onStart: () => void }) {
  const btnStyle: CSSProperties = {
    textDecoration: 'none', padding: '14px 28px', fontSize: 12,
    letterSpacing: '0.14em', transition: 'all 120ms',
    border: `1px solid ${vars.rose}`, color: vars.rose,
    cursor: 'pointer', background: 'none', fontFamily: vars.mono,
  };

  return (
    <section id="cta" style={{ ...sectionStyle, background: vars.bg2, textAlign: 'left' }}>
      <div style={secInnerStyle}>
        <SecLabel>07 {'·'} GET INVOLVED</SecLabel>
        <h2 style={{ ...h2Style, maxWidth: '26ch' }}>This is the inevitable architecture for agent coordination.</h2>
        <p style={dekStyle}>
          We're talking with eight enterprise design partners. Three are deploying. The runtime is open source, the cloud is in private beta, the chain ships in 2027.
        </p>

        <div className="fade" style={{ marginTop: 40, display: 'flex', gap: 16, flexWrap: 'wrap' }}>
          <button
            style={btnStyle}
            onClick={onStart}
            onMouseEnter={e => { e.currentTarget.style.background = vars.rose; e.currentTarget.style.color = vars.bg; }}
            onMouseLeave={e => { e.currentTarget.style.background = 'none'; e.currentTarget.style.color = vars.rose; }}
          >
            LAUNCH LIVE DEMO {'→'}
          </button>
          <a
            href="mailto:design-partner@nunchi.dev?subject=Nunchi%20design%20partner%20interest"
            style={{ ...btnStyle, color: vars.roseGray, borderColor: vars.borderStrong }}
            onMouseEnter={e => { e.currentTarget.style.color = vars.bone; e.currentTarget.style.borderColor = vars.rose; }}
            onMouseLeave={e => { e.currentTarget.style.color = vars.roseGray; e.currentTarget.style.borderColor = vars.borderStrong; }}
          >
            DESIGN PARTNER {'→'}
          </a>
        </div>

        <div style={{
          marginTop: 64, paddingTop: 32, borderTop: `1px solid ${vars.border}`,
          display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 36,
          color: vars.roseGray, fontSize: 12, lineHeight: 1.65, fontFamily: vars.mono,
        }}>
          <div>
            <div style={{ color: vars.rose, letterSpacing: '0.16em', fontSize: 10, marginBottom: 10 }}>RUNTIME</div>
            <strong style={{ color: vars.bone, fontWeight: 600 }}>Roko</strong> {'·'} Apache 2.0 {'·'} 18 crates {'·'} Rust {'·'} open source.
          </div>
          <div>
            <div style={{ color: vars.rose, letterSpacing: '0.16em', fontSize: 10, marginBottom: 10 }}>DESIGN PARTNERS</div>
            Currently in deployment with three named enterprise partners. Logos on request.
          </div>
          <div>
            <div style={{ color: vars.rose, letterSpacing: '0.16em', fontSize: 10, marginBottom: 10 }}>SECURITY</div>
            SOC 2 Type II in progress. Self-hosted option available.
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Footer ───

function Footer() {
  return (
    <footer style={{
      padding: '36px clamp(20px, 4vw, 56px)', borderTop: `1px solid ${vars.border}`,
      color: vars.muted, fontSize: 11, display: 'flex', justifyContent: 'space-between',
      gap: 16, flexWrap: 'wrap', fontFamily: vars.mono,
    }}>
      <div>NUNCHI {'·'} Series A {'·'} April 2026</div>
      <div>Made for agents that ship.</div>
    </footer>
  );
}

// ─── Main Landing ───

export function LandingPage() {
  const navigate = useNavigate();
  const bodyRef = useRef<HTMLDivElement>(null);

  const handleStart = () => navigate('/app');

  // Intersection observer for fade-in on scroll
  useEffect(() => {
    const body = bodyRef.current;
    if (!body) return;

    const obs = new IntersectionObserver(
      entries => {
        entries.forEach(e => {
          if (e.isIntersecting) {
            (e.target as HTMLElement).style.opacity = '1';
            (e.target as HTMLElement).style.transform = 'translateY(0)';
            obs.unobserve(e.target);
          }
        });
      },
      { threshold: 0.12, rootMargin: '0px 0px -8% 0px' },
    );

    body.querySelectorAll('.fade').forEach(el => obs.observe(el));
    return () => obs.disconnect();
  }, []);

  return (
    <div ref={bodyRef} style={{
      background: vars.bg, color: vars.bone,
      fontFamily: vars.mono, fontSize: 14, lineHeight: 1.6,
      WebkitFontSmoothing: 'antialiased',
      overflowX: 'hidden', minHeight: '100vh',
    }}>
      {/* CRT scanlines */}
      <div style={{
        position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 200,
        background: `repeating-linear-gradient(0deg, rgba(170,112,136,0.012) 0px, rgba(170,112,136,0.012) 1px, transparent 1px, transparent 3px)`,
        mixBlendMode: 'soft-light',
      }} />
      {/* Vignette */}
      <div style={{
        position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 199,
        background: 'radial-gradient(ellipse at 50% 0%, rgba(170,112,136,0.04) 0%, transparent 55%)',
      }} />

      <Nav onStart={handleStart} />
      <Hero onStart={handleStart} />
      <ProblemSection />
      <LoopSection />
      <CostSection />
      <ChainSection />
      <CTASection onStart={handleStart} />
      <Footer />

      {/* Fade-in base styles */}
      <style>{`
        .fade {
          opacity: 0;
          transform: translateY(20px);
          transition: opacity 700ms ease, transform 700ms cubic-bezier(0.2, 0.7, 0.3, 1);
        }
        ::selection {
          background: ${vars.rose};
          color: ${vars.bg};
        }
      `}</style>
    </div>
  );
}
