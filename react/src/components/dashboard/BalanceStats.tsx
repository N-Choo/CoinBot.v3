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
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4">
        {[1, 2, 3, 4].map(i => (
          <div key={i} className="bg-bg-panel border border-border-light rounded-xl p-4 backdrop-blur-xl">
            <Skeleton variant="text-sm" width="60%" />
            <div className="my-2"><Skeleton variant="text-lg" width="70%" /></div>
          </div>
        ))}
      </div>
    )
  }

  const cards = [
    { label: 'Total Balance', value: fmtUsd(stats.total), cls: 'text-text-main' },
    { label: 'Free Balance', value: fmtUsd(stats.free), cls: 'text-text-main' },
    { label: 'Locked Balance', value: fmtUsd(stats.locked), cls: 'text-text-main' },
    { label: 'Unrealized PnL', value: fmtSigned(totalPnl), cls: totalPnl >= 0 ? 'text-up-text' : 'text-down-text' },
  ]

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4">
      {cards.map(card => (
        <div
          key={card.label}
          tabIndex={0}
          className="bg-bg-panel border border-border-light rounded-xl p-4 backdrop-blur-xl transition-colors hover:border-accent/40"
        >
          <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-text-muted">{card.label}</div>
          <div className={`font-mono text-[22px] sm:text-[24px] font-bold mt-2 leading-tight truncate ${card.cls}`}>
            {card.value}
          </div>
        </div>
      ))}
    </div>
  )
}
