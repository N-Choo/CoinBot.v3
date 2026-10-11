import { useState, useEffect, useRef } from 'react'
import { getStats, type SystemStats } from '../../services/stats'

interface Stat {
  label: string
  value: number
  format: (n: number) => string
}

const fmtInt = (n: number) => Math.round(n).toLocaleString()
const fmtUsd = (n: number) => '$' + Math.round(n).toLocaleString()

function AnimatedStat({ label, value, format }: Stat) {
  const ref = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)
  const [display, setDisplay] = useState(0)
  const displayRef = useRef(0)

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

  useEffect(() => {
    if (!visible) return
    const from = displayRef.current
    const to = value
    const start = performance.now()
    const dur = 900
    let raf: number
    function tick(now: number) {
      const t = Math.min((now - start) / dur, 1)
      const next = from + (to - from) * t
      displayRef.current = next
      setDisplay(next)
      if (t < 1) raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [visible, value])

  return (
    <div className="flex flex-col items-center gap-1 shrink-0" ref={ref}>
      <span className="text-[24px] sm:text-[18px] lg:text-[24px] font-extrabold font-mono text-text-main" aria-live="polite">
        {format(display)}
      </span>
      <span className="text-[11px] sm:text-[9px] lg:text-[11px] text-text-muted uppercase tracking-[0.5px] font-medium">{label}</span>
    </div>
  )
}

const EMPTY: SystemStats = { accounts: 0, active_contracts: 0, funds_deployed: '0', trades_executed: 0 }

export default function StatsBar() {
  const [stats, setStats] = useState<SystemStats>(EMPTY)

  useEffect(() => {
    let active = true
    const load = () => {
      getStats()
        .then(d => { if (active) setStats(d) })
        .catch(() => {})
    }
    load()
    const id = setInterval(load, 15_000)
    return () => {
      active = false
      clearInterval(id)
    }
  }, [])

  const items: Stat[] = [
    { label: 'Accounts', value: stats.accounts, format: fmtInt },
    { label: 'Active Contracts', value: stats.active_contracts, format: fmtInt },
    { label: 'Funds Deployed', value: parseFloat(stats.funds_deployed) || 0, format: fmtUsd },
    { label: 'Trades Executed', value: stats.trades_executed, format: fmtInt },
  ]

  return (
    <div className="flex justify-center gap-8 sm:gap-4 lg:gap-12 px-6 py-8 border-y border-border-light bg-bg-panel overflow-x-auto">
      {items.map(s => <AnimatedStat key={s.label} {...s} />)}
    </div>
  )
}
