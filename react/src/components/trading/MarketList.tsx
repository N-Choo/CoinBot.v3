import type { TradingPair } from './types'

interface MarketListProps {
  pairs: TradingPair[]
  selectedPair: string
  onSelectPair: (pair: string) => void
}

export default function MarketList({ pairs, selectedPair, onSelectPair }: MarketListProps) {
  return (
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
        {pairs.map((coin, index) => (
          <div
            key={index}
            className={`market-item ${selectedPair === coin.pair ? 'active-pair' : ''}`}
            onClick={() => onSelectPair(coin.pair)}
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
  )
}
