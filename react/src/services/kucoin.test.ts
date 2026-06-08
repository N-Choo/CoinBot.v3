import { describe, it, expect, vi, beforeEach } from 'vitest'

beforeEach(() => {
  vi.restoreAllMocks()
  vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('No network in tests'))
})

describe('fetchTickers', () => {
  it('returns fallback data on network error', async () => {
    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(result.length).toBeGreaterThan(20)
    expect(result[0].pair).toBe('BTC/USDT')
  })

  it('returns fallback data on non-200 response', async () => {
    vi.restoreAllMocks()
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ ok: false } as Response)
    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    expect(result[0].pair).toBe('BTC/USDT')
  })

  it('includes BTC/USDT in fallback data', async () => {
    const { fetchTickers } = await import('./kucoin')
    const result = await fetchTickers()
    const btc = result.find(t => t.pair === 'BTC/USDT')
    expect(btc).toBeDefined()
    expect(btc!.price).toEqual(expect.any(String))
    expect(btc!.change).toEqual(expect.any(String))
    expect(typeof btc!.isUp).toBe('boolean')
  })
})