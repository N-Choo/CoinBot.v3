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

const feedTypeStyles: Record<string, string> = {
  BUY: 'bg-[rgba(0,212,170,0.15)] text-up',
  SELL: 'bg-[rgba(255,59,59,0.15)] text-down',
  STAKE: 'bg-[rgba(255,183,0,0.15)] text-[#ffb700]',
}

export default function ActivityFeed() {
  const items = [...MOCK_TRADES, ...MOCK_TRADES]

  return (
    <div className="px-6 py-8 max-w-[var(--max-width,1200px)] mx-auto w-full">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.5px] text-text-muted mb-4">
        <span className="w-1.5 h-1.5 rounded-full bg-accent animate-pulse" data-testid="feed-dot" />
        Live Activity
      </div>
      <div className="overflow-hidden">
        <div className="flex gap-6 w-max feed-track">
          {items.map((t, i) => (
            <span key={i} data-testid="feed-item" className="inline-flex items-center gap-2 px-4 py-2 border border-border-light rounded-full text-xs font-semibold font-mono whitespace-nowrap bg-bg-panel">
              <span className={`uppercase px-1.5 py-0.5 rounded text-[10px] ${feedTypeStyles[t.type] || 'text-text-muted'}`}>{t.type}</span>
              <span className="text-text-main">{t.pair}</span>
              <span className="text-text-muted">{t.price}</span>
              {t.pnl && <span className="text-up" data-testid="feed-pnl">{t.pnl}</span>}
            </span>
          ))}
        </div>
      </div>
    </div>
  )
}
