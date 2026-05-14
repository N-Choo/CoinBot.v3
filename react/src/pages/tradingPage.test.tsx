import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { BrowserRouter } from 'react-router-dom'

describe('TradingPage', () => {
  it('renders the TradingView chart container', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('BTC/USDT')).toBeTruthy()
    })
  })

  it('renders the DCA Smart-Contract settings', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('DCA Smart-Contract')).toBeTruthy()
    })
  })

  it('renders mock market list with trading pairs', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('SOL')).toBeTruthy()
      expect(screen.getByText('DOGE')).toBeTruthy()
      expect(screen.getByText('+2.4%')).toBeTruthy()
    })
  })

  it('shows price for selected pair in header', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      const prices = screen.getAllByText('64,230.50')
      expect(prices.length).toBeGreaterThan(0)
    })
  })

  it('selects a different pair from market list', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('SOL')).toBeTruthy()
    })
    fireEvent.click(screen.getByText('SOL'))
    await waitFor(() => {
      const headings = screen.getAllByText('SOL/USDT')
      expect(headings.length).toBeGreaterThan(0)
    })
  })

  it('renders the search input', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Search markets...')).toBeTruthy()
    })
  })

  it('renders the START TRADING button', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('START TRADING')).toBeTruthy()
    })
  })
})
