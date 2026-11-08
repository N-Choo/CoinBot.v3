import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'
import { getUserStats, type UserStats } from '../../services/stats'
import { useContractsData } from '../../hooks/useContractsData'

const fmtUsd = (v: string): string => {
  const n = parseFloat(v)
  return '$' + (Number.isNaN(n) ? 0 : n).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const fmtSigned = (n: number): string =>
  `${n >= 0 ? '+' : '−'}$${Math.abs(n).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`

// 2×2 on small screens, 4-up from lg. Dividers move with the layout.
const cellBorder = (i: number): string => [
  'lg:border-l-0',
  'border-l lg:border-l',
  'border-t lg:border-t-0 lg:border-l',
  'border-t border-l lg:border-t-0 lg:border-l',
][i]

export default function BalanceStats() {
  const { totalPnl } = useContractsData()
  const [stats, setStats] = useState<UserStats | null>(null)

  useEffect(() => {
    let active = true
    const load = () => {
      getUserStats()
        .then(d => { if (active) setStats(d) })
        .catch(() => {})
    }
    load()
    window.addEventListener('contracts:changed', load)
    return () => { active = false; window.removeEventListener('contracts:changed', load) }
  }, [])

  if (!stats) {
    return (
      <div className="grid grid-cols-2 lg:grid-cols-4 border border-border-light rounded-xl overflow-hidden bg-bg-panel backdrop-blur-xl">
        {[0, 1, 2, 3].map(i => (
          <div key={i} className={`min-w-0 p-3 sm:p-4 border-border-light ${cellBorder(i)}`}>
            <Skeleton variant="text-sm" width="60%" />
            <div className="my-2"><Skeleton variant="text-lg" width="70%" /></div>
            <Skeleton variant="text-sm" width="45%" />
          </div>
        ))}
      </div>
    )
  }

  const cards = [
    { label: 'Total Balance', value: fmtUsd(stats.total), sub: 'free + locked', cls: 'text-text-main' },
    { label: 'Free Balance', value: fmtUsd(stats.free), sub: 'available now', cls: 'text-text-main' },
    { label: 'Locked Balance', value: fmtUsd(stats.locked), sub: 'in contracts', cls: 'text-text-main' },
    { label: 'Unrealized PnL', value: fmtSigned(totalPnl), sub: 'live mark', cls: totalPnl >= 0 ? 'text-up-text' : 'text-down-text' },
  ]

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 border border-border-light rounded-xl overflow-hidden bg-bg-panel backdrop-blur-xl">
      {cards.map((card, i) => (
        <div
          key={card.label}
          tabIndex={0}
          className={`min-w-0 p-3 sm:p-4 border-border-light transition-colors hover:bg-[rgba(255,255,255,0.02)] ${cellBorder(i)}`}
        >
          <div className="text-[9px] sm:text-[10px] font-semibold uppercase tracking-[0.14em] text-text-muted truncate">{card.label}</div>
          <div className={`mt-1.5 font-mono text-[15px] sm:text-[17px] font-bold leading-tight truncate ${card.cls}`}>
            {card.value}
          </div>
          <div className="mt-0.5 text-[9px] text-text-muted truncate">{card.sub}</div>
        </div>
      ))}
    </div>
  )
}
