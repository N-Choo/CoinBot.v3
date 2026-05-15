export default function FeaturesSection() {
  return (
    <section className="features" id="features">
      <div className="features-inner">
        <div className="section-label">How it works</div>
        <h2 className="section-title">Three engines. One automated system.</h2>
        <p className="section-sub">Institutional-grade infrastructure, simplified for everyone.</p>

        <div className="feat-grid">
          <div className="feat-card">
            <div className="feat-num">01</div>
            <div className="feat-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></svg>
            </div>
            <h3>Buy Low, Sell High</h3>
            <p>Actively monitors the market to execute trades at optimal moments, capturing profit from price volatility.</p>
          </div>
          <div className="feat-card">
            <div className="feat-num">02</div>
            <div className="feat-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="3" y="11" width="18" height="11" rx="2" ry="2" /></svg>
            </div>
            <h3>Auto-Stake</h3>
            <p>Idle assets are instantly deployed to secure staking pools, earning passive APY between trades.</p>
          </div>
          <div className="feat-card">
            <div className="feat-num">03</div>
            <div className="feat-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10" /><path d="M12 6v6l4 2" /></svg>
            </div>
            <h3>Zero Effort</h3>
            <p>Connect once, configure your parameters, and let the bot navigate markets around the clock.</p>
          </div>
        </div>
      </div>
    </section>
  )
}
