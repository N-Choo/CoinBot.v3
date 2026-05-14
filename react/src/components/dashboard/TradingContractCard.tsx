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
    <article className="dash-panel" style={{ width: '100%', margin: 0 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
          <div className="icon-box" style={{ fontWeight: 'bold', color: 'var(--color-primary)' }}>
            {symbol}
          </div>
          <div>
            <h4 style={{ margin: 0, fontSize: 16, color: 'var(--text-main)' }}>{data.pair}</h4>
            <div className="data-label">Total: ${total.toLocaleString()}</div>
          </div>
        </div>
        <span className={`badge ${isActive ? 'active' : 'inactive'}`}>
          {(data.status || 'UNKNOWN').toUpperCase()}
        </span>
      </div>

      <div className="progress-track">
        <div style={{ width: `${stakePct}%`, background: 'var(--color-accent)', transition: 'width 1s ease' }} title="Staked" />
        <div style={{ width: `${tradePct}%`, background: 'var(--color-primary)', transition: 'width 1s ease' }} title="Trading" />
        <div style={{ width: `${freePct}%`, background: 'var(--text-muted)', transition: 'width 1s ease' }} title="Free" />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <div>
          <div className="data-label">Staked</div>
          <div className="data-value" style={{ color: 'var(--color-accent)' }}>${stake.toLocaleString()}</div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div className="data-label">Trading</div>
          <div className="data-value" style={{ color: 'var(--color-primary)' }}>${trading.toLocaleString()}</div>
        </div>
      </div>
    </article>
  )
}
