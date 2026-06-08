import { describe, it, expect } from 'vitest'

describe('fetchTickers', () => {
  it('returns fallback ticker data', async () => {
    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(Array.isArray(result)).toBe(true)
    expect(result.length).toBeGreaterThan(20)
    expect(result[0]).toMatchObject({
      pair: expect.any(String),
      price: expect.any(String),
      change: expect.any(String),
      isUp: expect.any(Boolean),
      volume: expect.any(String),
    })
  })

  it('includes BTC/USDT in the data', async () => {
    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    const btc = result.find(t => t.pair === 'BTC/USDT')
    expect(btc).toBeDefined()
    expect(btc!.price).toEqual(expect.any(String))
    expect(btc!.change).toEqual(expect.any(String))
    expect(typeof btc!.isUp).toBe('boolean')
  })
})
