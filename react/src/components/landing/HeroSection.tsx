import { useNavigate } from 'react-router-dom'
import { useAuth } from '../../hooks/useAuth'

export default function HeroSection() {
  const navigate = useNavigate()
  const { isAuthenticated, login } = useAuth()

  const handleStart = async () => {
    if (isAuthenticated || await login()) {
      navigate('/trading')
    }
  }

  return (
    <section className="hero">
      <div className="hero-inner">
        <div className="hero-body">
          <div className="hero-status">
            <span className="hero-status-dot" />
            Bot Online — $127K P&L Today
          </div>

          <h1>
            Trade smarter.<br />
            <span className="hero-em">Earn passively.</span>
          </h1>

          <p className="hero-sub">
            CoinBot scans the market 24/7 — buying low, selling high, and staking idle
            capital for passive yield. No keys, no manual trades, zero hands-on effort.
          </p>

          <div className="hero-actions">
            <button className="btn-p" onClick={handleStart}>
              Start Trading
            </button>
            <button className="btn-s" onClick={() => window.open('https://github.com/n-choo/CoinBot.v3', '_blank')}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/></svg>
              GitHub
            </button>
          </div>
        </div>

        <div className="hero-visual">
          <div className="hero-preview">
            <div className="hero-preview-header">
              <span className="hero-preview-label">Portfolio Value</span>
              <span className="hero-preview-value">$247,832.41</span>
              <span className="hero-preview-change price-up">+$12,430.22 (5.28%)</span>
            </div>

            <div className="hero-preview-chart">
              <svg viewBox="0 0 200 50" className="hero-sparkline" preserveAspectRatio="none">
                <defs>
                  <linearGradient id="sparkGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="var(--color-primary)" stopOpacity="0.3" />
                    <stop offset="100%" stopColor="var(--color-primary)" stopOpacity="0" />
                  </linearGradient>
                </defs>
                <path d="M0,40 L10,38 L20,42 L30,35 L40,38 L50,28 L60,32 L70,22 L80,26 L90,16 L100,20 L110,10 L120,14 L130,6 L140,10 L150,4 L160,8 L170,2 L180,5 L200,2" fill="none" stroke="var(--color-primary)" strokeWidth="2" vectorEffect="non-scaling-stroke" />
                <path d="M0,40 L10,38 L20,42 L30,35 L40,38 L50,28 L60,32 L70,22 L80,26 L90,16 L100,20 L110,10 L120,14 L130,6 L140,10 L150,4 L160,8 L170,2 L180,5 L200,2 L200,50 L0,50 Z" fill="url(#sparkGradient)" />
              </svg>
            </div>

            <div className="hero-preview-positions">
              <div className="hero-preview-pos">
                <span className="hero-preview-pos-name">BTC/USDT</span>
                <span className="price-up">+$3,241.50</span>
              </div>
              <div className="hero-preview-pos">
                <span className="hero-preview-pos-name">ETH/USDT</span>
                <span className="price-up">+$1,890.23</span>
              </div>
              <div className="hero-preview-pos">
                <span className="hero-preview-pos-name">SOL/USDT</span>
                <span className="price-down">-$427.80</span>
              </div>
            </div>
          </div>

          <div className="hero-stats">
            <div className="hero-stat">
              <span className="hero-stat-num">$450M+</span>
              <span className="hero-stat-label">Trading Volume</span>
            </div>
            <div className="hero-stat-div" />
            <div className="hero-stat">
              <span className="hero-stat-num">99.9%</span>
              <span className="hero-stat-label">Uptime</span>
            </div>
            <div className="hero-stat-div" />
            <div className="hero-stat">
              <span className="hero-stat-num">0.0%</span>
              <span className="hero-stat-label">Liquidation Risk</span>
            </div>
            <div className="hero-stat-div" />
            <div className="hero-stat">
              <span className="hero-stat-num">10K+</span>
              <span className="hero-stat-label">Active Users</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
