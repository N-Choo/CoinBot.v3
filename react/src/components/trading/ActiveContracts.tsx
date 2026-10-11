import { useState } from 'react'
import Skeleton from '../ui/Skeleton'
import { type ContractItem } from '../../services/contract'
import { useContractsData } from '../../hooks/useContractsData'

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

const fmtPnl = (pnl: number | null): string =>
  pnl == null ? '—' : `${pnl >= 0 ? '+' : '−'}$${Math.abs(pnl).toLocaleString(undefined, { maximumFractionDigits: 2 })}`

const shortDate = (iso: string): string => new Date(iso).toLocaleDateString()

const timeAgo = (iso: string): string => {
  const t = new Date(iso).getTime()
  if (Number.isNaN(t)) return ''
  const s = Math.max(1, Math.floor((Date.now() - t) / 1000))
  if (s < 60) return `${s}s ago`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

export default function ActiveContracts() {
  const { contracts, setContracts, loading, error, ordersByContract, ordersReady, pnlOf, contractPnl } = useContractsData()
  const [selected, setSelected] = useState<ContractItem | null>(null)
  const [ordersFilter, setOrdersFilter] = useState('All')

  const closeContract = () => {
    if (!selected) return
    setContracts(prev => prev.map(c => (c.id === selected.id ? { ...c, status: 'cancelled' } : c)))
    setSelected(null)
  }

  if (selected) {
    const orders = ordersByContract[selected.id] ?? []
    const filteredOrders = orders.filter(o => ordersFilter === 'All' || o.side.toLowerCase() === ordersFilter.toLowerCase())
    const boughtQty = orders.filter(o => o.side.toLowerCase() === 'buy').reduce((s, o) => s + (parseFloat(o.filled_qty) || parseFloat(o.requested_qty) || 0), 0)
    const soldQty = orders.filter(o => o.side.toLowerCase() === 'sell').reduce((s, o) => s + (parseFloat(o.filled_qty) || parseFloat(o.requested_qty) || 0), 0)
    let totalPnl = 0
    let hasPnl = false
    for (const o of orders) {
      const p = pnlOf(o, selected.ticker)
      if (p != null) { hasPnl = true; totalPnl += p }
    }

    return (
      <div className="flex flex-col bg-bg-panel border border-border-light rounded-xl overflow-hidden transition-colors hover:border-accent/40">
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

          <div className="border border-border-light rounded-lg overflow-hidden">
            <div className="flex items-center justify-between px-3 py-2 border-b border-border-light">
              <span className="text-[10px] font-semibold uppercase tracking-[0.16em] text-text-muted">Trade Orders</span>
              <div className="flex gap-1">
                {(['All', 'Buy', 'Sell'] as const).map(f => (
                  <button
                    key={f}
                    onClick={() => setOrdersFilter(f)}
                    className={`bg-transparent border rounded px-1.5 py-0.5 text-[10px] font-semibold cursor-pointer transition-colors ${ordersFilter === f ? 'border-accent text-accent' : 'border-border-light text-text-muted hover:text-text-main'}`}
                  >
                    {f}
                  </button>
                ))}
              </div>
            </div>

            {!ordersReady ? (
              <div className="p-3 space-y-2">
                {[1, 2].map(i => <div key={i} className="h-3 rounded bg-[rgba(255,255,255,0.04)]" />)}
              </div>
            ) : orders.length === 0 ? (
              <div className="px-3 py-3 text-[11px] text-text-muted text-center">No trade orders</div>
            ) : (
              <>
                <div className="flex flex-wrap gap-x-4 gap-y-1 px-3 py-2 text-[10px] text-text-muted border-b border-border-light">
                  <span>Orders <span className="text-text-main font-mono">{orders.length}</span></span>
                  <span>Bought <span className="text-text-main font-mono">{boughtQty.toLocaleString()}</span></span>
                  <span>Sold <span className="text-text-main font-mono">{soldQty.toLocaleString()}</span></span>
                  <span>PnL <span className={`font-mono ${!hasPnl ? 'text-text-muted' : totalPnl >= 0 ? 'text-up-text' : 'text-down-text'}`}>{fmtPnl(hasPnl ? totalPnl : null)}</span></span>
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-[11px] border-collapse">
                    <thead>
                      <tr className="text-text-muted border-b border-border-light">
                        <th className="text-left font-medium px-3 py-1.5 whitespace-nowrap">Time</th>
                        <th className="text-left font-medium px-3 py-1.5">Side</th>
                        <th className="text-right font-medium px-3 py-1.5">Qty</th>
                        <th className="text-right font-medium px-3 py-1.5 whitespace-nowrap">Filled Price</th>
                        <th className="text-right font-medium px-3 py-1.5">PnL</th>
                        <th className="text-right font-medium px-3 py-1.5">Fee</th>
                        <th className="text-left font-medium px-3 py-1.5">Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredOrders.map(o => {
                        const buy = o.side.toLowerCase() === 'buy'
                        const pnl = pnlOf(o, selected.ticker)
                        return (
                          <tr key={o.id} className="border-b border-border-light last:border-b-0">
                            <td className="px-3 py-1.5 text-text-muted font-mono whitespace-nowrap">{timeAgo(o.created_at)}</td>
                            <td className={`px-3 py-1.5 font-bold ${buy ? 'text-up' : 'text-down'}`}>{o.side.toUpperCase()}</td>
                            <td className="px-3 py-1.5 text-right font-mono text-text-main">{o.requested_qty || o.filled_qty || '—'}</td>
                            <td className="px-3 py-1.5 text-right font-mono text-text-muted">{o.fill_price ? `$${Number(o.fill_price).toLocaleString()}` : '—'}</td>
                            <td className={`px-3 py-1.5 text-right font-mono ${pnl == null ? 'text-text-muted' : pnl >= 0 ? 'text-up-text' : 'text-down-text'}`}>{fmtPnl(pnl)}</td>
                            <td className="px-3 py-1.5 text-right font-mono text-text-muted">{o.fee}</td>
                            <td className="px-3 py-1.5"><span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-[rgba(29,205,159,0.12)] text-up capitalize">{o.status}</span></td>
                          </tr>
                        )
                      })}
                    </tbody>
                  </table>
                </div>
              </>
            )}
          </div>

          <button
            className="w-full py-2 border border-down rounded-lg bg-transparent text-down text-[11px] font-bold tracking-wide cursor-pointer transition-colors hover:bg-down hover:text-white disabled:opacity-40 disabled:cursor-default disabled:hover:bg-transparent disabled:hover:text-down"
            onClick={closeContract}
            disabled={selected.status !== 'active'}
          >
            {selected.status === 'active' ? 'Close Contract' : `Contract ${selected.status}`}
          </button>
          {selected.status === 'active' && (
            <p className="text-[10px] text-text-muted text-center m-0">Closing releases the contract's funds back to your balance.</p>
          )}
        </div>
      </div>
    )
  }

  const availableTotal = contracts.reduce(
    (sum, c) => sum + (parseFloat(c.available_fund) || 0),
    0,
  )

  return (
    <div className="flex flex-col bg-bg-panel border border-border-light rounded-xl overflow-hidden transition-colors hover:border-accent/40">
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
                <th className="text-right font-medium px-3 py-1.5">PnL</th>
                <th className="text-right font-medium px-3 py-1.5 hidden sm:table-cell">SL%</th>
                <th className="text-right font-medium px-3 py-1.5 hidden lg:table-cell">TP%</th>
                <th className="text-right font-medium px-3 py-1.5 hidden lg:table-cell">Created</th>
              </tr>
            </thead>
            <tbody>
              {contracts.map(c => {
                const pnl = contractPnl(c)
                return (
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
                    <td className={`px-3 py-2 text-right font-mono ${pnl == null ? 'text-text-muted' : pnl >= 0 ? 'text-up-text' : 'text-down-text'}`}>{fmtPnl(pnl)}</td>
                    <td className="px-3 py-2 text-right font-mono text-text-muted hidden sm:table-cell">{c.sl_pct}%</td>
                    <td className="px-3 py-2 text-right font-mono text-text-muted hidden lg:table-cell">{c.tp_pct}%</td>
                    <td className="px-3 py-2 text-right font-mono text-text-muted hidden lg:table-cell">{shortDate(c.created_at)}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
