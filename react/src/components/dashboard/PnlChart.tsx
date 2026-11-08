import { useState, useEffect, useMemo, useRef, useId, type PointerEvent as ReactPointerEvent } from 'react'
import Skeleton from '../ui/Skeleton'
import { useContractsData } from '../../hooks/useContractsData'

const RANGES = [
  { label: '7D', days: 7 },
  { label: '14D', days: 14 },
  { label: '1M', days: 30 },
] as const

type RangeLabel = (typeof RANGES)[number]['label']

const DAY = 86_400_000

interface TradePoint { t: number; pnl: number }
interface SeriesPoint { t: number; cum: number; pnl: number | null }
interface HoverPoint extends SeriesPoint { open?: boolean }

const nf2 = new Intl.NumberFormat('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })

const money = (n: number, digits = 2): string => {
  const sign = n < 0 ? '\u2212' : '+'
  const abs = Math.abs(n)
  const body = digits === 2
    ? nf2.format(abs)
    : abs.toLocaleString('en-US', { maximumFractionDigits: digits })
  return `${sign}$${body}`
}

const axisMoney = (n: number): string => {
  const abs = Math.abs(n)
  const body = abs >= 1000 ? `${(abs / 1000).toFixed(1)}k` : abs.toFixed(0)
  return `${n < 0 ? '\u2212' : ''}$${body}`
}

const fmtDate = (t: number): string =>
  new Date(t).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })

function niceTicks(min: number, max: number, count: number): number[] {
  const span = max - min || 1
  const raw = span / count
  const mag = Math.pow(10, Math.floor(Math.log10(raw)))
  const norm = raw / mag
  const step = (norm < 1.5 ? 1 : norm < 3 ? 2 : norm < 7 ? 5 : 10) * mag
  const ticks: number[] = []
  for (let v = Math.ceil(min / step) * step; v <= max + 1e-9; v += step) {
    ticks.push(Number(v.toFixed(10)))
  }
  return ticks
}

function stepPath(pts: SeriesPoint[], x: (t: number) => number, y: (v: number) => number): string {
  let d = `M${x(pts[0].t).toFixed(2)},${y(pts[0].cum).toFixed(2)}`
  for (let i = 1; i < pts.length; i++) {
    d += ` H${x(pts[i].t).toFixed(2)} V${y(pts[i].cum).toFixed(2)}`
  }
  return d
}

export default function PnlChart() {
  const { contracts, ordersByContract, loading, ordersReady, prices, pnlOf } = useContractsData()
  const [range, setRange] = useState<RangeLabel>('14D')
  const [box, setBox] = useState({ w: 0, h: 0 })
  const [hover, setHover] = useState<{ x: number; y: number; point: HoverPoint } | null>(null)
  const wrapRef = useRef<HTMLDivElement>(null)
  const gradientId = useId()

  useEffect(() => {
    const el = wrapRef.current
    if (!el || typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(entries => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect
        setBox({ w: Math.round(width), h: Math.round(height) })
      }
    })
    ro.observe(el)
    setBox({ w: el.clientWidth, h: el.clientHeight })
    return () => ro.disconnect()
  }, [])

  // Closed round trips: filled close orders carry the realized PnL.
  const closed = useMemo(() => {
    const list: TradePoint[] = []
    for (const c of contracts) {
      for (const o of ordersByContract[c.id] ?? []) {
        if (o.action !== 'close' || o.status.toLowerCase() !== 'filled') continue
        const pnl = parseFloat(o.pnl ?? '')
        const t = new Date(o.updated_at || o.created_at).getTime()
        if (!Number.isFinite(pnl) || !Number.isFinite(t)) continue
        list.push({ t, pnl })
      }
    }
    return list.sort((a, b) => a.t - b.t)
  }, [contracts, ordersByContract])

  // Open positions: entry orders with no filled close against them, marked live.
  const unrealized = useMemo(() => {
    let sum = 0
    for (const c of contracts) {
      const orders = ordersByContract[c.id] ?? []
      const closedEntries = new Set(
        orders
          .filter(o => o.action === 'close' && o.status.toLowerCase() === 'filled' && o.parent_order_id)
          .map(o => o.parent_order_id as string),
      )
      for (const o of orders) {
        if (o.action === 'close' || closedEntries.has(o.id)) continue
        const pnl = pnlOf(o, c.ticker)
        if (pnl != null) sum += pnl
      }
    }
    return sum
    // eslint-disable-next-line react-hooks/exhaustive-deps -- pnlOf depends on prices
  }, [contracts, ordersByContract, prices])

  const days = RANGES.find(r => r.label === range)!.days

  const data = useMemo(() => {
    // eslint-disable-next-line react-hooks/purity -- the window tracks the current time
    const now = Date.now()
    const cutoff = now - days * DAY
    const inWindow = closed.filter(c => c.t >= cutoff)
    const points: SeriesPoint[] = [{ t: cutoff, cum: 0, pnl: null }]
    let cum = 0
    for (const trade of inWindow) {
      cum += trade.pnl
      points.push({ t: trade.t, cum, pnl: trade.pnl })
    }
    return {
      now,
      cutoff,
      points,
      realized: cum,
      count: inWindow.length,
      wins: inWindow.filter(t => t.pnl >= 0).length,
      equity: cum + unrealized,
    }
  }, [closed, days, unrealized])

  if (loading || !ordersReady) {
    return (
      <div className="p-3 sm:p-5">
        <Skeleton variant="text-sm" width="100px" />
        <Skeleton variant="text-lg" width="160px" />
        <Skeleton variant="chart" />
      </div>
    )
  }

  const { now, cutoff, points, realized, count, wins, equity } = data
  const up = equity >= 0
  const color = up ? 'var(--color-up)' : 'var(--color-down)'
  const winRate = count ? Math.round((wins / count) * 100) : 0

  const W = box.w
  const H = box.h
  const m = { t: 16, r: 14, b: 26, l: 52 }
  const plotW = W - m.l - m.r
  const plotH = H - m.t - m.b

  const vals = points.map(p => p.cum).concat([0, equity])
  let dmin = Math.min(...vals)
  let dmax = Math.max(...vals)
  const pad = (dmax - dmin || 1) * 0.18
  dmin -= pad
  dmax += pad

  const x = (t: number) => m.l + ((t - cutoff) / (now - cutoff)) * plotW
  const y = (v: number) => m.t + (1 - (v - dmin) / (dmax - dmin)) * plotH
  const yZero = Math.min(Math.max(y(0), m.t), m.t + plotH)

  const lineD = points.length > 1 ? stepPath(points, x, y) : ''
  const areaD = lineD
    ? `${lineD} H${x(now).toFixed(2)} V${yZero.toFixed(2)} H${x(cutoff).toFixed(2)} Z`
    : ''
  const last = points[points.length - 1]
  const candidates: HoverPoint[] = [...points, { t: now, cum: equity, pnl: null, open: true }]
  const xFractions = W < 520 ? [0, 0.5, 1] : [0, 0.25, 0.5, 0.75, 1]

  const onMove = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (!W || plotW <= 0 || !wrapRef.current) return
    const svgRect = e.currentTarget.getBoundingClientRect()
    const px = e.clientX - svgRect.left
    const t = cutoff + ((px - m.l) / plotW) * (now - cutoff)

    let best = candidates[0]
    let bestDist = Infinity
    for (const p of candidates) {
      const dist = Math.abs(p.t - t)
      if (dist < bestDist) { bestDist = dist; best = p }
    }

    const wrapRect = wrapRef.current.getBoundingClientRect()
    setHover({ x: e.clientX - wrapRect.left, y: e.clientY - wrapRect.top, point: best })
  }

  return (
    <div className="p-3 sm:p-5">
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3">
        <div>
          <div className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-text-muted">
            PnL Performance
            <span className="h-1 w-1 rounded-full bg-accent" aria-hidden="true" />
            <span>{range}</span>
          </div>
          <div className={`mt-2 font-mono text-[30px] sm:text-[38px] font-bold leading-none tracking-tight ${up ? 'text-up-text' : 'text-down-text'}`}>
            {money(equity)}
          </div>
          <div className="mt-2 font-mono text-[10px] sm:text-[11px] text-text-muted">
            realized <span className={realized >= 0 ? 'text-up-text' : 'text-down-text'}>{money(realized)}</span>
            {' \u00b7 '}unrealized <span className="text-accent">{money(unrealized)}</span>
          </div>
        </div>

        <div className="flex flex-col items-start sm:items-end gap-2">
          <span className={`inline-flex items-baseline gap-1.5 rounded-full border px-2.5 py-1 font-mono text-[12px] font-semibold ${realized >= 0 ? 'border-up/30 bg-[var(--color-up-bg)] text-up-text' : 'border-down/30 bg-[var(--color-down-bg)] text-down-text'}`}>
            {money(realized)}
            <span className="font-sans text-[9px] font-medium uppercase tracking-[0.06em] text-text-muted">{range} realized</span>
          </span>
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

      <div className="flex flex-wrap gap-4 mt-4 text-[9px] font-medium uppercase tracking-[0.08em] text-text-muted">
        <span className="inline-flex items-center gap-1.5">
          <i className="w-[18px] border-t-2" style={{ borderColor: 'var(--color-up)' }} />
          Realized (closed)
        </span>
        <span className="inline-flex items-center gap-1.5">
          <i className="w-[18px] border-t-2 border-dashed" style={{ borderColor: 'var(--color-accent)' }} />
          Unrealized (live)
        </span>
        <span className="inline-flex items-center gap-1.5">
          <i className="w-[18px] border-t" style={{ borderColor: 'var(--color-border-strong)' }} />
          Break-even
        </span>
      </div>

      <div ref={wrapRef} className="relative w-full h-[215px] sm:h-[250px] mt-1">
        {W > 0 && H > 0 && (
          <svg
            width={W}
            height={H}
            viewBox={`0 0 ${W} ${H}`}
            className="block overflow-visible"
            role="img"
            aria-label="Cumulative realized PnL over time"
            onPointerMove={onMove}
            onPointerLeave={() => setHover(null)}
          >
            <defs>
              <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" style={{ stopColor: color, stopOpacity: 0.18 }} />
                <stop offset="100%" style={{ stopColor: color, stopOpacity: 0 }} />
              </linearGradient>
            </defs>

            {niceTicks(dmin, dmax, 4).map(v => (
              <g key={`y-${v}`}>
                <line x1={m.l} x2={W - m.r} y1={y(v)} y2={y(v)} style={{ stroke: 'var(--color-border-light)' }} strokeWidth={1} />
                <text
                  x={m.l - 10}
                  y={y(v) + 3}
                  textAnchor="end"
                  style={{ fill: 'var(--color-text-muted)' }}
                  className="font-mono text-[9.5px]"
                >
                  {axisMoney(v)}
                </text>
              </g>
            ))}

            {xFractions.map(f => {
              const t = cutoff + (now - cutoff) * f
              return (
                <text
                  key={`x-${f}`}
                  x={x(t)}
                  y={H - 7}
                  textAnchor={f === 0 ? 'start' : f === 1 ? 'end' : 'middle'}
                  style={{ fill: 'var(--color-text-muted)' }}
                  className="font-mono text-[9.5px]"
                >
                  {fmtDate(t)}
                </text>
              )
            })}

            <line x1={m.l} x2={W - m.r} y1={yZero} y2={yZero} style={{ stroke: 'var(--color-border-strong)' }} strokeWidth={1} />

            {lineD ? (
              <>
                <path d={areaD} fill={`url(#${gradientId})`} />
                <path
                  d={`M${x(last.t).toFixed(2)},${y(last.cum).toFixed(2)} L${x(now).toFixed(2)},${y(equity).toFixed(2)}`}
                  fill="none"
                  stroke="var(--color-accent)"
                  strokeWidth={2}
                  strokeDasharray="5 5"
                  strokeLinecap="round"
                />
                <circle cx={x(now)} cy={y(equity)} r={4} style={{ fill: 'var(--color-bg-panel)', stroke: 'var(--color-accent)' }} strokeWidth={2} />
                <path d={lineD} fill="none" style={{ stroke: color }} strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round" />
                <circle cx={x(points[0].t)} cy={y(points[0].cum)} r={3} style={{ fill: 'var(--color-bg-panel)', stroke: 'var(--color-border-strong)' }} strokeWidth={1.5} />
                {points.slice(1).map((p, i) => (
                  <circle key={`${p.t}-${i}`} cx={x(p.t)} cy={y(p.cum)} r={2.6} style={{ fill: color }} />
                ))}
              </>
            ) : (
              <text
                x={W / 2}
                y={H / 2}
                textAnchor="middle"
                style={{ fill: 'var(--color-text-muted)' }}
                className="font-mono text-[10px]"
              >
                Not enough closed trades in this window
              </text>
            )}

            {hover && (
              <g>
                <line
                  x1={x(hover.point.t)}
                  x2={x(hover.point.t)}
                  y1={m.t}
                  y2={m.t + plotH}
                  style={{ stroke: 'var(--color-border-strong)' }}
                  strokeWidth={1}
                />
                <circle
                  cx={x(hover.point.t)}
                  cy={y(hover.point.cum)}
                  r={4.5}
                  style={{ fill: 'var(--color-bg-panel)', stroke: color }}
                  strokeWidth={2}
                />
              </g>
            )}
          </svg>
        )}

        {hover && (
          <div
            className="pointer-events-none absolute z-10 min-w-[130px] rounded-lg border border-border-light bg-bg-panel/95 px-2.5 py-2 text-[10px] leading-relaxed shadow-xl backdrop-blur"
            style={{
              left: Math.min(Math.max(hover.x + 14, 6), Math.max(W - 150, 6)),
              top: Math.min(Math.max(hover.y - 82, 6), Math.max(H - 84, 6)),
            }}
          >
            <div className="font-mono text-[9px] uppercase tracking-[0.06em] text-text-muted">
              {hover.point.open ? 'Now' : fmtDate(hover.point.t)}
            </div>
            <div className="flex justify-between gap-3 font-mono">
              <span className="text-text-muted">Realized</span>
              <span className={hover.point.cum >= 0 ? 'text-up-text' : 'text-down-text'}>{money(hover.point.cum)}</span>
            </div>
            {hover.point.pnl != null && (
              <div className="flex justify-between gap-3 font-mono">
                <span className="text-text-muted">Trade</span>
                <span className={hover.point.pnl >= 0 ? 'text-up-text' : 'text-down-text'}>{money(hover.point.pnl)}</span>
              </div>
            )}
            {hover.point.open && (
              <div className="flex justify-between gap-3 font-mono">
                <span className="text-text-muted">Unrealized</span>
                <span className="text-accent">{money(unrealized)}</span>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="grid grid-cols-3 gap-2 pt-3 mt-3 border-t border-border-light">
        <div>
          <div className="text-[9px] font-semibold uppercase tracking-[0.14em] text-text-muted">Realized</div>
          <div className={`mt-1 font-mono text-[15px] font-semibold ${realized >= 0 ? 'text-up-text' : 'text-down-text'}`}>{money(realized)}</div>
          <div className="mt-0.5 text-[9px] text-text-muted">in {range} window</div>
        </div>
        <div>
          <div className="text-[9px] font-semibold uppercase tracking-[0.14em] text-text-muted">Unrealized</div>
          <div className="mt-1 font-mono text-[15px] font-semibold text-accent">{money(unrealized)}</div>
          <div className="mt-0.5 text-[9px] text-text-muted">live mark · open orders</div>
        </div>
        <div>
          <div className="text-[9px] font-semibold uppercase tracking-[0.14em] text-text-muted">Closed trades</div>
          <div className="mt-1 font-mono text-[15px] font-semibold">{count}</div>
          <div className="mt-0.5 text-[9px] text-text-muted">{winRate}% win rate</div>
        </div>
      </div>

      <p className="mt-3 pl-2.5 border-l-2 border-accent/40 text-[9.5px] leading-relaxed text-text-muted">
        <b className="font-semibold text-text-main">Realized</b> is the cumulative sum of closed round-trip PnL, seeded at $0 at the window start and drawn as a step line.
        {' '}<b className="font-semibold text-text-main">Unrealized</b> is the live mark of open orders, shown as a separate dashed tail and never mixed into historical points.
      </p>
    </div>
  )
}
