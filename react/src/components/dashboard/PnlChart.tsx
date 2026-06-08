import { useState, useMemo, useEffect } from 'react'

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
  const [range, setRange] = useState('14D')
  const [ticker, setTicker] = useState(0)
  const [flash, setFlash] = useState('')

  useEffect(() => {
    const interval = setInterval(() => {
      const change = (Math.random() * 40 - 20)
      setTicker(prev => +(prev + change).toFixed(2))
      setFlash(change >= 0 ? 'flash-up' : 'flash-down')
      setTimeout(() => setFlash(''), 400)
    }, 3000)
    return () => clearInterval(interval)
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
  const color = up ? 'var(--color-primary)' : 'var(--color-danger)'

  return (
    <div className="dash-panel">
      <div className="pnl-header">
        <div>
          <div className="dash-title-sm">PnL Performance</div>
          <div className="pnl-value" style={{ color }}>
            <span className={`pnl-ticker ${flash}`}>
              ${last.toLocaleString()}
              {ticker !== 0 && (
                <span className="ticker-arrow">
                  {ticker > 0 ? `+${ticker}` : ticker}
                </span>
              )}
            </span>
          </div>
        </div>
        <div className="pnl-right-col">
          <div className="pnl-badge" style={{ background: up ? 'var(--color-up-bg)' : 'var(--color-down-bg)', color: up ? 'var(--color-up-text)' : 'var(--color-down-text)' }}>
            {up ? '\u25B2' : '\u25BC'} {Math.abs(Number(pct))}%
          </div>
          <div className="pnl-range-tabs pnl-range-wrap">
            {RANGES.map(r => (
              <button
                key={r.label}
                className={`pnl-tab ${range === r.label ? 'pnl-tab-active' : ''}`}
                onClick={() => setRange(r.label)}
              >
                {r.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="chart-wrapper pnl-chart-area">
        <svg viewBox="0 0 100 100" preserveAspectRatio="none">
          <path d={path.line} fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke" />
        </svg>
      </div>

      <div className="historic-stats-grid">
        <div>
          <div className="stat-label">Daily PnL</div>
          <div className="stat-value price-up">+$420</div>
          <div className="data-label">vs yesterday</div>
        </div>
        <div>
          <div className="stat-label">Weekly PnL</div>
          <div className="stat-value price-up">+$2,840</div>
          <div className="data-label">vs last week</div>
        </div>
        <div>
          <div className="stat-label">Monthly PnL</div>
          <div className="stat-value price-up">+$11,200</div>
          <div className="data-label">vs last month</div>
        </div>
      </div>
    </div>
  )
}
