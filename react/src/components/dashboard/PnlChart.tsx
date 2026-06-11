import { useState, useMemo, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

const ALL_DATA = [
  { date: 'Apr 04', value: 1200 }, { date: 'Apr 07', value: 980 },
  { date: 'Apr 09', value: 2400 }, { date: 'Apr 11', value: 1750 },
  { date: 'Apr 13', value: 3100 }, { date: 'Apr 15', value: 2600 },
  { date: 'Apr 18', value: 2100 }, { date: 'Apr 20', value: 3500 },
  { date: 'Apr 22', value: 1800 }, { date: 'Apr 24', value: 6200 },
  { date: 'Apr 26', value: 4900 }, { date: 'Apr 28', value: 9400 },
  { date: 'Apr 30', value: 8700 }, { date: 'May 02', value: 12400 },
]

const RANGES = [
  { label: '7D', count: 7 },
  { label: '14D', count: 14 },
  { label: '1M', count: ALL_DATA.length },
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

export default function PnlChart() {
  const [loading, setLoading] = useState(true)
  const [range, setRange] = useState('14D')
  const [ticker, setTicker] = useState(0)
  const [flash, setFlash] = useState('')

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 800)
    return () => clearTimeout(t)
  }, [])

  useEffect(() => {
    const timeouts: ReturnType<typeof setTimeout>[] = []
    const interval = setInterval(() => {
      const change = (Math.random() * 40 - 20)
      setTicker(prev => +(prev + change).toFixed(2))
      setFlash(change >= 0 ? 'flash-up' : 'flash-down')
      timeouts.push(setTimeout(() => setFlash(''), 400))
    }, 3000)
    return () => {
      clearInterval(interval)
      timeouts.forEach(clearTimeout)
    }
  }, [])

  const data = useMemo(() => {
    const r = RANGES.find(r => r.label === range)
    return ALL_DATA.slice(-r!.count)
  }, [range])

  const path = useMemo(() => buildPath(data), [data])
  if (!path) return null

  const last = data[data.length - 1].value
  const first = data[0].value
  const pct = (((last - first) / first) * 100).toFixed(1)
  const up = last >= first

  if (loading) {
    return (
      <div className="p-3 sm:p-5">
        <Skeleton variant="text-sm" width="100px" />
        <Skeleton variant="text-lg" width="160px" />
        <Skeleton variant="chart" />
      </div>
    )
  }

  return (
    <div className="p-3 sm:p-5">
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-2 mb-1">
        <div>
          <div className="text-[10px] text-text-muted font-semibold uppercase tracking-[0.3px]">PnL Performance</div>
          <div className="text-xl sm:text-[26px] font-extrabold font-mono leading-tight"
            style={{ color: up ? 'var(--color-up)' : 'var(--color-down)' }}>
            <span className={`inline-flex items-center gap-1 transition-colors duration-300 ${flash}`}>
              ${last.toLocaleString()}
              {ticker !== 0 && (
                <span className="inline-block text-xs" style={{ animation: 'ticker-pop 0.3s ease' }}>
                  {ticker > 0 ? `+${ticker}` : ticker}
                </span>
              )}
            </span>
          </div>
        </div>
        <div className="flex sm:text-right items-center sm:items-end gap-2 sm:flex-col">
          <span className="text-[11px] font-bold font-mono px-2 py-0.5 rounded"
            style={{ background: up ? 'var(--color-up-bg)' : 'var(--color-down-bg)', color: up ? 'var(--color-up-text)' : 'var(--color-down-text)' }}>
            {up ? '\u25B2' : '\u25BC'} {Math.abs(Number(pct))}%
          </span>
          <div className="flex gap-0.5 border border-border-light rounded p-0.5">
            {RANGES.map(r => (
              <button
                key={r.label}
                className={`bg-transparent border-none text-text-muted text-[10px] font-semibold px-2 py-0.5 rounded cursor-pointer transition-all ${range === r.label ? 'bg-neon-teal text-black' : ''}`}
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
        <div>
          <div className="text-[10px] text-text-muted font-medium mb-0.5">Daily PnL</div>
          <div className="text-xs sm:text-[13px] font-bold font-mono text-up-text">+$420</div>
          <div className="text-[10px] text-text-muted font-medium">vs yesterday</div>
        </div>
        <div>
          <div className="text-[10px] text-text-muted font-medium mb-0.5">Weekly PnL</div>
          <div className="text-xs sm:text-[13px] font-bold font-mono text-up-text">+$2,840</div>
          <div className="text-[10px] text-text-muted font-medium">vs last week</div>
        </div>
        <div>
          <div className="text-[10px] text-text-muted font-medium mb-0.5">Monthly PnL</div>
          <div className="text-xs sm:text-[13px] font-bold font-mono text-up-text">+$11,200</div>
          <div className="text-[10px] text-text-muted font-medium">vs last month</div>
        </div>
      </div>
    </div>
  )
}
