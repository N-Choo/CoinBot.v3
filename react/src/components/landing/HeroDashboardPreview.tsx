export default function HeroDashboardPreview() {
  return (
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
          <path
            d="M0,40 L10,38 L20,42 L30,35 L40,38 L50,28 L60,32 L70,22 L80,26 L90,16 L100,20 L110,10 L120,14 L130,6 L140,10 L150,4 L160,8 L170,2 L180,5 L200,2"
            fill="none"
            stroke="var(--color-primary)"
            strokeWidth="2"
            vectorEffect="non-scaling-stroke"
          />
          <path
            d="M0,40 L10,38 L20,42 L30,35 L40,38 L50,28 L60,32 L70,22 L80,26 L90,16 L100,20 L110,10 L120,14 L130,6 L140,10 L150,4 L160,8 L170,2 L180,5 L200,2 L200,50 L0,50 Z"
            fill="url(#sparkGradient)"
          />
        </svg>
      </div>

      <div className="hero-preview-positions">
        <div className="hero-preview-pos">
          <div className="hero-preview-pos-left">
            <span className="hero-preview-pos-name">BTC/USDT</span>
            <span className="hero-preview-pos-size">0.42 BTC</span>
          </div>
          <span className="hero-preview-pos-pnl price-up">+$3,241.50</span>
        </div>
        <div className="hero-preview-pos">
          <div className="hero-preview-pos-left">
            <span className="hero-preview-pos-name">ETH/USDT</span>
            <span className="hero-preview-pos-size">4.8 ETH</span>
          </div>
          <span className="hero-preview-pos-pnl price-up">+$1,890.23</span>
        </div>
        <div className="hero-preview-pos">
          <div className="hero-preview-pos-left">
            <span className="hero-preview-pos-name">SOL/USDT</span>
            <span className="hero-preview-pos-size">120 SOL</span>
          </div>
          <span className="hero-preview-pos-pnl price-down">-$427.80</span>
        </div>
      </div>
    </div>
  )
}
