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
    <div className="live-ticker">
      {tickers.map(t => (
        <div key={t.raw} className={`ticker-row ${t.isUp ? 'up' : 'down'}`}>
          <span className="ticker-pair">{t.pair}</span>
          <span className="ticker-price">${t.price}</span>
          <span className="ticker-change">{t.change}</span>
        </div>
      ))}
    </div>
  )
}
