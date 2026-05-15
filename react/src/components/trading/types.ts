export interface BotSettings {
  Amount: string
  Expires: string
  TakeProfit: string
  StopLoss: string
}

export interface TradingPair {
  pair: string
  price: string
  change: string
  isUp: boolean
  volume: string
}
