import { render, screen } from '@testing-library/react'
import { beforeAll, describe, test, expect, vi } from 'vitest'

import PnlChart from './PnlChart'
import { getMyContracts, getContractOrders, type ContractItem, type ContractOrder } from '../../services/contract'
import { fetchTickerPrice } from '../../services/kucoin'
import { ContractsProvider } from '../../hooks/useContractsData'

vi.mock('../../services/contract', () => ({
  getMyContracts: vi.fn(),
  getContractOrders: vi.fn(),
}))
vi.mock('../../services/kucoin', () => ({
  fetchTickerPrice: vi.fn().mockResolvedValue(120),
}))

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 800 })
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => 220 })
})

const mockContracts = vi.mocked(getMyContracts)
const mockOrders = vi.mocked(getContractOrders)

const CONTRACT: ContractItem = {
  id: 'c1',
  ticker: 'BTC/USDT',
  status: 'active',
  init_fund: '1000',
  available_fund: '1000',
  total_traded_fund: '0',
  sl_pct: 2,
  tp_pct: 5,
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
}

const baseOrder: Omit<ContractOrder, 'id' | 'action' | 'status' | 'pnl'> = {
  side: 'buy',
  requested_qty: '0',
  filled_qty: '0',
  fill_price: null,
  funds: '0',
  fee: '0',
  parent_order_id: null,
  created_at: new Date(Date.now() - 2 * 86_400_000).toISOString(),
  updated_at: new Date(Date.now() - 2 * 86_400_000).toISOString(),
}

describe('PnlChart', () => {
  test('plots realized close-order PnL and marks open entries as unrealized', async () => {
    mockContracts.mockResolvedValue([CONTRACT])
    mockOrders.mockResolvedValue([
      {
        ...baseOrder,
        id: 'entry-open',
        action: 'open',
        status: 'open',
        pnl: null,
        requested_qty: '1',
        funds: '100',
      },
      {
        ...baseOrder,
        id: 'entry-closed',
        action: 'open',
        status: 'filled',
        pnl: null,
        requested_qty: '1',
        filled_qty: '1',
        fill_price: '100',
        funds: '100',
        fee: '1',
      },
      {
        ...baseOrder,
        id: 'close-1',
        action: 'close',
        status: 'filled',
        pnl: '25.5',
        parent_order_id: 'entry-closed',
        requested_qty: '1',
        filled_qty: '1',
        fill_price: '125',
        funds: '100',
        fee: '1',
      },
    ] as ContractOrder[])
    vi.mocked(fetchTickerPrice).mockResolvedValue(120)

    render(<ContractsProvider><PnlChart /></ContractsProvider>)

    expect(await screen.findByText('PnL Performance')).toBeInTheDocument()
    expect(await screen.findByRole('img', { name: 'Cumulative realized PnL over time' })).toBeInTheDocument()
    expect(screen.getAllByText('+$25.50').length).toBeGreaterThan(0)
    expect(screen.getAllByText('+$20.00').length).toBeGreaterThan(0)
    expect(screen.getAllByText('+$45.50').length).toBeGreaterThan(0)
  })
})
