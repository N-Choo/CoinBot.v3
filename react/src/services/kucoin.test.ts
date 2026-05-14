import { describe, it, expect, vi, beforeEach } from 'vitest'

const kucoinMock = {
  data: {
    ticker: [
      { symbol: 'BTC-USDT', last: '64230.5', changeRate: '0.024', vol: '1234567' },
      { symbol: 'ETH-USDT', last: '3420.1', changeRate: '-0.018', vol: '890123' },
      { symbol: 'SOL-USDT', last: '142.75', changeRate: '0.042', vol: '456789' },
    ],
  },
}

beforeEach(() => {
  vi.restoreAllMocks()
})

describe('fetchTickers', () => {
  it('fetches and formats tickers from KuCoin API', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(kucoinMock),
    } as Response)

    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()

    expect(fetch).toHaveBeenCalledWith('https://api.kucoin.com/api/v1/market/allTickers')
    expect(result).toHaveLength(3)
    expect(result[0]).toMatchObject({
      pair: 'BTC/USDT',
      price: '64,230.50',
      change: '+2.40%',
      isUp: true,
      volume: '1.23M',
    })
  })

  it('filters USDT pairs only', async () => {
    const mixed = {
      data: {
        ticker: [
          { symbol: 'BTC-USDT', last: '64230', changeRate: '0.01', vol: '100' },
          { symbol: 'ETH-BTC', last: '0.05', changeRate: '0.01', vol: '100' },
        ],
      },
    }
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mixed),
    } as Response)

    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(result).toHaveLength(1)
    expect(result[0].pair).toBe('BTC/USDT')
  })

  it('returns fallback data on API error', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'))

    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(result.length).toBeGreaterThanOrEqual(6)
    expect(result[0].pair).toBe('BTC/USDT')
  })

  it('fallback has at least 30 tickers when API fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'))

    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(result.length).toBeGreaterThanOrEqual(30)
  })
})

describe('searchTickers', () => {
  beforeEach(async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(kucoinMock),
    } as Response)
  })

  it('filters tickers by search query', async () => {
    const { fetchTickers, searchTickers } = await import('./kucoin')
    const all = await fetchTickers()
    const results = searchTickers(all, 'btc')
    expect(results).toHaveLength(1)
    expect(results[0].pair).toBe('BTC/USDT')
  })

  it('returns all when query is empty', async () => {
    const { fetchTickers, searchTickers } = await import('./kucoin')
    const all = await fetchTickers()
    const results = searchTickers(all, '')
    expect(results).toHaveLength(3)
  })

  it('is case insensitive', async () => {
    const { fetchTickers, searchTickers } = await import('./kucoin')
    const all = await fetchTickers()
    expect(searchTickers(all, 'BTC')).toHaveLength(1)
    expect(searchTickers(all, 'btc')).toHaveLength(1)
    expect(searchTickers(all, 'Sol')).toHaveLength(1)
  })
})
