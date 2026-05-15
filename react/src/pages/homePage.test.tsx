import { render, screen } from '@testing-library/react'
import { describe, test, expect } from 'vitest'
import { BrowserRouter } from 'react-router-dom'
import CoinBot from './homePage'

describe('CoinBot (homePage)', () => {
  test('renders hero section with title', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Trade smarter.')).toBeInTheDocument()
    expect(screen.getByText('Earn passively.')).toBeInTheDocument()
  })

  test('renders stats section', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Trading Volume')).toBeInTheDocument()
    expect(screen.getByText('$450M+')).toBeInTheDocument()
    expect(screen.getByText('99.9%')).toBeInTheDocument()
    expect(screen.getByText('10K+')).toBeInTheDocument()
    expect(screen.getByText('Active Users')).toBeInTheDocument()
  })

  test('renders feature cards', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Buy Low, Sell High')).toBeInTheDocument()
    expect(screen.getByText('Auto-Stake')).toBeInTheDocument()
    expect(screen.getByText('Zero Effort')).toBeInTheDocument()
  })

  test('renders reactor system diagram', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('Deposit')).toBeInTheDocument()
    expect(screen.getByText('Execute')).toBeInTheDocument()
    expect(screen.getByText('Exchange')).toBeInTheDocument()
    expect(screen.getByText('Stake')).toBeInTheDocument()
    expect(screen.getByText('Withdraw')).toBeInTheDocument()
  })

  test('renders footer', () => {
    render(<BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}><CoinBot /></BrowserRouter>)
    expect(screen.getByText('CoinBot')).toBeInTheDocument()
  })
})
