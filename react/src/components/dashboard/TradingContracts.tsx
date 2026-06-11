import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

const logoUrl = (symbol: string) =>
  `https://cdn.jsdelivr.net/gh/atomiclabs/cryptocurrency-icons@master/128/color/${symbol.toLowerCase()}.png`

const contracts = [
  { id: 1, pair: 'BTC/USD', status: 'Active', total: 15000, stake: 5000, trading: 8000, free: 2000, pnl: 1240, roe: 24.8 },
  { id: 2, pair: 'ETH/USD', status: 'Active', total: 8400, stake: 4000, trading: 3400, free: 1000, pnl: -320, roe: -8.0 },
  { id: 3, pair: 'SOL/USD', status: 'Inactive', total: 3200, stake: 3200, trading: 0, free: 0, pnl: 0, roe: 0 },
  { id: 4, pair: 'AVAX/USD', status: 'Active', total: 5600, stake: 2000, trading: 3000, free: 600, pnl: 480, roe: 24.0 },
]

export default function TradingContracts() {
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState('All')

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 500)
    return () => clearTimeout(t)
  }, [])

  const filtered = contracts.filter(c => filter === 'All' || c.status === filter)

  return (
    <div className="p-3 sm:p-5 border-t border-border-light">
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3 mb-4">
        <div className="flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-neon-teal shrink-0" />
          <h3 className="m-0 text-text-main text-sm font-semibold">Trading Contracts</h3>
        </div>
        <div className="flex gap-3 items-center flex-wrap">
          {['All', 'Active', 'Inactive'].map(t => (
            <button
              key={t}
              className={`bg-transparent border px-2.5 py-1 rounded text-[10px] font-semibold cursor-pointer transition-all ${filter === t ? 'border-neon-teal text-neon-teal' : 'border-border-light text-text-muted hover:text-text-main'}`}
              onClick={() => setFilter(t)}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full border-collapse font-mono text-xs">
          <thead>
            <tr>
              <th className="text-left px-3 py-2 text-[10px] font-semibold text-text-muted uppercase tracking-[0.3px] border-b border-border-light">Pair</th>
              <th className="text-left px-3 py-2 text-[10px] font-semibold text-text-muted uppercase tracking-[0.3px] border-b border-border-light">Status</th>
              <th className="text-right px-3 py-2 text-[10px] font-semibold text-text-muted uppercase tracking-[0.3px] border-b border-border-light">Size</th>
              <th className="text-right px-3 py-2 text-[10px] font-semibold text-text-muted uppercase tracking-[0.3px] border-b border-border-light">P&L</th>
              <th className="text-right px-3 py-2 text-[10px] font-semibold text-text-muted uppercase tracking-[0.3px] border-b border-border-light">ROE</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              [1, 2, 3, 4].map(i => (
                <tr key={i}>
                  <td className="px-3 py-2.5"><Skeleton variant="text" width="100px" /></td>
                  <td className="px-3 py-2.5"><Skeleton variant="text-sm" width="60px" /></td>
                  <td className="px-3 py-2.5 text-right"><Skeleton variant="text" width="70px" /></td>
                  <td className="px-3 py-2.5 text-right"><Skeleton variant="text" width="70px" /></td>
                  <td className="px-3 py-2.5 text-right"><Skeleton variant="text" width="50px" /></td>
                </tr>
              ))
            ) : filtered.length === 0 ? (
              <tr><td colSpan={5} className="text-center py-8 text-text-muted text-xs">No contracts found</td></tr>
            ) : (
              filtered.map(c => (
                <tr key={c.id} className="transition-colors hover:bg-[rgba(255,255,255,0.025)]">
                  <td className="px-3 py-2.5 border-b border-border-light">
                    <div className="flex items-center gap-2">
                      <img
                        className="w-5 h-5 rounded-full shrink-0 object-cover"
                        src={logoUrl(c.pair.split('/')[0])}
                        alt={c.pair.split('/')[0]}
                        onError={e => { (e.target as HTMLImageElement).style.display = 'none' }}
                      />
                      <span className="text-xs font-semibold text-text-main">{c.pair}</span>
                    </div>
                  </td>
                  <td className="px-3 py-2.5 border-b border-border-light">
                    <span className={`inline-flex items-center gap-1.5 text-[11px] font-semibold ${c.status === 'Active' ? 'text-up' : 'text-text-muted'}`}>
                      <span className={`w-1.5 h-1.5 rounded-full ${c.status === 'Active' ? 'bg-up' : 'bg-text-muted'}`} />
                      {c.status}
                    </span>
                  </td>
                  <td className="px-3 py-2.5 border-b border-border-light text-right text-text-main">${c.total.toLocaleString()}</td>
                  <td className="px-3 py-2.5 border-b border-border-light text-right">
                    <span className={`text-xs font-semibold ${c.pnl > 0 ? 'text-up-text' : c.pnl < 0 ? 'text-down-text' : ''}`}>
                      {c.pnl > 0 ? '+' : ''}${c.pnl.toLocaleString()}
                    </span>
                  </td>
                  <td className="px-3 py-2.5 border-b border-border-light text-right">
                    <span className={`text-xs font-semibold ${c.roe > 0 ? 'text-up-text' : c.roe < 0 ? 'text-down-text' : ''}`}>
                      {c.roe > 0 ? '+' : ''}{c.roe}%
                    </span>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
