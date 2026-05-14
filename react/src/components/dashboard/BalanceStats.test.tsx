import { render, screen } from '@testing-library/react'
import { describe, test, expect } from 'vitest'
import BalanceStats from './BalanceStats'

describe('BalanceStats', () => {
  test('renders all 3 stat cards', () => {
    render(<BalanceStats />)
    expect(screen.getByText('Total Balance')).toBeInTheDocument()
    expect(screen.getByText('Free Balance')).toBeInTheDocument()
    expect(screen.getByText('Locked Balance')).toBeInTheDocument()
  })

  test('shows Total Balance, Free Balance, Locked Balance values', () => {
    render(<BalanceStats />)
    expect(screen.getByText('$124,582.40')).toBeInTheDocument()
    expect(screen.getByText('$48,230.15')).toBeInTheDocument()
    expect(screen.getByText('$76,352.25')).toBeInTheDocument()
  })

  test('shows positive changes with price-up-bg class', () => {
    render(<BalanceStats />)
    const changes = screen.getAllByText(/[+-]\d+\.\d+%/)
    expect(changes).toHaveLength(3)
    expect(changes[0]).toHaveClass('price-up-bg')
    expect(changes[1]).toHaveClass('price-up-bg')
    expect(changes[2]).toHaveClass('price-down-bg')
  })
})
