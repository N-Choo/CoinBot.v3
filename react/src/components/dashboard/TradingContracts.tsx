import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

const logoUrl = (symbol: string) =>
  `https://cdn.jsdelivr.net/gh/atomiclabs/cryptocurrency-icons@master/128/color/${symbol.toLowerCase()}.png`

interface Contract {
  id: number
  pair: string
  status: 'Active' | 'Inactive'
  total: number
  stake: number
  trading: number
  free: number
  pnl: number
  roe: number
}

const contracts: Contract[] = [
  { id: 1, pair: 'BTC/USD', status: 'Active', total: 15000, stake: 5000, trading: 8000, free: 2000, pnl: 1240, roe: 24.8 },
  { id: 2, pair: 'ETH/USD', status: 'Active', total: 8400, stake: 4000, trading: 3400, free: 1000, pnl: -320, roe: -8.0 },
  { id: 3, pair: 'SOL/USD', status: 'Inactive', total: 3200, stake: 3200, trading: 0, free: 0, pnl: 0, roe: 0 },
  { id: 4, pair: 'AVAX/USD', status: 'Active', total: 5600, stake: 2000, trading: 3000, free: 600, pnl: 480, roe: 24.0 },
]

export default function TradingContracts() {
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState('All')
  const [selected, setSelected] = useState<Contract | null>(null)
  const [addAmount, setAddAmount] = useState('')
  const [withdrawAmount, setWithdrawAmount] = useState('')

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 500)
    return () => clearTimeout(t)
  }, [])

  const filtered = contracts.filter(c => filter === 'All' || c.status === filter)

  const closeDetail = () => {
    setSelected(null)
    setAddAmount('')
    setWithdrawAmount('')
  }

  if (selected) {
    return (
      <div className="p-3 sm:p-5 border-t border-border-light">
        <div className="flex items-center justify-between border-b border-border-light pb-3 mb-4">
          <div className="flex items-center gap-2">
            <button className="bg-transparent border-none text-text-muted cursor-pointer p-0 flex items-center text-sm hover:text-text-main" onClick={closeDetail}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="15 18 9 12 15 6" /></svg>
            </button>
            <img className="w-5 h-5 rounded-full shrink-0 object-cover" src={logoUrl(selected.pair.split('/')[0])} alt="" onError={e => { (e.target as HTMLImageElement).style.display = 'none' }} />
            <h3 className="m-0 text-text-main text-sm font-semibold">{selected.pair}</h3>
            <span className={`inline-flex items-center gap-1.5 text-[11px] font-semibold ${selected.status === 'Active' ? 'text-up' : 'text-text-muted'}`}>
              <span className={`w-1.5 h-1.5 rounded-full ${selected.status === 'Active' ? 'bg-up' : 'bg-text-muted'}`} />
              {selected.status}
            </span>
          </div>
          <span className={`text-sm font-mono font-bold ${selected.pnl > 0 ? 'text-up-text' : selected.pnl < 0 ? 'text-down-text' : ''}`}>
            {selected.pnl > 0 ? '+' : ''}${selected.pnl.toLocaleString()}
          </span>
        </div>

        <div className="bg-[rgba(255,255,255,0.02)] border border-border-light rounded-lg p-3 grid grid-cols-2 gap-x-6 gap-y-2 text-[11px] mb-4">
          <div className="flex justify-between">
            <span className="text-text-muted">Total Size</span>
            <span className="text-text-main font-semibold font-mono">${selected.total.toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Stake</span>
            <span className="text-text-main font-mono">${selected.stake.toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Trading</span>
            <span className="text-text-main font-mono">${selected.trading.toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Free</span>
            <span className="text-text-main font-mono">${selected.free.toLocaleString()}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">ROE</span>
            <span className={`font-mono font-semibold ${selected.roe > 0 ? 'text-up-text' : selected.roe < 0 ? 'text-down-text' : ''}`}>
              {selected.roe > 0 ? '+' : ''}{selected.roe}%
            </span>
          </div>
        </div>

        <div className="space-y-3">
          <div>
            <label className="block text-[10px] text-text-muted mb-1 font-semibold uppercase tracking-[0.3px]">Add Funds</label>
            <div className="flex gap-2">
              <input type="number" placeholder="0.00" value={addAmount} onChange={e => setAddAmount(e.target.value)}
                className="flex-1 bg-[rgba(255,255,255,0.03)] border border-border-light rounded px-3 py-2 text-text-main text-xs outline-none font-mono" />
              <button className="px-4 py-2 border-none rounded bg-accent text-white text-[11px] font-bold cursor-pointer transition-opacity hover:opacity-90 disabled:opacity-40"
                disabled={!addAmount || parseFloat(addAmount) <= 0}>
                ADD
              </button>
            </div>
          </div>
          <div>
            <label className="block text-[10px] text-text-muted mb-1 font-semibold uppercase tracking-[0.3px]">Withdraw</label>
            <div className="flex gap-2">
              <input type="number" placeholder="0.00" value={withdrawAmount} onChange={e => setWithdrawAmount(e.target.value)}
                className="flex-1 bg-[rgba(255,255,255,0.03)] border border-border-light rounded px-3 py-2 text-text-main text-xs outline-none font-mono" />
              <button className="px-4 py-2 border-none rounded bg-down text-white text-[11px] font-bold cursor-pointer transition-opacity hover:opacity-90 disabled:opacity-40"
                disabled={!withdrawAmount || parseFloat(withdrawAmount) <= 0}>
                WITHDRAW
              </button>
            </div>
          </div>
        </div>
      </div>
    )
  }

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
                <tr key={c.id} className="transition-colors hover:bg-[rgba(255,255,255,0.025)] cursor-pointer"
                  onClick={() => setSelected(c)}>
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
