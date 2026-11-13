import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, test, vi } from 'vitest'

import ActiveContracts from './ActiveContracts'
import { getContractOrders, getMyContracts } from '../../services/contract'
import { ContractsProvider } from '../../hooks/useContractsData'

const renderAC = () => render(<ContractsProvider><ActiveContracts /></ContractsProvider>)

vi.mock('../../services/contract', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../services/contract')>()
  return {
    ...actual,
    getMyContracts: vi.fn(),
    getContractOrders: vi.fn().mockResolvedValue([]),
  }
})

const mockGet = vi.mocked(getMyContracts)
const mockOrders = vi.mocked(getContractOrders)

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
  mockOrders.mockReset()
  mockOrders.mockResolvedValue([])
})

describe('ActiveContracts', () => {
  test('renders contracts returned by the API', async () => {
    mockGet.mockResolvedValue(ROWS)
    renderAC()
    expect(await screen.findByText('BTC')).toBeInTheDocument()
    expect(screen.getByText('ETH')).toBeInTheDocument()
    expect(screen.getByText('2 contracts')).toBeInTheDocument()
  })

  test('shows the empty state when there are no contracts', async () => {
    mockGet.mockResolvedValue([])
    renderAC()
    expect(await screen.findByText('No contracts')).toBeInTheDocument()
  })

  test('shows the error state when the request fails', async () => {
    mockGet.mockRejectedValue(new Error('boom'))
    renderAC()
    expect(await screen.findByText('Failed to load contracts')).toBeInTheDocument()
  })

  test('row click opens details with no add/withdraw controls', async () => {
    mockGet.mockResolvedValue(ROWS)
    renderAC()
    await userEvent.click(await screen.findByText('BTC'))
    expect(screen.getByText('Allocated')).toBeInTheDocument()
    expect(screen.getByText('$5,000')).toBeInTheDocument()
    expect(screen.getByText('BTC/USDT')).toBeInTheDocument()
    expect(screen.getByText('2% / 5%')).toBeInTheDocument()
    expect(screen.queryByText('ADD')).not.toBeInTheDocument()
    expect(screen.queryByText('WITHDRAW')).not.toBeInTheDocument()
  })

  test('counts one closed round trip, not its two filled legs', async () => {
    mockGet.mockResolvedValue(ROWS)
    mockOrders.mockImplementation(async (id: string) =>
      id === '1'
        ? [
            {
              id: 'entry',
              side: 'buy',
              action: 'open',
              status: 'filled',
              requested_qty: '0.1',
              filled_qty: '0.1',
              fill_price: '100',
              funds: '10',
              fee: '0.01',
              pnl: null,
              parent_order_id: null,
              created_at: '2026-10-01T00:00:00Z',
              updated_at: '2026-10-01T00:00:00Z',
            },
            {
              id: 'exit',
              side: 'sell',
              action: 'close',
              status: 'filled',
              requested_qty: '0.1',
              filled_qty: '0.1',
              fill_price: '110',
              funds: '11',
              fee: '0.01',
              pnl: '1',
              parent_order_id: 'entry',
              created_at: '2026-10-01T01:00:00Z',
              updated_at: '2026-10-01T01:00:00Z',
            },
          ]
        : [],
    )

    renderAC()
    await userEvent.click(await screen.findByText('BTC'))

    expect(await screen.findByTestId('closed-count')).toHaveTextContent('1')
    expect(screen.getByTestId('orders-count')).toHaveTextContent('2')
  })
})
