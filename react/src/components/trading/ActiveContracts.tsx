import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

interface Contract {
  id: number
  pair: string
  side: 'Long' | 'Short'
  status: 'Active' | 'Paused' | 'Inactive'
  size: number
  entryPrice: number
  markPrice: number
  pnl: number
  liquidation: number
}

const MOCK: Contract[] = [
  { id: 1, pair: 'BTC/USDT', side: 'Long', status: 'Active', size: 1.5, entryPrice: 64230, markPrice: 64850, pnl: 930.00, liquidation: 58900 },
  { id: 2, pair: 'ETH/USDT', side: 'Short', status: 'Active', size: 12, entryPrice: 3420, markPrice: 3380, pnl: 480.00, liquidation: 3650 },
  { id: 3, pair: 'SOL/USDT', side: 'Long', status: 'Paused', size: 25, entryPrice: 142.75, markPrice: 141.20, pnl: -38.75, liquidation: 128 },
  { id: 4, pair: 'AVAX/USDT', side: 'Short', status: 'Active', size: 50, entryPrice: 35.40, markPrice: 34.80, pnl: 30.00, liquidation: 42 },
  { id: 5, pair: 'DOGE/USDT', side: 'Long', status: 'Active', size: 5000, entryPrice: 0.145, markPrice: 0.151, pnl: 30.00, liquidation: 0.12 },
]

const pnlPct = (entry: number, mark: number, side: 'Long' | 'Short'): number => {
  const raw = ((mark - entry) / entry) * 100
  return side === 'Long' ? raw : -raw
}

const logoUrl = (symbol: string) =>
  `https://cdn.jsdelivr.net/gh/atomiclabs/cryptocurrency-icons@master/128/color/${symbol.toLowerCase()}.png`

export default function ActiveContracts() {
  const [loading, setLoading] = useState(true)
  const [selected, setSelected] = useState<Contract | null>(null)
  const [addAmount, setAddAmount] = useState('')
  const [withdrawAmount, setWithdrawAmount] = useState('')
  const [ticker, setTicker] = useState(0)
  const [flash, setFlash] = useState('')

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 400)
    return () => clearTimeout(t)
  }, [])

  useEffect(() => {
    const timeouts: ReturnType<typeof setTimeout>[] = []
    const interval = setInterval(() => {
      const change = (Math.random() * 40 - 20)
      setTicker(prev => +(prev + change).toFixed(2))
      setFlash(change >= 0 ? 'text-up' : 'text-down')
      timeouts.push(setTimeout(() => setFlash(''), 400))
    }, 3000)
    return () => {
      clearInterval(interval)
      timeouts.forEach(clearTimeout)
    }
  }, [])

  const closeDetail = () => {
    setSelected(null)
    setAddAmount('')
    setWithdrawAmount('')
  }

  if (selected) {
    const pct = pnlPct(selected.entryPrice, selected.markPrice, selected.side)
    return (
      <div className="flex flex-col bg-bg-panel border-t border-border-light">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border-light">
          <div className="flex items-center gap-2">
            <button className="bg-transparent border-none text-text-muted cursor-pointer p-0 flex items-center text-sm hover:text-text-main" onClick={closeDetail}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="15 18 9 12 15 6" /></svg>
            </button>
            <img className="w-5 h-5 rounded-full shrink-0 object-cover" src={logoUrl(selected.pair.split('/')[0])} alt="" onError={e => { (e.target as HTMLImageElement).style.display = 'none' }} />
            <span className="text-[13px] font-semibold text-text-main">{selected.pair.split('/')[0]}</span>
            <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded ${selected.side === 'Long' ? 'bg-up/10 text-up' : 'bg-down/10 text-down'}`}>{selected.side}</span>
            <span className={`w-1.5 h-1.5 rounded-full ${selected.status === 'Active' ? 'bg-up' : selected.status === 'Paused' ? 'bg-text-muted' : 'bg-down'}`} />
          </div>
          <div className="flex items-center gap-3">
            <span className={`text-[13px] font-mono font-bold transition-colors duration-300 ${flash || (selected.pnl >= 0 ? 'text-up' : 'text-down')}`}>
              {selected.pnl >= 0 ? '+' : ''}${selected.pnl.toFixed(2)}
            </span>
          </div>
        </div>

        <div className="p-4 space-y-3 overflow-y-auto" style={{ scrollbarWidth: 'thin' }}>
          <div className="bg-[rgba(255,255,255,0.02)] border border-border-light rounded-lg p-3 grid grid-cols-2 gap-x-6 gap-y-2 text-[11px]">
            <div className="flex justify-between">
              <span className="text-text-muted">Size</span>
              <span className="text-text-main font-semibold font-mono">{selected.size.toFixed(2)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Entry</span>
              <span className="text-text-main font-mono">${selected.entryPrice.toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Mark</span>
              <span className="text-text-main font-mono">${selected.markPrice.toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">PnL %</span>
              <span className={`font-mono font-semibold ${pct >= 0 ? 'text-up' : 'text-down'}`}>{pct >= 0 ? '+' : ''}{pct.toFixed(2)}%</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Liquidation</span>
              <span className="text-down font-mono">${selected.liquidation.toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Status</span>
              <span className={`font-semibold ${selected.status === 'Active' ? 'text-up' : selected.status === 'Paused' ? 'text-text-muted' : 'text-down'}`}>{selected.status}</span>
            </div>
          </div>

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
    <div className="flex flex-col bg-bg-panel border-t border-border-light">
      <div className="flex items-center justify-between px-4 py-2.5">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-semibold text-text-main">Open Positions</span>
          <span className={`inline-flex items-center gap-1 text-[11px] font-mono font-semibold transition-colors duration-300 ${flash || 'text-text-muted'}`}>
            {ticker !== 0 && (
              <span className="inline-block text-xs" style={{ animation: 'ticker-pop 0.3s ease' }}>
                {ticker > 0 ? `+${ticker}` : ticker}
              </span>
            )}
          </span>
        </div>
        <div className="flex items-center gap-3">
          {loading ? (
            <Skeleton variant="text-sm" width="60px" />
          ) : (
            <>
              <span className="text-[11px] text-text-muted">{MOCK.length} contracts</span>
              <span className="text-[11px] font-semibold text-up">
                +{MOCK.reduce((s, c) => s + Math.max(c.pnl, 0), 0).toLocaleString()}
              </span>
              <span className="text-[11px] font-semibold text-down">
                {MOCK.reduce((s, c) => s + Math.min(c.pnl, 0), 0).toLocaleString()}
              </span>
            </>
          )}
        </div>
      </div>

      {loading ? (
        <div className="px-4 pb-3 space-y-1.5">
          {[1, 2, 3].map(i => (
            <Skeleton key={i} variant="card" width="100%" height="36px" />
          ))}
        </div>
      ) : MOCK.length === 0 ? (
        <div className="px-4 pb-4 text-xs text-text-muted text-center">No open positions</div>
      ) : (
        <div className="overflow-y-auto max-h-[220px] overflow-x-auto" style={{ scrollbarWidth: 'thin' }}>
          <table className="w-full text-[11px] border-collapse">
            <thead>
              <tr className="text-text-muted border-b border-border-light">
                <th className="text-left font-medium px-3 py-1.5">Pair</th>
                <th className="text-right font-medium px-3 py-1.5">Side</th>
                <th className="text-right font-medium px-3 py-1.5">Size</th>
                <th className="text-right font-medium px-3 py-1.5 hidden sm:table-cell">Entry</th>
                <th className="text-right font-medium px-3 py-1.5 hidden sm:table-cell">Mark</th>
                <th className="text-right font-medium px-3 py-1.5">PnL</th>
                <th className="text-right font-medium px-3 py-1.5 hidden lg:table-cell">Liq.</th>
              </tr>
            </thead>
            <tbody>
              {MOCK.map(c => (
                <tr key={c.id} className="border-b border-border-light hover:bg-[rgba(255,255,255,0.03)] transition-colors cursor-pointer"
                  onClick={() => setSelected(c)}>
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-2">
                      <img className="w-4 h-4 rounded-full shrink-0 object-cover" src={logoUrl(c.pair.split('/')[0])} alt="" onError={e => { (e.target as HTMLImageElement).style.display = 'none' }} />
                      <span className="font-semibold text-text-main">{c.pair.split('/')[0]}</span>
                      <span className={`w-1.5 h-1.5 rounded-full ${c.status === 'Active' ? 'bg-up' : c.status === 'Paused' ? 'bg-text-muted' : 'bg-down'}`} />
                    </div>
                  </td>
                  <td className="px-3 py-2 text-right">
                    <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded ${c.side === 'Long' ? 'bg-up/10 text-up' : 'bg-down/10 text-down'}`}>
                      {c.side}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-text-main font-semibold">{c.size.toFixed(2)}</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden sm:table-cell">${c.entryPrice.toLocaleString()}</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden sm:table-cell">${c.markPrice.toLocaleString()}</td>
                  <td className={`px-3 py-2 text-right font-mono font-bold ${c.pnl >= 0 ? 'text-up' : 'text-down'}`}>
                    {c.pnl >= 0 ? '+' : ''}${c.pnl.toFixed(2)}
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden lg:table-cell">${c.liquidation.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
