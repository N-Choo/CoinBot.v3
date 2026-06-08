import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { BrowserRouter } from 'react-router-dom'

describe('TradingPage', () => {
  it('renders the TradingView chart container', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getAllByText('BTC/USDT').length).toBeGreaterThanOrEqual(1)
    })
  })

  it('renders the bot strategy panel', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('Bot Strategy')).toBeTruthy()
      expect(screen.getByText('Start Bot')).toBeTruthy()
      expect(screen.getByText('Allocation')).toBeTruthy()
    })
  })

  it('shows price for selected pair in ticker', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><Trading /></BrowserRouter>)
    await waitFor(() => {
      expect(screen.getByText('$64,230.50')).toBeTruthy()
    })
  })

  it('opens pair dropdown and searches for a coin', async () => {
    const { default: Trading } = await import('./tradingPage')
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><Trading /></BrowserRouter>)
    const pairBtn = screen.getByRole('button', { name: /BTC\/USDT/i })
    fireEvent.click(pairBtn)
    const input = screen.getByPlaceholderText('Search coins...')
    fireEvent.change(input, { target: { value: 'SOL' } })
    await waitFor(() => {
      expect(screen.getByText('SOL/USDT')).toBeTruthy()
    })
  })
})
