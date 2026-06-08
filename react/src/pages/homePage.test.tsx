import { render, screen } from '@testing-library/react'
import { describe, test, expect, vi, beforeAll } from 'vitest'
import { BrowserRouter } from 'react-router-dom'
import CoinBot from './homePage'

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
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Your wealth.')).toBeInTheDocument()
    expect(screen.getByText('Automated.')).toBeInTheDocument()
  })

  test('renders stats section', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Trading Volume')).toBeInTheDocument()
    expect(screen.getByText('Uptime')).toBeInTheDocument()
    expect(screen.getByText('Active Users')).toBeInTheDocument()
    expect(screen.getByText('Liquidation Risk')).toBeInTheDocument()
  })

  test('renders feature cards', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Buy Low, Sell High')).toBeInTheDocument()
    expect(screen.getByText('Auto-Stake')).toBeInTheDocument()
    expect(screen.getByText('Zero Effort')).toBeInTheDocument()
  })

  test('renders CTA buttons', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Start Trading')).toBeInTheDocument()
    expect(screen.getByText('GitHub')).toBeInTheDocument()
  })

  test('renders footer', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('CoinBot')).toBeInTheDocument()
  })
})
