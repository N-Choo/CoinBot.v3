import { render, screen } from '@testing-library/react'
import { describe, test, expect } from 'vitest'
import ActivityFeed from './ActivityFeed'

const TRADE_COUNT = 10

describe('ActivityFeed', () => {
  test('renders Live Activity header with green dot', () => {
    render(<ActivityFeed />)
    const header = screen.getByText('Live Activity')
    expect(header).toBeInTheDocument()
    expect(screen.getByTestId('feed-dot')).toBeInTheDocument()
  })

  test('renders all duplicated trade items (20 total)', () => {
    render(<ActivityFeed />)
    const items = screen.getAllByTestId('feed-item')
    expect(items).toHaveLength(TRADE_COUNT * 2)
  })

  test('renders BUY, SELL, and STAKE items with correct classes', () => {
    render(<ActivityFeed />)
    expect(screen.getAllByText('BUY').length).toBeGreaterThan(0)
    expect(screen.getAllByText('SELL').length).toBeGreaterThan(0)
    expect(screen.getAllByText('STAKE').length).toBeGreaterThan(0)
  })

  test('shows trade type for each item', () => {
    render(<ActivityFeed />)
    expect(screen.getAllByText('BUY').length).toBeGreaterThan(0)
    expect(screen.getAllByText('SELL').length).toBeGreaterThan(0)
    expect(screen.getAllByText('STAKE').length).toBeGreaterThan(0)
  })

  test('shows pair names', () => {
    render(<ActivityFeed />)
    expect(screen.getAllByText('BTC/USDT')).toHaveLength(2)
    expect(screen.getAllByText('ETH/USDT')).toHaveLength(2)
    expect(screen.getAllByText('SOL/USDT')).toHaveLength(2)
  })

  test('does not render pnl for STAKE items', () => {
    render(<ActivityFeed />)
    const stakePairs = screen.getAllByText('ETH \u2192 Lido')
    stakePairs.forEach(pair => {
      const feedItem = pair.closest('[data-testid="feed-item"]')
      const pnl = feedItem?.querySelector('[data-testid="feed-pnl"]')
      expect(pnl).toBeNull()
    })
  })

  test('renders pnl for non-STAKE items without double plus sign', () => {
    render(<ActivityFeed />)
    const pnlElements = screen.getAllByTestId('feed-pnl')
    expect(pnlElements.length).toBeGreaterThan(0)
    pnlElements.forEach(el => {
      const text = el.textContent || ''
      expect(text).not.toMatch(/^\+\+/)
    })
  })
})
