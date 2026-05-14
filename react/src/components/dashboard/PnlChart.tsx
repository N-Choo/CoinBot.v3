import { useState, useMemo } from 'react'

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
  const H = 100 - 20
  const pts = data.map((d, i) => ({
    x: (i / (data.length - 1)) * 100,
    y: 10 + H - ((d.value - min) / range) * H,
  }))
  const line = pts.map((p, i) => {
    if (i === 0) return `M${p.x.toFixed(1)},${p.y.toFixed(1)}`
    const prev = pts[i - 1]
    const cx = (prev.x + p.x) / 2
    return `C${cx.toFixed(1)},${prev.y.toFixed(1)} ${cx.toFixed(1)},${p.y.toFixed(1)} ${p.x.toFixed(1)},${p.y.toFixed(1)}`
  }).join(' ')
  const area = `${line} L100,100 L0,100 Z`
  return { line, area, pts }
}

export default function PnlChart() {
  const [range, setRange] = useState('14D')

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
          <div className="pnl-value" style={{ color }}>${last.toLocaleString()}</div>
        </div>
        <div className="pnl-right-col">
          <div className="pnl-badge" style={{ color }}>
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
          <defs>
            <linearGradient id="pg" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity="0.3" />
              <stop offset="100%" stopColor={color} stopOpacity="0" />
            </linearGradient>
          </defs>
          <path d={path.area} fill="url(#pg)" />
          <path d={path.line} fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" />
        </svg>
      </div>

      <div className="historic-stats-grid">
        <div><div className="stat-label">Daily PnL</div><div className="stat-value price-up">+$420</div></div>
        <div><div className="stat-label">Weekly PnL</div><div className="stat-value price-up">+$2,840</div></div>
        <div><div className="stat-label">Monthly PnL</div><div className="stat-value price-up">+$11,200</div></div>
      </div>
    </div>
  )
}
