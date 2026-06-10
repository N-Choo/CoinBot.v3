export interface BotSettings {
  Ticker: string
  Amount: string
  TakeProfit: string
  StopLoss: string
}

export interface TradingPair {
  pair: string
  price: string
  change: string
  isUp: boolean
  volume: string
  high?: string
  low?: string
}
