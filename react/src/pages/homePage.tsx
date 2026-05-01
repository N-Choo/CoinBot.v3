
import { useState } from 'react';
import '../styles/homepage.css';

// Architecture Flow with specific badge colors and labels
const stepsData = [
  {
    title: "Contract Purchase",
    desc: "You deposit $1,000. Capital moves from your wallet into the Bot Contract, ready for deployment.",
    badge: { label: "Step 1 — Deposit", bg: "#EEEDFE", textC: "#3C3489" },
  },
  {
    title: "Initial Allocation",
    desc: "The bot splits your capital to optimize income: it uses $600 for active trades and puts the remaining $400 into the Staking Pool for passive yield.",
    badge: { label: "Step 2 — Split", bg: "#E1F5EE", textC: "#0F6E56" },
  },
  {
    title: "Dynamic Unstaking",
    desc: "The bot intelligently schedules unstaking. If funds from closed trades are lower than the estimated requirement for the next trade, it unstakes and reinvests into the opportunity.",
    badge: { label: "Step 3 — Rebalance", bg: "#FAEEDA", textC: "#854F0B" },
  },
  {
    title: "Trade Execution",
    desc: "When a trade executes, it is securely logged. After closing a position, the bot secures the $200 profit and prepares to re-stake the majority of the funds while keeping a portion liquid for unexpected opportunities.",
    badge: { label: "Step 4 — Profit", bg: "#E1F5EE", textC: "#0F6E56" },
  },
  {
    title: "Withdraw & Fee",
    desc: "You withdraw your funds. Your principal and profits return to your wallet. A 5% performance fee on the profit ($10) goes to the platform host.",
    badge: { label: "Step 5 — Withdraw", bg: "#EEEDFE", textC: "#3C3489" },
  }
];

export default function CoinBot() {
  // Start at step 0 so the first card is visible immediately
  const [activeStep, setActiveStep] = useState(0);

  const handleStepClick = (index) => {
    setActiveStep(index);
  };

  const nextStep = () => {
    setActiveStep((prev) => (prev === stepsData.length - 1 ? 0 : prev + 1));
  };

  const prevStep = () => {
    if (activeStep > 0) setActiveStep(activeStep - 1);
  };

  // Renders the specific visualizer diagram based on the active step
  const renderVisualizer = () => {
    // The "key" prop forces React to remount the DOM node, re-triggering the CSS animations
    switch (activeStep) {
      case 0:
        return (
          <div key="vis-0" className="node-diagram flow-in">
            <div className="nd-box" style={{ background: '#EEEDFE', borderColor: '#AFA9EC', color: '#3C3489' }}>
              <div className="nd-amount">$1,000</div>
              <div className="nd-label">Your wallet</div>
            </div>
            <div className="nd-arrow">→</div>
            <div className="nd-box pulse" style={{ background: '#534AB7', color: '#fff', borderColor: '#534AB7' }}>
              <div className="nd-amount">$1,000</div>
              <div className="nd-label">Bot contract</div>
            </div>
          </div>
        );
      case 1:
        return (
          <div key="vis-1" className="bar-row pop-in">
            <div className="bar-item">
              <div className="bar-label">Active trades</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '60%', background: '#534AB7', color: '#fff' }}>$600</div>
              </div>
            </div>
            <div className="bar-item">
              <div className="bar-label">Staking pool</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '40%', background: '#1D9E75', color: '#fff' }}>$400</div>
              </div>
            </div>
          </div>
        );
      case 2:
        return (
          <div key="vis-2" className="pop-in" style={{ width: '100%' }}>
            <div className="node-diagram" style={{ marginBottom: '14px' }}>
              <div className="nd-box" style={{ background: '#FAEEDA', borderColor: '#FAC775', color: '#854F0B' }}>
                <div className="nd-amount">🏦</div>
                <div className="nd-label">Staking pool</div>
              </div>
              <div className="nd-arrow">⇄</div>
              <div className="nd-box pulse" style={{ background: '#534AB7', color: '#fff', borderColor: '#534AB7' }}>
                <div className="nd-amount">📈</div>
                <div className="nd-label">Next trade</div>
              </div>
            </div>
            <div style={{ textAlign: 'center', fontSize: '12px', color: 'var(--text-muted)' }}>
              <span className="vis-tag" style={{ background: '#FAEEDA', color: '#854F0B' }}>Unstakes if needed</span>
              &nbsp;→&nbsp;
              <span className="vis-tag" style={{ background: '#EEEDFE', color: '#3C3489' }}>Reinvests capital</span>
            </div>
          </div>
        );
      case 3:
        return (
          <div key="vis-3" className="bar-row pop-in">
            <div className="bar-item">
              <div className="bar-label">Capital</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '80%', background: '#534AB7', color: '#fff' }}>$800</div>
              </div>
            </div>
            <div className="bar-item">
              <div className="bar-label">Profit secured ✓</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '20%', background: '#1D9E75', color: '#fff' }}>$200</div>
              </div>
            </div>
            <div className="bar-item">
              <div className="bar-label">Re-staking</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '70%', background: '#9FE1CB', color: '#085041' }}>$700</div>
              </div>
            </div>
            <div className="bar-item">
              <div className="bar-label">Liquid reserve</div>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: '10%', background: '#5DCAA5', color: '#085041' }}>$100</div>
              </div>
            </div>
          </div>
        );
      case 4:
        return (
          <div key="vis-4" className="pop-in" style={{ width: '100%' }}>
            <div className="node-diagram" style={{ marginBottom: '14px' }}>
              <div className="nd-box pulse" style={{ background: '#534AB7', color: '#fff', borderColor: '#534AB7' }}>
                <div className="nd-amount">🤖 $1,190</div>
                <div className="nd-label">Bot contract</div>
              </div>
              <div className="nd-arrow">→</div>
              <div className="nd-box" style={{ background: '#EEEDFE', borderColor: '#AFA9EC', color: '#3C3489' }}>
                <div className="nd-amount">💳 $1,190</div>
                <div className="nd-label">Your wallet</div>
              </div>
            </div>
            <div style={{ textAlign: 'center' }}>
              <span className="vis-tag" style={{ background: '#FAEEDA', color: '#854F0B' }}>Platform fee: $10 (5% of profit)</span>
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  const currentData = stepsData[activeStep];

  return (
    <div className="cb-wrap">

      {/* Hero Section */}
      <section className="cb-hero">
        <h1>The Wealth Engine<br /><em>On Autopilot.</em></h1>
        <p className="cb-hero-sub">
          CoinBot automates your trading — buying low and selling high — while staking idle assets for passive yield. 24/7 precision.
        </p>
        <div className="cb-hero-btns">
          <button className="btn-p">Launch Your Bot</button>
          <button className="btn-s">View Strategies</button>
        </div>
      </section>

      {/* Pillars Section */}
      <section className="cb-pillars" id="architecture">
        <div className="cb-section-label">Built for precision</div>
        <h2 className="cb-section-h">Institutional-grade infrastructure,<br />simplified for everyone.</h2>
        <p className="cb-section-sub">Three core engines working in concert.</p>
        <div className="pillars-grid">
          <div className="pillar-card">
            <h3>Buy Low, Sell High</h3>
            <p>Actively monitors the market to execute trades at optimal moments, capturing profit from price volatility.</p>
          </div>
          <div className="pillar-card">
            <h3>Auto-Stake</h3>
            <p>Idle assets are instantly deployed to secure staking pools, earning passive APY between trades.</p>
          </div>
          <div className="pillar-card">
            <h3>Zero Effort</h3>
            <p>Connect once, configure your parameters, and let the bot navigate markets around the clock.</p>
          </div>
        </div>
      </section>

      {/* NEW Fund Flow Section */}
      <section className="cb-flow">
        <div className="cb-section-label">Fund architecture</div>
        <h2 className="cb-section-h">See exactly where your capital goes.</h2>
        <p className="cb-section-sub">Step through the full lifecycle — including dynamic reinvestment.</p>

        {/* Timeline & Card Wrapper */}
        <div className="flow-interactive-wrap">

          {/* Horizontal Timeline */}
          <div className="vis-timeline">
            {stepsData.map((step, index) => {
              const isActive = index === activeStep;
              const isDone = index < activeStep;
              return (
                <div key={index} style={{ display: 'flex', flex: 1, alignItems: 'center' }}>
                  <div
                    className={`vis-step-item ${isActive ? 'active' : ''} ${isDone ? 'done' : ''}`}
                    onClick={() => handleStepClick(index)}
                  >
                    <div className="vis-step-dot">{isDone ? '✓' : index + 1}</div>
                    <div className="vis-step-label">{step.title}</div>
                  </div>
                  {/* Connector line (don't show after the last item) */}
                  {index < stepsData.length - 1 && (
                    <div className="vis-connector">
                      <div className={`vis-connector-fill ${isDone ? 'active' : ''}`}></div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Dynamic Info Card */}
          <div className="vis-card-area">
            <div className="vis-card-inner">
              <div
                className="vis-step-badge"
                style={{ background: currentData.badge.bg, color: currentData.badge.textC }}
              >
                {currentData.badge.label}
              </div>

              <div className="vis-step-title">{currentData.title}</div>
              <div className="vis-step-desc">{currentData.desc}</div>

              {/* Animated Visualizer Box */}
              <div className="vis-render-area">
                {renderVisualizer()}
              </div>
            </div>
          </div>

          {/* Controls */}
          <div className="vis-controls">
            <button className="vis-btn" onClick={prevStep} disabled={activeStep === 0}>← Prev</button>
            <button className="vis-btn-primary" onClick={nextStep}>
              {activeStep === stepsData.length - 1 ? '↺ Restart' : 'Next →'}
            </button>
            <span className="vis-progress-text">{activeStep + 1} of {stepsData.length}</span>
          </div>

        </div>
      </section>

      {/* Footer */}
      <footer>
        <p>© {new Date().getFullYear()} CoinBot</p>
      </footer>
    </div>
  );
}
