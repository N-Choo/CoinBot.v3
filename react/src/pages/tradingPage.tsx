import { useState } from 'react'
import '../styles/tradingPage.css'
import TradingChart from '../components/trading/TradingChart'
import TradingHeader from '../components/trading/TradingHeader'
import TradingSettings from '../components/trading/TradingSettings'
import MarketList from '../components/trading/MarketList'
import type { BotSettings } from '../components/trading/types'

const tradingPairs = [
  { pair: 'BTC/USDT', price: '64,230.50', change: '+2.4%', isUp: true, volume: '1.2B' },
  { pair: 'ETH/USDT', price: '3,420.10', change: '+1.8%', isUp: true, volume: '800M' },
  { pair: 'SOL/USDT', price: '142.75', change: '-4.2%', isUp: false, volume: '450M' },
  { pair: 'AVAX/USDT', price: '35.40', change: '-1.1%', isUp: false, volume: '120M' },
  { pair: 'LINK/USDT', price: '18.90', change: '+5.7%', isUp: true, volume: '300M' },
  { pair: 'DOGE/USDT', price: '0.145', change: '+0.5%', isUp: true, volume: '95M' },
]

const Trading = () => {
  const [selectedPair, setSelectedPair] = useState('BTC/USDT')

  const [botSettings, setBotSettings] = useState<BotSettings>({
    Amount: '100',
    Expires: '5',
    TakeProfit: '50.0',
    StopLoss: '35.0'
  })

  const handleSettingChange = (key: keyof BotSettings, value: string) => {
    setBotSettings((prev) => ({
      ...prev,
      [key]: value
    }))
  }

  const handleStartTrading = () => {
    const payload = {
      pair: selectedPair,
      parameters: botSettings
    }

    console.log('Payload:', payload)
    alert(`Starting bot on ${payload.pair}\n\nParameters:\n${JSON.stringify(payload.parameters, null, 2)}`)
  }

  return (
    <div className="trading-page-wrapper">
      <div className="trading-grid">

        <div className="main-column">

          <div className="chart-container" style={{ minHeight: '450px' }}>
            <TradingChart symbol={selectedPair} />
          </div>

          <div className="panel top-controls-panel">
            <TradingHeader selectedPair={selectedPair} onStartTrading={handleStartTrading} />
            <TradingSettings botSettings={botSettings} onSettingChange={handleSettingChange} />
          </div>
        </div>

        <div className="side-column">
          <MarketList pairs={tradingPairs} selectedPair={selectedPair} onSelectPair={setSelectedPair} />
        </div>

      </div>
    </div>
  )
}

export default Trading
