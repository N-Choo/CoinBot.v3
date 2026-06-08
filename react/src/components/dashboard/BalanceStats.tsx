import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

const stats = [
  { label: 'Total Balance', value: '$124,582.40', change: '+3.24%', isUp: true },
  { label: 'Free Balance', value: '$48,230.15', change: '+1.87%', isUp: true },
  { label: 'Locked Balance', value: '$76,352.25', change: '-0.52%', isUp: false },
]

export default function BalanceStats() {
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 600)
    return () => clearTimeout(t)
  }, [])

  if (loading) {
    return (
      <div className="balance-stats-grid">
        {[1, 2, 3].map(i => (
          <div key={i} className="dash-panel">
            <div className="balance-card-header">
              <Skeleton variant="text-sm" width="60%" />
              <Skeleton variant="circle" width="28px" height="28px" />
            </div>
            <Skeleton variant="text-lg" width="70%" className="dash-value-lg" />
            <Skeleton variant="text-sm" width="40%" />
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="balance-stats-grid">
      {stats.map((stat, idx) => (
        <div key={idx} className="dash-panel" tabIndex={0}>
          <div className="balance-card-header">
            <div className="balance-card-header-left">
              <span className="dash-title-sm">{stat.label}</span>
            </div>
            <div className="icon-box">
              {idx === 0 && (
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
                </svg>
              )}
              {idx === 1 && (
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                </svg>
              )}
              {idx === 2 && (
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
                </svg>
              )}
            </div>
          </div>
          <div className="dash-value-lg">{stat.value}</div>
          <div className="balance-change-row">
            <span className={`change-dot ${stat.isUp ? 'change-dot-up' : 'change-dot-down'}`} />
            <span className={stat.isUp ? 'price-up-bg' : 'price-down-bg'}>{stat.change}</span>
            <span className="balance-change-label">24h</span>
          </div>
        </div>
      ))}
    </div>
  )
}
