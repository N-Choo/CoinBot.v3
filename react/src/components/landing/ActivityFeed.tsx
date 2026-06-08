const MOCK_TRADES = [
  { type: 'BUY', pair: 'BTC/USDT', price: '$68,210', pnl: '+$312.50' },
  { type: 'SELL', pair: 'ETH/USDT', price: '$3,490', pnl: '+$89.20' },
  { type: 'STAKE', pair: 'ETH → Lido', price: '3.2% APY', pnl: '' },
  { type: 'BUY', pair: 'SOL/USDT', price: '$140.20', pnl: '+$2.60' },
  { type: 'SELL', pair: 'AVAX/USDT', price: '$35.40', pnl: '+$14.80' },
  { type: 'BUY', pair: 'LINK/USDT', price: '$18.90', pnl: '+$5.70' },
  { type: 'STAKE', pair: 'USDC → Aave', price: '5.1% APY', pnl: '' },
  { type: 'SELL', pair: 'DOGE/USDT', price: '$0.12', pnl: '+$1.50' },
  { type: 'BUY', pair: 'OP/USDT', price: '$3.15', pnl: '+$8.40' },
  { type: 'SELL', pair: 'MATIC/USDT', price: '$0.72', pnl: '+$3.20' },
]

export default function ActivityFeed() {
  const items = [...MOCK_TRADES, ...MOCK_TRADES]

  return (
    <div className="activity-feed">
      <div className="feed-header">
        <span className="feed-dot" />
        Live Activity
      </div>
      <div className="feed-scroll">
        <div className="feed-track">
          {items.map((t, i) => (
            <span key={i} className={`feed-item feed-${t.type.toLowerCase()}`}>
              <span className="feed-type">{t.type}</span>
              <span className="feed-pair">{t.pair}</span>
              <span className="feed-price">{t.price}</span>
              {t.pnl && <span className="feed-pnl">{t.pnl}</span>}
            </span>
          ))}
        </div>
      </div>
    </div>
  )
}
