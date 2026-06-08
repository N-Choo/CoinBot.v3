import { render, screen } from '@testing-library/react'
import { describe, test, expect } from 'vitest'
import BalanceStats from './BalanceStats'

describe('BalanceStats', () => {
  test('renders all 3 stat cards', async () => {
    render(<BalanceStats />)
    expect(await screen.findByText('Total Balance')).toBeInTheDocument()
    expect(await screen.findByText('Free Balance')).toBeInTheDocument()
    expect(await screen.findByText('Locked Balance')).toBeInTheDocument()
  })

  test('shows Total Balance, Free Balance, Locked Balance values', async () => {
    render(<BalanceStats />)
    expect(await screen.findByText('$124,582.40')).toBeInTheDocument()
    expect(await screen.findByText('$48,230.15')).toBeInTheDocument()
    expect(await screen.findByText('$76,352.25')).toBeInTheDocument()
  })

  test('shows positive changes with price-up-bg class', async () => {
    render(<BalanceStats />)
    const changes = await screen.findAllByText(/[+-]\d+\.\d+%/)
    expect(changes).toHaveLength(3)
    expect(changes[0]).toHaveClass('price-up-bg')
    expect(changes[1]).toHaveClass('price-up-bg')
    expect(changes[2]).toHaveClass('price-down-bg')
  })
})
