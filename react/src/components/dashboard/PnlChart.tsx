import { useState, useMemo } from 'react'
import Skeleton from '../ui/Skeleton'
import { useContractsData } from '../../hooks/useContractsData'

const RANGES = [
  { label: '7D', days: 7 },
  { label: '14D', days: 14 },
  { label: '1M', days: 30 },
]

function buildPath(data: { value: number }[]) {
  if (data.length < 2) return null
  const vals = data.map(d => d.value)
  const min = Math.min(...vals)
  const max = Math.max(...vals)
  const range = max - min || 1
  const padding = range * 0.1
  const adjMin = min - padding
  const adjMax = max + padding
  const adjRange = adjMax - adjMin
  const H = 80

  const pts = data.map((d, i) => ({
    x: (i / (data.length - 1)) * 100,
    y: 90 - ((d.value - adjMin) / adjRange) * H,
  }))

  const line = pts.map((p, i) => {
    if (i === 0) return `M${p.x.toFixed(1)},${p.y.toFixed(1)}`
    return `L${p.x.toFixed(1)},${p.y.toFixed(1)}`
  }).join(' ')

  const area = `${line} L100,90 L0,90 Z`
  return { line, area, pts, min, max }
}

const money = (n: number) => `${n >= 0 ? '+' : '−'}$${Math.abs(n).toLocaleString(undefined, { maximumFractionDigits: 2 })}`

export default function PnlChart() {
  const { contracts, ordersByContract, loading, prices, pnlOf } = useContractsData()
  const [range, setRange] = useState('14D')

  // Filled/open orders across all contracts, time-ordered, each with its live PnL.
  const filled = useMemo(() => {
    const list: { t: number; pnl: number }[] = []
    for (const c of contracts) {
      for (const o of (ordersByContract[c.id] ?? [])) {
        const pnl = pnlOf(o, c.ticker)
        if (pnl != null) list.push({ t: new Date(o.created_at).getTime(), pnl })
      }
    }
    return list.sort((a, b) => a.t - b.t)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [contracts, ordersByContract, prices])

  // Cumulative PnL within the selected window (baseline 0 at the window start).
  const series = useMemo(() => {
    const days = RANGES.find(r => r.label === range)!.days
    // eslint-disable-next-line react-hooks/purity -- the window tracks the current time
    const cutoff = Date.now() - days * 86_400_000
    let acc = 0
    const out: { date: string; value: number }[] = []
    for (const f of filled) {
      if (f.t < cutoff) continue
      acc += f.pnl
      out.push({ date: '', value: acc })
    }
    return out
  }, [filled, range])

  const sumSince = (ms: number) => filled.filter(f => f.t >= Date.now() - ms).reduce((s, f) => s + f.pnl, 0)

  const path = useMemo(() => buildPath(series), [series])

  if (loading) {
    return (
      <div className="p-3 sm:p-5">
        <Skeleton variant="text-sm" width="100px" />
        <Skeleton variant="text-lg" width="160px" />
        <Skeleton variant="chart" />
      </div>
    )
  }

  if (!path) {
    return (
      <div className="p-3 sm:p-5">
        <div className="text-[10px] text-text-muted font-semibold uppercase tracking-[0.3px]">PnL Performance</div>
        <div className="h-[120px] sm:h-[180px] flex items-center justify-center text-xs text-text-muted">
          Not enough orders yet
        </div>
      </div>
    )
  }

  const last = series[series.length - 1].value
  const first = series[0].value
  const pct = first !== 0 ? (((last - first) / Math.abs(first)) * 100).toFixed(1) : '—'
  const up = last >= first

  const blocks: [string, number, string][] = [
    ['Daily PnL', sumSince(86_400_000), 'vs yesterday'],
    ['Weekly PnL', sumSince(7 * 86_400_000), 'vs last week'],
    ['Monthly PnL', sumSince(30 * 86_400_000), 'vs last month'],
  ]

  return (
    <div className="p-3 sm:p-5">
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-2 mb-1">
        <div>
          <div className="text-[10px] text-text-muted font-semibold uppercase tracking-[0.3px]">PnL Performance</div>
          <div className="text-xl sm:text-[26px] font-extrabold font-mono leading-tight transition-colors duration-300"
            style={{ color: up ? 'var(--color-up)' : 'var(--color-down)' }}>
            {money(last)}
          </div>
        </div>
        <div className="flex sm:text-right items-center sm:items-end gap-2 sm:flex-col">
          {pct !== '—' && (
            <span className="text-[11px] font-bold font-mono px-2 py-0.5 rounded"
              style={{ background: up ? 'var(--color-up-bg)' : 'var(--color-down-bg)', color: up ? 'var(--color-up-text)' : 'var(--color-down-text)' }}>
              {up ? '\u25B2' : '\u25BC'} {Math.abs(Number(pct))}%
            </span>
          )}
          <div className="flex gap-0.5 border border-border-light rounded p-0.5">
            {RANGES.map(r => (
              <button
                key={r.label}
                className={`bg-transparent border-none text-text-muted text-[10px] font-semibold px-2 py-0.5 rounded cursor-pointer transition-all ${range === r.label ? 'bg-accent text-black' : ''}`}
                onClick={() => setRange(r.label)}
              >
                {r.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="relative w-full my-2 cursor-default overflow-visible h-[100px] sm:h-[180px]">
        <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="block w-full h-full overflow-visible">
          <defs>
            <linearGradient id="chartFill" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={up ? 'var(--color-up)' : 'var(--color-down)'} stopOpacity="0.15" />
              <stop offset="100%" stopColor={up ? 'var(--color-up)' : 'var(--color-down)'} stopOpacity="0" />
            </linearGradient>
          </defs>
          <path d={path.area} fill="url(#chartFill)" />
          <path d={path.line} fill="none" stroke={up ? 'var(--color-up)' : 'var(--color-down)'} strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
        </svg>
      </div>

      <div className="grid grid-cols-3 gap-3 pt-3 mt-1 border-t border-border-light">
        {blocks.map(([label, value, caption]) => (
          <div key={label}>
            <div className="text-[10px] text-text-muted font-medium mb-0.5">{label}</div>
            <div className={`text-xs sm:text-[13px] font-bold font-mono ${value >= 0 ? 'text-up-text' : 'text-down-text'}`}>{money(value)}</div>
            <div className="text-[10px] text-text-muted font-medium">{caption}</div>
          </div>
        ))}
      </div>
    </div>
  )
}
