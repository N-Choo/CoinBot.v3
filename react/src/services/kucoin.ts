export interface TickerData {
  pair: string
  price: string
  change: string
  isUp: boolean
  volume: string
  raw: string
  high?: string
  low?: string
}

const FALLBACK: TickerData[] = [
  { pair: 'BTC/USDT', price: '64,230.50', change: '+2.40%', isUp: true, volume: '1.2M', raw: 'BTC-USDT' },
  { pair: 'ETH/USDT', price: '3,420.10', change: '+1.80%', isUp: true, volume: '800K', raw: 'ETH-USDT' },
  { pair: 'SOL/USDT', price: '142.75', change: '+4.20%', isUp: true, volume: '450K', raw: 'SOL-USDT' },
  { pair: 'XRP/USDT', price: '0.62', change: '+1.30%', isUp: true, volume: '520K', raw: 'XRP-USDT' },
  { pair: 'ADA/USDT', price: '0.48', change: '-0.80%', isUp: false, volume: '340K', raw: 'ADA-USDT' },
  { pair: 'AVAX/USDT', price: '35.40', change: '-1.10%', isUp: false, volume: '120K', raw: 'AVAX-USDT' },
  { pair: 'DOT/USDT', price: '7.25', change: '+3.50%', isUp: true, volume: '210K', raw: 'DOT-USDT' },
  { pair: 'LINK/USDT', price: '18.90', change: '+5.70%', isUp: true, volume: '300K', raw: 'LINK-USDT' },
  { pair: 'MATIC/USDT', price: '0.72', change: '+2.10%', isUp: true, volume: '190K', raw: 'MATIC-USDT' },
  { pair: 'SHIB/USDT', price: '0.000025', change: '-1.50%', isUp: false, volume: '680K', raw: 'SHIB-USDT' },
  { pair: 'TRX/USDT', price: '0.11', change: '+0.90%', isUp: true, volume: '280K', raw: 'TRX-USDT' },
  { pair: 'ATOM/USDT', price: '8.90', change: '+2.60%', isUp: true, volume: '150K', raw: 'ATOM-USDT' },
  { pair: 'LTC/USDT', price: '85.20', change: '-0.30%', isUp: false, volume: '175K', raw: 'LTC-USDT' },
  { pair: 'BCH/USDT', price: '245.00', change: '+1.70%', isUp: true, volume: '90K', raw: 'BCH-USDT' },
  { pair: 'NEAR/USDT', price: '3.85', change: '+4.80%', isUp: true, volume: '160K', raw: 'NEAR-USDT' },
  { pair: 'APT/USDT', price: '9.40', change: '-2.20%', isUp: false, volume: '130K', raw: 'APT-USDT' },
  { pair: 'ARB/USDT', price: '1.05', change: '+3.30%', isUp: true, volume: '145K', raw: 'ARB-USDT' },
  { pair: 'OP/USDT', price: '3.15', change: '-2.30%', isUp: false, volume: '180K', raw: 'OP-USDT' },
  { pair: 'SUI/USDT', price: '4.82', change: '+3.10%', isUp: true, volume: '250K', raw: 'SUI-USDT' },
  { pair: 'PEPE/USDT', price: '0.000008', change: '+7.20%', isUp: true, volume: '420K', raw: 'PEPE-USDT' },
  { pair: 'INJ/USDT', price: '24.50', change: '+5.10%', isUp: true, volume: '85K', raw: 'INJ-USDT' },
  { pair: 'TIA/USDT', price: '8.15', change: '-3.40%', isUp: false, volume: '110K', raw: 'TIA-USDT' },
  { pair: 'SEI/USDT', price: '0.55', change: '+2.80%', isUp: true, volume: '95K', raw: 'SEI-USDT' },
  { pair: 'RNDR/USDT', price: '8.90', change: '+6.30%', isUp: true, volume: '75K', raw: 'RNDR-USDT' },
  { pair: 'AAVE/USDT', price: '105.00', change: '+4.50%', isUp: true, volume: '55K', raw: 'AAVE-USDT' },
  { pair: 'UNI/USDT', price: '7.80', change: '+1.90%', isUp: true, volume: '100K', raw: 'UNI-USDT' },
  { pair: 'ALGO/USDT', price: '0.19', change: '-0.60%', isUp: false, volume: '140K', raw: 'ALGO-USDT' },
  { pair: 'FTM/USDT', price: '0.68', change: '+5.40%', isUp: true, volume: '115K', raw: 'FTM-USDT' },
  { pair: 'SAND/USDT', price: '0.42', change: '+3.70%', isUp: true, volume: '80K', raw: 'SAND-USDT' },
  { pair: 'AXS/USDT', price: '7.10', change: '-1.80%', isUp: false, volume: '65K', raw: 'AXS-USDT' },
  { pair: 'EGLD/USDT', price: '38.60', change: '+2.50%', isUp: true, volume: '45K', raw: 'EGLD-USDT' },
  { pair: 'HBAR/USDT', price: '0.08', change: '+4.10%', isUp: true, volume: '125K', raw: 'HBAR-USDT' },
  { pair: 'ICP/USDT', price: '10.25', change: '+0.80%', isUp: true, volume: '70K', raw: 'ICP-USDT' },
  { pair: 'STX/USDT', price: '1.95', change: '+6.20%', isUp: true, volume: '105K', raw: 'STX-USDT' },
  { pair: 'IMX/USDT', price: '1.55', change: '+3.90%', isUp: true, volume: '88K', raw: 'IMX-USDT' },
  { pair: 'CRO/USDT', price: '0.09', change: '+1.20%', isUp: true, volume: '60K', raw: 'CRO-USDT' },
  { pair: 'QNT/USDT', price: '82.00', change: '-0.50%', isUp: false, volume: '35K', raw: 'QNT-USDT' },
  { pair: 'RUNE/USDT', price: '5.45', change: '+4.60%', isUp: true, volume: '78K', raw: 'RUNE-USDT' },
]

function formatPrice(n: string): string {
  const v = parseFloat(n)
  if (isNaN(v)) return '0.00'
  if (v >= 1000) return v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
  return v.toFixed(v < 1 ? 6 : 2)
}

function formatChange(r: string): string {
  const v = parseFloat(r) * 100
  return `${v >= 0 ? '+' : ''}${v.toFixed(2)}%`
}

function formatVolume(v: string): string {
  const n = parseFloat(v)
  if (isNaN(n)) return '0'
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toFixed(0)
}

export async function fetchTickers(): Promise<TickerData[]> {
  try {
    const res = await fetch('/kucoin-api/api/v1/market/allTickers')
    if (!res.ok) return FALLBACK
    const json = await res.json()
    const tickers = json.data?.ticker || []
    if (!tickers.length) return FALLBACK
    return tickers
      .filter((t: { symbol: string }) => t.symbol.endsWith('-USDT'))
      .map((t: { symbol: string; last: string; changeRate: string; high: string; low: string; vol: string }) => ({
        pair: t.symbol.replace('-', '/'),
        price: formatPrice(t.last),
        change: formatChange(t.changeRate),
        isUp: parseFloat(t.changeRate) >= 0,
        volume: formatVolume(t.vol),
        high: formatPrice(t.high),
        low: formatPrice(t.low),
        raw: t.symbol,
      }))
  } catch {
    return FALLBACK
  }
}

export function searchTickers(list: TickerData[], query: string): TickerData[] {
  if (!query.trim()) return list
  const q = query.toLowerCase()
  return list.filter(t => t.pair.toLowerCase().includes(q))
}
