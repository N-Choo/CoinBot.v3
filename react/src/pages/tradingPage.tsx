import { useState, useEffect, useRef, memo } from 'react'
import '../styles/tradingPage.css'

interface ChartProps {
  symbol: string
}

const TradingViewChart = memo<ChartProps>(({ symbol }) => {
  const container = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!container.current) return
    container.current.innerHTML = ''

    const script = document.createElement('script')
    script.src = 'https://s3.tradingview.com/external-embedding/embed-widget-advanced-chart.js'
    script.type = 'text/javascript'
    script.async = true

    const formattedSymbol = `BINANCE:${symbol.replace('/', '')}`

    script.innerHTML = JSON.stringify({
      "autosize": true,
      "symbol": formattedSymbol,
      "interval": "60",
      "timezone": "Etc/UTC",
      "theme": "dark",
      "style": "1",
      "locale": "en",
      "enable_publishing": false,
      "hide_top_toolbar": false,
      "hide_legend": false,
      "save_image": false,
      "container_id": "tradingview_chart_container"
    })

    container.current.appendChild(script)
  }, [symbol])

  return (
    <div className="tradingview-widget-container" ref={container} style={{ height: '100%', width: '100%' }}>
      <div id="tradingview_chart_container" style={{ height: '100%', width: '100%' }} />
    </div>
  )
})

TradingViewChart.displayName = 'TradingViewChart'

interface BotSettings {
  Amount: string
  Expires: string
  TakeProfit: string
  StopLoss: string
}

const Trading = () => {
  const [selectedPair, setSelectedPair] = useState('BTC/USDT')

  const [botSettings, setBotSettings] = useState<BotSettings>({
    Amount: '100',
    Expires: '5',
    TakeProfit: '50.0',
    StopLoss: '35.0'
  })

  const tradingPairs = [
    { pair: 'BTC/USDT', price: '64,230.50', change: '+2.4%', isUp: true, volume: '1.2B' },
    { pair: 'ETH/USDT', price: '3,420.10', change: '+1.8%', isUp: true, volume: '800M' },
    { pair: 'SOL/USDT', price: '142.75', change: '-4.2%', isUp: false, volume: '450M' },
    { pair: 'AVAX/USDT', price: '35.40', change: '-1.1%', isUp: false, volume: '120M' },
    { pair: 'LINK/USDT', price: '18.90', change: '+5.7%', isUp: true, volume: '300M' },
    { pair: 'DOGE/USDT', price: '0.145', change: '+0.5%', isUp: true, volume: '95M' },
  ]

  const TradeSettings = [
    { key: 'Amount' as const, label: 'Trade Amount', placeholder: '100', suffix: 'USDT' },
    { key: 'Expires' as const, label: 'Max Trades/Day', placeholder: '5' },
    { key: 'TakeProfit' as const, label: 'Take Profit', placeholder: '50.0', suffix: '%' },
    { key: 'StopLoss' as const, label: 'Stop Loss', placeholder: '35.0', suffix: '%' },
  ]

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
            <TradingViewChart symbol={selectedPair} />
          </div>

          <div className="panel top-controls-panel">
            <div className="chart-header-row">
              <div className="pair-info">
                <div className="pair-title-group">
                  <h1 className="pair-title">{selectedPair}</h1>
                  <span className="current-price">64,230.50</span>
                </div>

                <div className="pair-stats">
                  <div className="stat-block">
                    <span className="stat-label">24h Change</span>
                    <span className="stat-value price-up">+2.40%</span>
                  </div>
                  <div className="stat-divider"></div>
                  <div className="stat-block">
                    <span className="stat-label">24h Volume</span>
                    <span className="stat-value">1.2B</span>
                  </div>
                </div>
              </div>

              <button className="start-bot-btn desktop-only" onClick={handleStartTrading}>
                <span>START TRADING</span>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="5" y1="12" x2="19" y2="12"></line>
                  <polyline points="12 5 19 12 12 19"></polyline>
                </svg>
              </button>
            </div>

            <div className="bot-settings-integrated">
              <div className="settings-title">
                <div className="status-dot"></div>
                <h3>Trading Parameters</h3>
              </div>
              <div className="settings-grid">
                {TradeSettings.map((setting) => (
                  <div key={setting.key} className="modern-input-group">
                    <label>{setting.label}</label>
                    <div className="input-wrapper">
                      <input
                        type="text"
                        placeholder={setting.placeholder}
                        value={botSettings[setting.key]}
                        onChange={(e) => handleSettingChange(setting.key, e.target.value)}
                      />
                      {setting.suffix && <span className="currency-badge">{setting.suffix}</span>}
                    </div>
                  </div>
                ))}
              </div>
            </div>

          </div>
        </div>

        <div className="side-column">
          <div className="panel market-panel">

            <div className="market-header">
              <div className="modern-search">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8"></circle>
                  <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                </svg>
                <input type="text" placeholder="Search markets..." />
              </div>
            </div>

            <div className="market-list-header">
              <span>Asset</span>
              <span className="text-right">Price</span>
              <span className="text-right">24h</span>
            </div>

            <div className="market-list">
              {tradingPairs.map((coin, index) => (
                <div
                  key={index}
                  className={`market-item ${selectedPair === coin.pair ? 'active-pair' : ''}`}
                  onClick={() => setSelectedPair(coin.pair)}
                >
                  <div className="coin-info">
                    <span className="coin-name">{coin.pair.split('/')[0]}</span>
                    <span className="coin-base">/{coin.pair.split('/')[1]}</span>
                  </div>
                  <div className="coin-price text-right">{coin.price}</div>
                  <div className={`coin-change text-right ${coin.isUp ? 'price-up-bg' : 'price-down-bg'}`}>
                    {coin.change}
                  </div>
                </div>
              ))}
            </div>

          </div>
        </div>

      </div>
    </div>
  )
}

export default Trading
