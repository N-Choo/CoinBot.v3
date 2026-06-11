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
      <div className="flex w-full min-w-0">
        {[1, 2, 3].map(i => (
          <div key={i} className="flex-1 min-w-0 p-3 sm:p-5 sm:border-r border-border-light last:border-r-0">
            <Skeleton variant="text-sm" width="60%" />
            <Skeleton variant="text-lg" width="70%" />
            <Skeleton variant="text-sm" width="40%" />
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="flex w-full min-w-0">
      {stats.map((stat, idx) => (
        <div key={idx} className="flex-1 min-w-0 p-3 sm:p-5 sm:border-r border-border-light last:border-r-0 border-b sm:border-b-0" tabIndex={0}>
          <div className="flex justify-between items-center">
            <div className="flex flex-col">
              <span className="text-[10px] text-text-muted font-semibold uppercase tracking-[0.3px]">{stat.label}</span>
            </div>
            <div className="w-8 h-8 rounded flex items-center justify-center border border-border-light shrink-0">
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
          <div className="text-lg sm:text-[22px] font-extrabold font-mono text-text-main leading-tight my-1 sm:my-1.5 truncate">{stat.value}</div>
          <div className="flex items-center gap-1.5 text-xs font-semibold">
            <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${stat.isUp ? 'bg-up' : 'bg-down'}`} />
            <span className={`${stat.isUp ? 'text-up-text' : 'text-down-text'}`}>{stat.change}</span>
            <span className="text-text-muted font-medium text-[10px]">24h</span>
          </div>
        </div>
      ))}
    </div>
  )
}
