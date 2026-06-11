import { useState, useEffect } from 'react'
import TradingChart from '../components/trading/TradingChart'
import TradingForm from '../components/trading/TradingForm'
import MarketTickerBar from '../components/trading/MarketTickerBar'
import ActiveContracts from '../components/trading/ActiveContracts'
import type { BotSettings, TradingPair } from '../components/trading/types'
import { fetchTickers } from '../services/kucoin'

const FALLBACK_PAIRS: TradingPair[] = [
  { pair: 'BTC/USDT', price: '64,230.50', change: '+2.4%', isUp: true, volume: '1.2B', high: '65,120', low: '62,300' },
  { pair: 'ETH/USDT', price: '3,420.10', change: '+1.8%', isUp: true, volume: '800M', high: '3,480', low: '3,350' },
  { pair: 'SOL/USDT', price: '142.75', change: '-4.2%', isUp: false, volume: '450M', high: '149', low: '138' },
  { pair: 'AVAX/USDT', price: '35.40', change: '-1.1%', isUp: false, volume: '120M', high: '36.8', low: '34.2' },
  { pair: 'LINK/USDT', price: '18.90', change: '+5.7%', isUp: true, volume: '300M', high: '19.5', low: '17.8' },
  { pair: 'DOGE/USDT', price: '0.145', change: '+0.5%', isUp: true, volume: '95M', high: '0.148', low: '0.139' },
]

const Trading = () => {
  const [tradingPairs, setTradingPairs] = useState<TradingPair[]>(FALLBACK_PAIRS)
  const [selectedPair, setSelectedPair] = useState('BTC/USDT')

  useEffect(() => {
    let active = true
    const fetch = () => fetchTickers().then(data => { if (active) setTradingPairs(data) })
    fetch()
    const interval = setInterval(fetch, 30000)
    return () => { active = false; clearInterval(interval) }
  }, [])

  const [editable, setEditable] = useState({
    Amount: '100',
    TakeProfit: '50.0',
    StopLoss: '35.0'
  })

  const botSettings: BotSettings = { Ticker: selectedPair, ...editable }

  const handleSettingChange = (key: keyof BotSettings, value: string) => {
    if (key === 'Ticker') return
    setEditable((prev) => ({ ...prev, [key]: value }))
  }

  return (
    <div className="min-h-screen pt-14 flex bg-bg-dark">
      <div className="flex flex-col w-full">
        <MarketTickerBar pairs={tradingPairs} selectedPair={selectedPair} onSelectPair={setSelectedPair} />
        <div className="flex flex-col lg:flex-row flex-1 min-h-0 border-t border-border-light">
          <div className="flex-1 flex flex-col min-w-0 lg:border-r border-border-light">
            <div className="flex-1 min-h-[400px] lg:min-h-[500px]">
              <TradingChart symbol={selectedPair} />
            </div>
            <ActiveContracts />
          </div>
          <div className="lg:w-[320px] shrink-0 border-t lg:border-t-0 border-border-light">
            <TradingForm selectedPair={selectedPair} botSettings={botSettings} onSettingChange={handleSettingChange} />
          </div>
        </div>
      </div>
    </div>
  )
}

export default Trading
