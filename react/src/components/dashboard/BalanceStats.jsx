export default function BalanceStats() {
  const stats = [
    { label: 'Total Balance', value: '$124,582.40', change: '+3.24%', isUp: true },
    { label: 'Free Balance', value: '$48,230.15', change: '+1.87%', isUp: true },
    { label: 'Locked Balance', value: '$76,352.25', change: '-0.52%', isUp: false },
  ];

  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '24px' }}>
      {stats.map((stat, idx) => (
        <div key={idx} className="dash-panel" tabIndex="0">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span className="dash-title-sm">{stat.label}</span>
            <div className="icon-box" aria-hidden="true">
              {idx === 0 && <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></svg>}
              {idx === 1 && <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2" /></svg>}
              {idx === 2 && <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" /></svg>}
            </div>
          </div>
          <div className="dash-value-lg">{stat.value}</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', fontWeight: 600 }}>
            {/* Matches trading page's price-up/price-down badges */}
            <span className={stat.isUp ? 'price-up-bg' : 'price-down-bg'}>
              {stat.change}
            </span>
            <span style={{ color: 'var(--text-muted)', fontWeight: 500 }}>vs last 24h</span>
          </div>
        </div>
      ))}
    </div>
  );
}
