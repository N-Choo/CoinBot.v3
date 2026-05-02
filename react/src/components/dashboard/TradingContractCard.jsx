export default function TradingContractCard({ data }) {
  const isActive = data.status === 'Active';
  const total = data.stake + data.trading + data.free;

  // Calculate percentages for the glassy progress bar
  const stakePct = (data.stake / total) * 100;
  const tradePct = (data.trading / total) * 100;
  const freePct = (data.free / total) * 100;

  return (
    <article className="dash-panel" style={{ width: '100%', margin: 0 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '16px' }}>
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          <div className="icon-box" style={{ background: 'var(--bg-active)', fontWeight: 'bold' }}>
            {data.pair.split('/')[0].charAt(0)}
          </div>
          <div>
            <h4 style={{ margin: 0, fontSize: '16px', color: 'var(--text-main)' }}>{data.pair}</h4>
            <span className="data-label">Total: ${total.toLocaleString()}</span>
          </div>
        </div>
        <span className={`badge ${isActive ? 'active' : 'inactive'}`}>
          {data.status.toUpperCase()}
        </span>
      </div>

      {/* Improved Progress Bar with Tooltips/ARIA */}
      <div className="progress-track" role="progressbar" aria-valuenow={stakePct + tradePct} aria-valuemin="0" aria-valuemax="100">
        <div style={{ width: `${stakePct}%`, background: 'var(--color-accent)', transition: 'width 1s ease' }} title="Staked" />
        <div style={{ width: `${tradePct}%`, background: 'var(--color-primary)', transition: 'width 1s ease' }} title="Trading" />
        <div style={{ width: `${freePct}%`, background: 'var(--bg-hover)', transition: 'width 1s ease' }} title="Free" />
      </div>

      {/* Grid Layout for Data Points */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <span className="data-label">Staked</span>
          <span className="data-value" style={{ color: 'var(--color-accent)' }}>${data.stake.toLocaleString()}</span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', textAlign: 'right' }}>
          <span className="data-label">Trading</span>
          <span className="data-value" style={{ color: 'var(--color-primary)' }}>${data.trading.toLocaleString()}</span>
        </div>
      </div>

      {/* Hover action indicator */}
      <div style={{
        marginTop: '16px',
        paddingTop: '12px',
        borderTop: '1px solid var(--border-light)',
        textAlign: 'center'
      }}>
        <button className="nav-link" style={{ fontSize: '11px', width: '100%', opacity: 0.7 }}>
          VIEW DETAILS →
        </button>
      </div>
    </article>
  );
}
