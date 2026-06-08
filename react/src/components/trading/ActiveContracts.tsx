import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

interface Contract {
  id: number
  pair: string
  status: 'Active' | 'Paused' | 'Inactive'
  value: number
  pnl: number
}

const MOCK: Contract[] = [
  { id: 1, pair: 'BTC/USDT', status: 'Active', value: 15000, pnl: 312.50 },
  { id: 2, pair: 'ETH/USDT', status: 'Active', value: 8400, pnl: -42.30 },
  { id: 3, pair: 'SOL/USDT', status: 'Paused', value: 3200, pnl: 0 },
]

const statusColors: Record<string, string> = {
  Active: 'var(--color-up)',
  Paused: 'var(--text-muted)',
  Inactive: 'var(--color-down)',
}

export default function ActiveContracts() {
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 400)
    return () => clearTimeout(t)
  }, [])

  return (
    <div className="active-contracts">
      <div className="ac-header">
        <span className="ac-title">Active Contracts</span>
        {loading ? (
          <Skeleton variant="text-sm" width="60px" />
        ) : (
          <span className="ac-count">{MOCK.length} running</span>
        )}
      </div>

      {loading ? (
        <div className="ac-scroll">
          {[1, 2, 3].map(i => (
            <div key={i} className="ac-card" style={{ border: 'none', background: 'transparent', padding: 0 }}>
              <Skeleton variant="card" width="140px" height="56px" />
            </div>
          ))}
        </div>
      ) : MOCK.length === 0 ? (
        <div className="ac-empty">No active contracts</div>
      ) : (
        <div className="ac-scroll">
          {MOCK.map(c => (
            <div key={c.id} className="ac-card">
              <div className="ac-card-top">
                <span className="ac-pair">{c.pair.split('/')[0]}</span>
                <span className="ac-status" style={{ color: statusColors[c.status] }}>
                  ● {c.status}
                </span>
              </div>
              <div className="ac-card-bottom">
                <span className="ac-value">${c.value.toLocaleString()}</span>
                <span className={`ac-pnl ${c.pnl >= 0 ? 'up' : 'down'}`}>
                  {c.pnl >= 0 ? '+' : ''}${c.pnl.toFixed(2)}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
