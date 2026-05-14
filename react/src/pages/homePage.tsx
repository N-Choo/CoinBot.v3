import '../styles/homepage.css'

export default function CoinBot() {
  return (
    <div className="landing">
      {/* Hero */}
      <section className="hero">
        <div className="hero-body">
          <div className="hero-badge">Automated Trading Protocol</div>
          <h1>
            Trade smarter.<br />
            <span className="hero-em">Earn passively.</span>
          </h1>
          <p className="hero-sub">
            CoinBot automates your trading &mdash; buying low and selling high &mdash; while staking idle assets for passive yield. 24/7 precision, zero manual effort.
          </p>
          <div className="hero-actions">
            <button className="btn-p">Launch Your Bot</button>
            <button className="btn-s">View Strategies</button>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="stats">
        <div className="stats-inner">
          <div className="stat">
            <span className="stat-num">$450M+</span>
            <span className="stat-label">Trading Volume</span>
          </div>
          <div className="stat-div" />
          <div className="stat">
            <span className="stat-num">99.9%</span>
            <span className="stat-label">Uptime</span>
          </div>
          <div className="stat-div" />
          <div className="stat">
            <span className="stat-num">0.0%</span>
            <span className="stat-label">Liquidation Risk</span>
          </div>
          <div className="stat-div" />
          <div className="stat">
            <span className="stat-num">10K+</span>
            <span className="stat-label">Active Users</span>
          </div>
        </div>
      </section>

      {/* System Diagram */}
      <section className="reactor">
        <div className="reactor-box">
          <div className="reactor-header">
            <div className="section-label">How it flows</div>
          </div>
          <div className="reactor-flow">
            <div className="reactor-node">
              <div className="reactor-core">
                <div className="reactor-core-inner" />
              </div>
              <span className="reactor-label">Deposit</span>
              <span className="reactor-sub">Wallet</span>
            </div>
            <div className="reactor-line" />
            <div className="reactor-node">
              <div className="reactor-core">
                <div className="reactor-core-inner" />
              </div>
              <span className="reactor-label">Execute</span>
              <span className="reactor-sub">Bot</span>
            </div>
            <div className="reactor-line" />
            <div className="reactor-node">
              <div className="reactor-core">
                <div className="reactor-core-inner" />
              </div>
              <span className="reactor-label">Exchange</span>
              <span className="reactor-sub">Market</span>
            </div>
            <div className="reactor-line" />
            <div className="reactor-node">
              <div className="reactor-core">
                <div className="reactor-core-inner" />
              </div>
              <span className="reactor-label">Stake</span>
              <span className="reactor-sub">Pool</span>
            </div>
            <div className="reactor-line" />
            <div className="reactor-node">
              <div className="reactor-core">
                <div className="reactor-core-inner" />
              </div>
              <span className="reactor-label">Withdraw</span>
              <span className="reactor-sub">Profit</span>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
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

      {/* Footer */}
      <footer className="footer">
        <div className="footer-inner">
          <span className="footer-brand">CoinBot</span>
          <p>&copy; {new Date().getFullYear()} &mdash; All rights reserved.</p>
        </div>
      </footer>
    </div>
  )
}
