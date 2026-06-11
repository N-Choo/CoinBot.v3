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
    <div className="flex flex-col items-center gap-1 shrink-0" ref={ref}>
      <span className="text-[24px] sm:text-[18px] lg:text-[24px] font-extrabold font-mono text-text-main" aria-live="polite">{visible ? display : '0'}{suffix}</span>
      <span className="text-[11px] sm:text-[9px] lg:text-[11px] text-text-muted uppercase tracking-[0.5px] font-medium">{label}</span>
    </div>
  )
}

export default function StatsBar() {
  return (
    <div className="flex justify-center gap-8 sm:gap-4 lg:gap-12 px-6 py-8 border-y border-border-light bg-bg-panel overflow-x-auto">
      {STATS.map(s => <AnimatedStat key={s.label} value={s.value} label={s.label} />)}
    </div>
  )
}
