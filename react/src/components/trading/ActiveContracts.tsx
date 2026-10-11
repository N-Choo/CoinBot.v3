import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'
import { getMyContracts, type ContractItem } from '../../services/contract'

const STATUS_COLOR: Record<string, string> = {
  active: 'text-up',
  inactive: 'text-text-muted',
  completed: 'text-accent',
  cancelled: 'text-down',
  failed: 'text-down',
}

const statusDot = (status: string): string =>
  status === 'active'
    ? 'bg-up'
    : status === 'inactive'
      ? 'bg-text-muted'
      : status === 'completed'
        ? 'bg-accent'
        : 'bg-down'

const logoUrl = (symbol: string) =>
  `https://cdn.jsdelivr.net/gh/atomiclabs/cryptocurrency-icons@master/128/color/${symbol.toLowerCase()}.png`

const baseSymbol = (ticker: string) => ticker.split('/')[0]

const FUND_FMT: Intl.NumberFormatOptions = { maximumFractionDigits: 6 }

const fmt = (value: string): string => {
  const n = parseFloat(value)
  return Number.isNaN(n) ? value : n.toLocaleString(undefined, FUND_FMT)
}

const shortDate = (iso: string): string => new Date(iso).toLocaleDateString()

export default function ActiveContracts() {
  const [contracts, setContracts] = useState<ContractItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(false)
  const [selected, setSelected] = useState<ContractItem | null>(null)

  useEffect(() => {
    let active = true
    getMyContracts()
      .then(data => {
        if (active) setContracts(data)
      })
      .catch(() => {
        if (active) setError(true)
      })
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
    }
  }, [])

  if (selected) {
    return (
      <div className="flex flex-col bg-bg-panel border-t border-border-light">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border-light">
          <div className="flex items-center gap-2">
            <button className="bg-transparent border-none text-text-muted cursor-pointer p-0 flex items-center text-sm hover:text-text-main" aria-label="Back" onClick={() => setSelected(null)}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="15 18 9 12 15 6" /></svg>
            </button>
            <img className="w-5 h-5 rounded-full shrink-0 object-cover" src={logoUrl(baseSymbol(selected.ticker))} alt="" onError={e => { (e.target as HTMLImageElement).style.display = 'none' }} />
            <span className="text-[13px] font-semibold text-text-main">{baseSymbol(selected.ticker)}</span>
            <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded capitalize ${STATUS_COLOR[selected.status] ?? 'text-text-muted'}`}>{selected.status}</span>
          </div>
        </div>

        <div className="p-4 space-y-3 overflow-y-auto" style={{ scrollbarWidth: 'thin' }}>
          <div className="bg-[rgba(255,255,255,0.02)] border border-border-light rounded-lg p-3 grid grid-cols-2 gap-x-6 gap-y-2 text-[11px]">
            <div className="flex justify-between">
              <span className="text-text-muted">Ticker</span>
              <span className="text-text-main font-semibold font-mono">{selected.ticker}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Status</span>
              <span className={`font-semibold capitalize ${STATUS_COLOR[selected.status] ?? 'text-text-muted'}`}>{selected.status}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Allocated</span>
              <span className="text-text-main font-mono">${fmt(selected.init_fund)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Available</span>
              <span className="text-text-main font-mono">${fmt(selected.available_fund)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Traded</span>
              <span className="text-text-main font-mono">${fmt(selected.total_traded_fund)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">SL / TP</span>
              <span className="text-text-main font-mono">{selected.sl_pct}% / {selected.tp_pct}%</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Created</span>
              <span className="text-text-main font-mono">{shortDate(selected.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Updated</span>
              <span className="text-text-main font-mono">{shortDate(selected.updated_at)}</span>
            </div>
          </div>
        </div>
      </div>
    )
  }

  const availableTotal = contracts.reduce(
    (sum, c) => sum + (parseFloat(c.available_fund) || 0),
    0,
  )

  return (
    <div className="flex flex-col bg-bg-panel border-t border-border-light">
      <div className="flex items-center justify-between px-4 py-2.5">
        <span className="text-[13px] font-semibold text-text-main">Contracts</span>
        <div className="flex items-center gap-3">
          {loading ? (
            <Skeleton variant="text-sm" width="60px" />
          ) : error ? null : (
            <>
              <span className="text-[11px] text-text-muted">{contracts.length} contracts</span>
              <span className="text-[11px] font-semibold text-text-main">${availableTotal.toLocaleString(undefined, FUND_FMT)} available</span>
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
      ) : error ? (
        <div className="px-4 pb-4 text-xs text-down text-center">Failed to load contracts</div>
      ) : contracts.length === 0 ? (
        <div className="px-4 pb-4 text-xs text-text-muted text-center">No contracts</div>
      ) : (
        <div className="overflow-y-auto max-h-[220px] overflow-x-auto" style={{ scrollbarWidth: 'thin' }}>
          <table className="w-full text-[11px] border-collapse">
            <thead>
              <tr className="text-text-muted border-b border-border-light">
                <th className="text-left font-medium px-3 py-1.5">Pair</th>
                <th className="text-left font-medium px-3 py-1.5">Status</th>
                <th className="text-right font-medium px-3 py-1.5">Allocated</th>
                <th className="text-right font-medium px-3 py-1.5">Available</th>
                <th className="text-right font-medium px-3 py-1.5 hidden sm:table-cell">Traded</th>
                <th className="text-right font-medium px-3 py-1.5 hidden sm:table-cell">SL%</th>
                <th className="text-right font-medium px-3 py-1.5 hidden lg:table-cell">TP%</th>
                <th className="text-right font-medium px-3 py-1.5 hidden lg:table-cell">Created</th>
              </tr>
            </thead>
            <tbody>
              {contracts.map(c => (
                <tr
                  key={c.id}
                  tabIndex={0}
                  className="border-b border-border-light hover:bg-[rgba(255,255,255,0.03)] transition-colors cursor-pointer"
                  onClick={() => setSelected(c)}
                  onKeyDown={e => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault()
                      setSelected(c)
                    }
                  }}
                >
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-2">
                      <img className="w-4 h-4 rounded-full shrink-0 object-cover" src={logoUrl(baseSymbol(c.ticker))} alt="" onError={e => { (e.target as HTMLImageElement).style.display = 'none' }} />
                      <span className="font-semibold text-text-main">{baseSymbol(c.ticker)}</span>
                      <span className={`w-1.5 h-1.5 rounded-full ${statusDot(c.status)}`} />
                    </div>
                  </td>
                  <td className="px-3 py-2 text-left">
                    <span className={`text-[10px] font-semibold px-1.5 py-0.5 rounded capitalize ${STATUS_COLOR[c.status] ?? 'text-text-muted'}`}>{c.status}</span>
                  </td>
                  <td className="px-3 py-2 text-right font-mono text-text-main font-semibold">${fmt(c.init_fund)}</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted">${fmt(c.available_fund)}</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden sm:table-cell">${fmt(c.total_traded_fund)}</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden sm:table-cell">{c.sl_pct}%</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden lg:table-cell">{c.tp_pct}%</td>
                  <td className="px-3 py-2 text-right font-mono text-text-muted hidden lg:table-cell">{shortDate(c.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
