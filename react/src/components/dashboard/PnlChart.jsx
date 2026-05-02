import React, { useState, useRef, useEffect, useMemo } from 'react';

const ALL_DATA = [
  { date: 'Apr 04', value: 1200 }, { date: 'Apr 07', value: 980 },
  { date: 'Apr 09', value: 2400 }, { date: 'Apr 11', value: 1750 },
  { date: 'Apr 13', value: 3100 }, { date: 'Apr 15', value: 2600 },
  { date: 'Apr 18', value: 2100 }, { date: 'Apr 20', value: 3500 },
  { date: 'Apr 22', value: 1800 }, { date: 'Apr 24', value: 6200 },
  { date: 'Apr 26', value: 4900 }, { date: 'Apr 28', value: 9400 },
  { date: 'Apr 30', value: 8700 }, { date: 'May 02', value: 12400 },
];

const RANGES = [
  { label: '7D', count: 7 },
  { label: '14D', count: 14 },
  { label: '1M', count: ALL_DATA.length },
];

function buildPoints(data) {
  const vals = data.map(d => d.value);
  const minVal = Math.min(...vals);
  const maxVal = Math.max(...vals);
  const range = maxVal - minVal || 1;
  const PAD_TOP = 10, PAD_BTM = 10;
  const H = 100 - PAD_TOP - PAD_BTM;

  return data.map((d, i) => ({
    x: (i / (data.length - 1)) * 100,
    y: PAD_TOP + H - ((d.value - minVal) / range) * H,
    ...d,
  }));
}

function smoothPath(pts) {
  return pts.map((p, i) => {
    if (i === 0) return `M ${p.x.toFixed(2)},${p.y.toFixed(2)}`;
    const prev = pts[i - 1];
    const cx = (prev.x + p.x) / 2;
    return `C ${cx.toFixed(2)},${prev.y.toFixed(2)} ${cx.toFixed(2)},${p.y.toFixed(2)} ${p.x.toFixed(2)},${p.y.toFixed(2)}`;
  }).join(' ');
}

export default function PnlChart() {
  const [range, setRange] = useState('14D');
  const [hoverData, setHoverData] = useState(null);
  const [drawn, setDrawn] = useState(false);
  const svgRef = useRef(null);

  const data = useMemo(() => {
    const r = RANGES.find(r => r.label === range);
    return ALL_DATA.slice(-r.count);
  }, [range]);

  const points = useMemo(() => buildPoints(data), [data]);
  const linePath = useMemo(() => smoothPath(points), [points]);
  const areaPath = `${linePath} L 100,100 L 0,100 Z`;

  const lastVal = data[data.length - 1].value;
  const firstVal = data[0].value;
  const pctChange = (((lastVal - firstVal) / firstVal) * 100).toFixed(1);
  const isPositive = lastVal >= firstVal;

  useEffect(() => {
    setDrawn(false);
    setHoverData(null);
    const t = setTimeout(() => setDrawn(true), 60);
    return () => clearTimeout(t);
  }, [range]);

  const handleMouseMove = (e) => {
    if (!svgRef.current) return;
    const rect = svgRef.current.getBoundingClientRect();
    const xPct = ((e.clientX - rect.left) / rect.width) * 100;
    const closest = points.reduce((prev, curr) =>
      Math.abs(curr.x - xPct) < Math.abs(prev.x - xPct) ? curr : prev
    );
    setHoverData(closest);
  };

  const display = hoverData ?? { value: lastVal, date: data[data.length - 1].date };
  const lineColor = isPositive ? 'var(--color-primary)' : '#ff4e4e';

  return (
    <div className="dash-panel pnl-panel" onMouseLeave={() => setHoverData(null)}>
      <div className="pnl-header">
        <div className="pnl-header-left">
          <span className="pnl-eyebrow">PnL Performance</span>
          <div className="pnl-value" style={{ color: lineColor }}>
            ${display.value.toLocaleString()}
          </div>
          <span className="pnl-date-label">{display.date}</span>
        </div>

        <div className="pnl-header-right">
          <span className="pnl-badge" style={{ background: isPositive ? 'var(--bg-success)' : 'rgba(255,60,60,0.12)', color: lineColor }}>
            {isPositive ? '▲' : '▼'} {Math.abs(pctChange)}%
          </span>
          <div className="pnl-range-tabs">
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

      <div className="chart-wrapper" style={{ height: '150px' }}>
        <svg
          ref={svgRef}
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          onMouseMove={handleMouseMove}
        >
          <defs>
            <linearGradient id="pnlGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={lineColor} stopOpacity="0.4" />
              <stop offset="100%" stopColor={lineColor} stopOpacity="0" />
            </linearGradient>
            <clipPath id="pnlClip">
              <rect x="0" y="0" height="100" width={drawn ? '100' : '0'} style={{ transition: 'width 1s ease' }} />
            </clipPath>
          </defs>

          {/* Area fill */}
          <path d={areaPath} fill="url(#pnlGrad)" opacity="0.15" clipPath="url(#pnlClip)" />

          {/* Simple Main Line — No extra scanner lines or circles rendered here */}
          <path
            d={linePath}
            fill="none"
            stroke={lineColor}
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            clipPath="url(#pnlClip)"
          />
        </svg>
      </div>

      {/* Historic Grid remains for data utility */}
      <div className="historic-stats-grid">
        <div className="stat-block">
          <span className="stat-label">Daily PnL</span>
          <span className="stat-value price-up">+$420.50</span>
        </div>
        <div className="stat-block">
          <span className="stat-label">Weekly PnL</span>
          <span className="stat-value price-up">+$2,840.10</span>
        </div>
        <div className="stat-block">
          <span className="stat-label">Monthly PnL</span>
          <span className="stat-value price-up">+$11,200.00</span>
        </div>
      </div>
    </div>
  );
}
