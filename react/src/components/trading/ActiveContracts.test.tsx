import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, test, vi } from 'vitest'

import ActiveContracts from './ActiveContracts'
import { getMyContracts } from '../../services/contract'

vi.mock('../../services/contract', () => ({
  getMyContracts: vi.fn(),
}))

const mockGet = vi.mocked(getMyContracts)

const ROWS = [
  {
    id: '1',
    ticker: 'BTC/USDT',
    status: 'active',
    init_fund: '5000',
    available_fund: '4500',
    total_traded_fund: '500',
    sl_pct: 2,
    tp_pct: 5,
    created_at: '2026-10-01T00:00:00Z',
    updated_at: '2026-10-01T00:00:00Z',
  },
  {
    id: '2',
    ticker: 'ETH/USDT',
    status: 'completed',
    init_fund: '2000',
    available_fund: '2000',
    total_traded_fund: '0',
    sl_pct: 1.5,
    tp_pct: 4,
    created_at: '2026-10-02T00:00:00Z',
    updated_at: '2026-10-02T00:00:00Z',
  },
]

beforeEach(() => {
  mockGet.mockReset()
})

describe('ActiveContracts', () => {
  test('renders contracts returned by the API', async () => {
    mockGet.mockResolvedValue(ROWS)
    render(<ActiveContracts />)
    expect(await screen.findByText('BTC')).toBeInTheDocument()
    expect(screen.getByText('ETH')).toBeInTheDocument()
    expect(screen.getByText('2 contracts')).toBeInTheDocument()
  })

  test('shows the empty state when there are no contracts', async () => {
    mockGet.mockResolvedValue([])
    render(<ActiveContracts />)
    expect(await screen.findByText('No contracts')).toBeInTheDocument()
  })

  test('shows the error state when the request fails', async () => {
    mockGet.mockRejectedValue(new Error('boom'))
    render(<ActiveContracts />)
    expect(await screen.findByText('Failed to load contracts')).toBeInTheDocument()
  })

  test('row click opens details with no add/withdraw controls', async () => {
    mockGet.mockResolvedValue(ROWS)
    render(<ActiveContracts />)
    await userEvent.click(await screen.findByText('BTC'))
    expect(screen.getByText('Allocated')).toBeInTheDocument()
    expect(screen.getByText('$5,000')).toBeInTheDocument()
    expect(screen.getByText('BTC/USDT')).toBeInTheDocument()
    expect(screen.getByText('2% / 5%')).toBeInTheDocument()
    expect(screen.queryByText('ADD')).not.toBeInTheDocument()
    expect(screen.queryByText('WITHDRAW')).not.toBeInTheDocument()
  })
})
