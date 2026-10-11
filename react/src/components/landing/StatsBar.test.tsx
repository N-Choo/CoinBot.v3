import { render, screen } from '@testing-library/react'
import { beforeAll, describe, expect, test, vi } from 'vitest'

import StatsBar from './StatsBar'
import { getStats } from '../../services/stats'

vi.mock('../../services/stats', () => ({ getStats: vi.fn() }))
const mockGet = vi.mocked(getStats)

beforeAll(() => {
  const origRAF = globalThis.requestAnimationFrame
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => origRAF(() => cb(performance.now() + 9999)))

  class MockObserver {
    private cb: IntersectionObserverCallback
    constructor(cb: IntersectionObserverCallback) { this.cb = cb }
    observe() { setTimeout(() => this.cb([{ isIntersecting: true }] as IntersectionObserverEntry[], null!), 0) }
    unobserve = vi.fn()
    disconnect = vi.fn()
  }
  vi.stubGlobal('IntersectionObserver', MockObserver as unknown as typeof IntersectionObserver)
})

describe('StatsBar', () => {
  test('renders live system metrics', async () => {
    mockGet.mockResolvedValue({ accounts: 3, active_contracts: 2, funds_deployed: '8000', trades_executed: 12 })
    render(<StatsBar />)
    expect(screen.getByText('Accounts')).toBeInTheDocument()
    expect(screen.getByText('Active Contracts')).toBeInTheDocument()
    expect(screen.getByText('Trades Executed')).toBeInTheDocument()
    expect(await screen.findByText('$8,000')).toBeInTheDocument()
  })
})
