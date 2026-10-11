import { render, screen } from '@testing-library/react'
import { describe, test, expect, vi } from 'vitest'
import BalanceStats from './BalanceStats'
import { getUserStats } from '../../services/stats'
import { ContractsProvider } from '../../hooks/useContractsData'

vi.mock('../../services/stats', () => ({ getUserStats: vi.fn() }))
vi.mock('../../services/contract', () => ({
  getMyContracts: vi.fn().mockResolvedValue([]),
  getContractOrders: vi.fn().mockResolvedValue([]),
}))

const mockGet = vi.mocked(getUserStats)

const wrap = (ui: React.ReactNode) => render(<ContractsProvider>{ui}</ContractsProvider>)

describe('BalanceStats', () => {
  test('renders the four stat cards', async () => {
    mockGet.mockResolvedValue({ total: '124582.40', free: '48230.15', locked: '76352.25', realized_pnl: '0' })
    wrap(<BalanceStats />)
    expect(await screen.findByText('Total Balance')).toBeInTheDocument()
    expect(screen.getByText('Free Balance')).toBeInTheDocument()
    expect(screen.getByText('Locked Balance')).toBeInTheDocument()
    expect(screen.getByText('Realized PnL')).toBeInTheDocument()
  })

  test('renders the balances returned by the API', async () => {
    mockGet.mockResolvedValue({ total: '124582.40', free: '48230.15', locked: '76352.25', realized_pnl: '0' })
    wrap(<BalanceStats />)
    expect(await screen.findByText('$124,582.40')).toBeInTheDocument()
    expect(screen.getByText('$48,230.15')).toBeInTheDocument()
    expect(screen.getByText('$76,352.25')).toBeInTheDocument()
  })
})
