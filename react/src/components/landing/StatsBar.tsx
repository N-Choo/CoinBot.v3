import { useState, useEffect, useRef } from 'react'

const STATS = [
  { label: 'Trading Volume', value: '$450M+' },
  { label: 'Uptime', value: '99.9%' },
  { label: 'Active Users', value: '10K+' },
  { label: 'Liquidation Risk', value: '0.0%' },
]

function AnimatedStat({ value, label }: { value: string; label: string }) {
  const ref = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const obs = new IntersectionObserver(
      ([entry]) => { if (entry.isIntersecting) { setVisible(true); obs.disconnect() } },
      { threshold: 0.3 }
    )
    obs.observe(el)
    return () => obs.disconnect()
  }, [])

  const num = parseFloat(value.replace(/[^0-9.]/g, '')) || 0
  const suffix = value.replace(/[0-9.]/g, '')
  const [display, setDisplay] = useState(0)

  useEffect(() => {
    if (!visible) return
    const start = performance.now()
    const dur = 1200
    let raf: number
    function tick(now: number) {
      const t = Math.min((now - start) / dur, 1)
      setDisplay(Math.round(t * num))
      if (t < 1) raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [visible, num])

  return (
    <div className="stat-item" ref={ref}>
      <span className="stat-value" aria-live="polite">{visible ? display : '0'}{suffix}</span>
      <span className="stat-label">{label}</span>
    </div>
  )
}

export default function StatsBar() {
  return (
    <div className="stats-bar">
      {STATS.map(s => <AnimatedStat key={s.label} value={s.value} label={s.label} />)}
    </div>
  )
}
