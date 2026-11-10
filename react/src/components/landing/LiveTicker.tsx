import { useEffect, useId, useState } from 'react'
import { fetchTickers, type TickerData } from '../../services/kucoin'

const WATCHED = ['BTC-USDT', 'ETH-USDT', 'SOL-USDT']

const FALLBACK: TickerData[] = [
  { pair: 'BTC/USDT', price: '74,464.10', change: '-0.69%', isUp: false, volume: '1.2M', raw: 'BTC-USDT', high: '75,120.00', low: '73,980.00' },
  { pair: 'ETH/USDT', price: '2,078.56', change: '-2.40%', isUp: false, volume: '800K', raw: 'ETH-USDT', high: '2,140.10', low: '2,061.40' },
  { pair: 'SOL/USDT', price: '84.60', change: '-3.17%', isUp: false, volume: '450K', raw: 'SOL-USDT', high: '88.20', low: '83.90' },
]

interface Row {
  raw: string
  pair: string
  price: string
  change: string
  isUp: boolean
  volume: string
  high: string
  low: string
  spark: number[]
}

function syntheticSpark(change: string, seed: number): number[] {
  const trend = Number(change.replace('%', '')) / 100
  return Array.from({ length: 24 }, (_, i) => {
    const base = 1 + trend * (i / 23)
    const wobble = Math.sin((i + seed) * 0.9) * 0.004 + Math.sin((i + seed) * 0.31) * 0.002
    return base + wobble
  })
}

function Sparkline({ values, up, width = 108, height = 34 }: { values: number[]; up: boolean; width?: number; height?: number }) {
  const id = `spark-${useId().replace(/:/g, '')}`
  if (values.length < 2) return <div style={{ width, height }} />

  const min = Math.min(...values)
  const max = Math.max(...values)
  const range = max - min || 1
  const points = values.map((value, index) => {
    const x = (index / (values.length - 1)) * width
    const y = height - ((value - min) / range) * height
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  const line = points.join(' ')
  const color = up ? 'var(--color-up)' : 'var(--color-down)'

  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} aria-hidden="true" className="overflow-visible">
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.28" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <polygon points={`0,${height} ${line} ${width},${height}`} fill={`url(#${id})`} />
      <polyline points={line} fill="none" stroke={color} strokeWidth="1.6" strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  )
}

function useMarketRows() {
  const [rows, setRows] = useState<Row[]>(() => FALLBACK.map((t, i) => ({
    ...t,
    high: t.high ?? '—',
    low: t.low ?? '—',
    spark: syntheticSpark(t.change, i + 1),
  })))

  useEffect(() => {
    let active = true

    const load = async () => {
      try {
        const tickers = await fetchTickers()
        const picked = WATCHED.map(raw => tickers.find(t => t.raw === raw)).filter((t): t is TickerData => Boolean(t))
        if (!picked.length) return

        const sparks = await Promise.all(picked.map(async ticker => {
          try {
            const res = await fetch(`/kucoin-api/api/v1/market/candles?type=1hour&symbol=${ticker.raw}`)
            if (!res.ok) return []
            const json = await res.json()
            const data: string[][] = json?.data ?? []
            return data
              .slice()
              .reverse()
              .map(candle => parseFloat(candle[2]))
              .filter(Number.isFinite)
          } catch {
            return []
          }
        }))

        if (!active) return
        setRows(picked.map((ticker, index) => ({
          ...ticker,
          high: ticker.high ?? '—',
          low: ticker.low ?? '—',
          spark: sparks[index].length > 1 ? sparks[index] : syntheticSpark(ticker.change, index + 1),
        })))
      } catch {
        // Keep the fallback numbers.
      }
    }

    load()
    const interval = window.setInterval(load, 30_000)
    return () => {
      active = false
      window.clearInterval(interval)
    }
  }, [])

  return rows
}

function ChangeBadge({ row }: { row: Row }) {
  return (
    <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[11px] font-semibold font-mono ${
      row.isUp ? 'text-up bg-[var(--color-up-bg)]' : 'text-down bg-[var(--color-down-bg)]'
    }`}>
      {row.change}
    </span>
  )
}

export default function LiveTicker() {
  const rows = useMarketRows()
  const [featured, ...rest] = rows

  if (!featured) return null

  return (
    <div className="rounded-xl border border-border-light bg-bg-panel p-5 backdrop-blur-xl">
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
          <span className="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-muted">Market Pulse</span>
        </div>
        <span className="text-[10px] font-mono uppercase tracking-[0.14em] text-text-muted">KuCoin</span>
      </div>

      <div className="rounded-lg border border-border-light bg-[var(--bg-input)] px-4 py-3">
        <div className="flex items-start justify-between">
          <div>
            <div className="text-[14px] font-bold font-mono text-text-main">{featured.pair}</div>
            <div className="mt-0.5 text-[24px] font-extrabold font-mono tracking-[-0.5px] text-text-main">${featured.price}</div>
          </div>
          <ChangeBadge row={featured} />
        </div>
        <div className="mt-2 flex items-center justify-between gap-3">
          <Sparkline values={featured.spark} up={featured.isUp} width={170} height={52} />
          <div className="text-right text-[9px] font-mono text-text-muted">
            <div>H {featured.high}</div>
            <div>L {featured.low}</div>
            <div>V {featured.volume}</div>
          </div>
        </div>
      </div>

      <div className="mt-3 flex flex-col">
        {rest.map(row => (
          <div key={row.raw} className="grid grid-cols-[1fr_72px_auto] items-center gap-3 border-b border-border-light py-2.5 last:border-0 last:pb-0">
            <div>
              <div className="text-[12px] font-semibold font-mono text-text-main">{row.pair}</div>
              <div className="text-[10px] font-mono text-text-muted">${row.price}</div>
            </div>
            <Sparkline values={row.spark} up={row.isUp} width={72} height={24} />
            <div className="text-right">
              <ChangeBadge row={row} />
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
