import { useState, useEffect } from 'react'
import { fetchTickers } from '../../services/kucoin'
import type { TickerData } from '../../services/kucoin'

const WATCHED = ['BTC-USDT', 'ETH-USDT', 'SOL-USDT']

export default function LiveTicker() {
  const [tickers, setTickers] = useState<TickerData[]>([])

  useEffect(() => {
    let active = true

    const poll = async () => {
      try {
        const all = await fetchTickers()
        if (active) setTickers(all.filter(t => WATCHED.includes(t.raw)))
      } catch (e) {
        console.error('Failed to fetch tickers:', e)
      }
      if (active) setTimeout(poll, 10_000)
    }

    poll()
    return () => { active = false }
  }, [])

  return (
    <div className="border border-border-light rounded-xl p-2 bg-bg-panel backdrop-blur-xl">
      {tickers.map(t => (
        <div key={t.raw} className={`grid grid-cols-[100px_1fr_80px] items-center gap-3 px-4 py-3.5 rounded-lg transition-colors hover:bg-[rgba(255,255,255,0.03)] ${t.isUp ? 'text-up' : 'text-down'}`}>
          <span className="text-[14px] font-bold font-mono text-text-main">{t.pair}</span>
          <span className="text-[16px] font-bold font-mono text-right">${t.price}</span>
          <span className={`text-[13px] font-semibold font-mono text-right px-2 py-0.5 rounded ${t.isUp ? 'bg-[rgba(0,212,170,0.1)]' : 'bg-[rgba(255,59,59,0.1)]'}`}>{t.change}</span>
        </div>
      ))}
    </div>
  )
}
