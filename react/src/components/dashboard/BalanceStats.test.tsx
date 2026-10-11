import { render, screen } from '@testing-library/react'
import { describe, test, expect, vi } from 'vitest'
import BalanceStats from './BalanceStats'
import { getUserStats } from '../../services/stats'

vi.mock('../../services/stats', () => ({ getUserStats: vi.fn() }))
const mockGet = vi.mocked(getUserStats)

describe('BalanceStats', () => {
  test('renders the four stat cards', async () => {
    mockGet.mockResolvedValue({ total: '124582.40', free: '48230.15', locked: '76352.25', realized_pnl: '12400' })
    render(<BalanceStats />)
    expect(await screen.findByText('Total Balance')).toBeInTheDocument()
    expect(screen.getByText('Free Balance')).toBeInTheDocument()
    expect(screen.getByText('Locked Balance')).toBeInTheDocument()
    expect(screen.getByText('Realized PnL')).toBeInTheDocument()
  })

  test('renders the balances returned by the API', async () => {
    mockGet.mockResolvedValue({ total: '124582.40', free: '48230.15', locked: '76352.25', realized_pnl: '12400' })
    render(<BalanceStats />)
    expect(await screen.findByText('$124,582.40')).toBeInTheDocument()
    expect(screen.getByText('$48,230.15')).toBeInTheDocument()
    expect(screen.getByText('$76,352.25')).toBeInTheDocument()
    expect(screen.getByText('$12,400.00')).toBeInTheDocument()
  })
})
