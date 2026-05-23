export default function TradingContractCard({ data }: { data?: { status?: string; pair?: string; stake?: number; trading?: number; free?: number } }) {
  if (!data) return null

  const isActive = data.status === 'Active'
  const stake = data.stake || 0
  const trading = data.trading || 0
  const free = data.free || 0
  const total = stake + trading + free
  const stakePct = total ? (stake / total) * 100 : 0
  const tradePct = total ? (trading / total) * 100 : 0
  const freePct = total ? (free / total) * 100 : 0
  const symbol = data.pair ? data.pair.split('/')[0].charAt(0) : '?'

  return (
    <article className="dash-panel contract-card-wrap">
      <div className="contract-card-header">
        <div className="contract-card-pair">
          <div className="icon-box contract-card-symbol">
            {symbol}
          </div>
          <div>
            <h4 className="contract-card-title">{data.pair}</h4>
            <div className="data-label">Total: ${total.toLocaleString()}</div>
          </div>
        </div>
        <span className={`badge ${isActive ? 'active' : 'inactive'}`}>
          {(data.status || 'UNKNOWN').toUpperCase()}
        </span>
      </div>

      <div className="progress-track">
        <div style={{ width: `${stakePct}%`, background: '#f87171', transition: 'width 1s ease' }} title="Staked" />
        <div style={{ width: `${tradePct}%`, background: 'var(--color-primary)', transition: 'width 1s ease' }} title="Trading" />
        <div style={{ width: `${freePct}%`, background: 'var(--text-muted)', transition: 'width 1s ease' }} title="Free" />
      </div>

      <div className="contract-card-stats">
        <div>
          <div className="data-label">Staked</div>
          <div className="data-value" style={{ color: '#f87171' }}>${stake.toLocaleString()}</div>
        </div>
        <div className="contract-card-stats-right">
          <div className="data-label">Trading</div>
          <div className="data-value" style={{ color: 'var(--color-primary)' }}>${trading.toLocaleString()}</div>
        </div>
      </div>
    </article>
  )
}
