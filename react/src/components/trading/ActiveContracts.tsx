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
  Active: 'text-up',
  Paused: 'text-text-muted',
  Inactive: 'text-down',
}

export default function ActiveContracts() {
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 400)
    return () => clearTimeout(t)
  }, [])

  return (
    <div className="flex flex-col bg-bg-panel border-t border-border-light">
      <div className="flex items-center justify-between px-3 sm:px-4 py-2.5">
        <span className="text-[13px] font-semibold text-text-main">Active Contracts</span>
        {loading ? (
          <Skeleton variant="text-sm" width="60px" />
        ) : (
          <span className="text-[11px] text-text-muted">{MOCK.length} running</span>
        )}
      </div>

      {loading ? (
        <div className="flex gap-2 px-3 sm:px-4 pb-3 overflow-x-auto" style={{ scrollbarWidth: 'thin' }}>
          {[1, 2, 3].map(i => (
            <Skeleton key={i} variant="card" width="140px" height="56px" />
          ))}
        </div>
      ) : MOCK.length === 0 ? (
        <div className="px-4 pb-4 text-xs text-text-muted text-center">No active contracts</div>
      ) : (
        <div className="flex gap-2 px-3 sm:px-4 pb-3 overflow-x-auto" style={{ scrollbarWidth: 'thin' }}>
          {MOCK.map(c => (
            <div key={c.id} className="flex flex-col shrink-0 gap-0.5 bg-bg-input rounded-lg px-3 py-2 min-w-[150px]">
              <div className="flex items-center justify-between">
                <span className="text-[11px] font-bold text-text-main">{c.pair.split('/')[0]}</span>
                <span className={`text-[10px] font-semibold ${statusColors[c.status]}`}>● {c.status}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[12px] font-mono font-bold text-text-main">${c.value.toLocaleString()}</span>
                <span className={`text-[11px] font-mono font-bold ${c.pnl >= 0 ? 'text-up' : 'text-down'}`}>
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
