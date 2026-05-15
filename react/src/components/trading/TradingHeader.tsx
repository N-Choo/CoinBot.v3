interface TradingHeaderProps {
  selectedPair: string
  onStartTrading: () => void
}

export default function TradingHeader({ selectedPair, onStartTrading }: TradingHeaderProps) {
  return (
    <div className="chart-header-row">
      <div className="pair-info">
        <div className="pair-title-group">
          <h1 className="pair-title">{selectedPair}</h1>
          <span className="current-price">64,230.50</span>
        </div>

        <div className="pair-stats">
          <div className="stat-block">
            <span className="stat-label">24h Change</span>
            <span className="stat-value price-up">+2.40%</span>
          </div>
          <div className="stat-divider"></div>
          <div className="stat-block">
            <span className="stat-label">24h Volume</span>
            <span className="stat-value">1.2B</span>
          </div>
        </div>
      </div>

      <button className="start-bot-btn desktop-only" onClick={onStartTrading}>
        <span>START TRADING</span>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <line x1="5" y1="12" x2="19" y2="12"></line>
          <polyline points="12 5 19 12 12 19"></polyline>
        </svg>
      </button>
    </div>
  )
}
