import { render, screen } from '@testing-library/react'
import { describe, test, expect, vi, beforeAll } from 'vitest'
import { BrowserRouter } from 'react-router-dom'
import { AuthProvider } from '../hooks/useAuth'
import CoinBot from './homePage'

vi.mock('../services/stats', () => ({
  getStats: vi.fn().mockResolvedValue({ accounts: 0, active_contracts: 0, funds_deployed: '0', trades_executed: 0 }),
}))

function Wrapper({ children }: { children: React.ReactNode }) {
  return <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><AuthProvider>{children}</AuthProvider></BrowserRouter>
}

beforeAll(() => {
  const origRAF = globalThis.requestAnimationFrame
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => origRAF(() => cb(performance.now() + 9999)))

  class MockObserver {
    private cb: IntersectionObserverCallback
    constructor(cb: IntersectionObserverCallback) {
      this.cb = cb
    }
    observe() { setTimeout(() => this.cb([{ isIntersecting: true }] as IntersectionObserverEntry[], null!), 0) }
    unobserve = vi.fn()
    disconnect = vi.fn()
  }
  vi.stubGlobal('IntersectionObserver', MockObserver as unknown as typeof IntersectionObserver)
})

describe('CoinBot (homePage)', () => {
  test('renders hero section with title', () => {
    render(<Wrapper><CoinBot /></Wrapper>)
    expect(screen.getByText('Your wealth.')).toBeInTheDocument()
    expect(screen.getByText('Automated.')).toBeInTheDocument()
  })

  test('renders stats section', () => {
    render(<Wrapper><CoinBot /></Wrapper>)
    expect(screen.getByText('Accounts')).toBeInTheDocument()
    expect(screen.getByText('Active Contracts')).toBeInTheDocument()
    expect(screen.getByText('Funds Deployed')).toBeInTheDocument()
    expect(screen.getByText('Trades Executed')).toBeInTheDocument()
  })

  test('renders feature cards', () => {
    render(<Wrapper><CoinBot /></Wrapper>)
    expect(screen.getByText('Wallet Accounts')).toBeInTheDocument()
    expect(screen.getByText('Trading Service')).toBeInTheDocument()
    expect(screen.getByText('Automated Deposits')).toBeInTheDocument()
  })

  test('renders CTA buttons', () => {
    render(<Wrapper><CoinBot /></Wrapper>)
    expect(screen.getByText('Start Trading')).toBeInTheDocument()
    expect(screen.getByText('GitHub')).toBeInTheDocument()
  })

  test('renders footer', () => {
    render(<Wrapper><CoinBot /></Wrapper>)
    expect(screen.getByText('CoinBot')).toBeInTheDocument()
  })
})
