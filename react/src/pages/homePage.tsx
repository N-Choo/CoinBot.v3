import React from 'react';
import '../styles/homepage.css';

const CoinBotLandingPage = () => {
  return (
    <div className="lp-wrapper">

      {/* 2. Hero Section */}
      <header className="hero">
        <div className="hero-content">
          <h1 className="hero-title">
            The Wealth Engine <br />
            <span className="text-gradient">On Autopilot.</span>
          </h1>
          <p className="hero-desc">
            CoinBot automates your trading—buying low and selling high—while staking your idle assets to generate passive income.
          </p>
          <div className="hero-btns">
            {/* Changed from "Start Saving Now" to reflect active trading */}
            <button className="btn-primary">Launch Your Bot</button>
          </div>
        </div>
      </header>

      {/* 3. Features: The Three Pillars */}
      <section id="how" className="features-grid">
        <div className="f-card">
          {/* Shifted focus from DCA 'averaging down' to capturing profit */}
          <div className="f-icon">📈</div>
          <h3>Buy Low, Sell High</h3>
          <p>CoinBot actively monitors the market to execute trades at the perfect moments, capturing profits from price volatility.</p>
        </div>
        <div className="f-card">
          <div className="f-icon">🔒</div>
          <h3>Auto-Stake</h3>
          {/* Clarified that it's idle assets waiting to be traded */}
          <p>Your idle assets are instantly moved to secure staking pools, earning you passive APY while waiting for the next optimal trade.</p>
        </div>
        <div className="f-card">
          <div className="f-icon">⚡</div>
          <h3>Zero Effort</h3>
          {/* Removed the DCA "Set your schedule" phrase */}
          <p>Connect your wallet once, set your trading parameters, and let the bot navigate the market 24/7 without manual intervention.</p>
        </div>
      </section>

      {/* 4. Social Proof / Stats */}
      <section className="stats-bar">
        <div className="stat-item">
          {/* Changed from "Assets Managed" to "Trading Volume" to fit the bot theme */}
          <h4>$450M+</h4>
          <span>Trading Volume</span>
        </div>
        <div className="stat-item">
          <h4>99.9%</h4>
          <span>Uptime</span>
        </div>
        <div className="stat-item">
          <h4>0.0%</h4>
          <span>Liquidation Risk</span>
        </div>
      </section>

      {/* 5. Footer */}
      <footer className="lp-footer">
        <p>© 2026 Nexus Labs. Built on Arbitrum.</p>
      </footer>
    </div>
  );
};

export default CoinBotLandingPage;
