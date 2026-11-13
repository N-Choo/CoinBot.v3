import { describe, expect, test } from 'vitest'

import { isClosedOrder, type ContractOrder } from './contract'

const order = (over: Partial<ContractOrder>): ContractOrder => ({
  id: 'o',
  side: 'sell',
  action: 'close',
  status: 'filled',
  requested_qty: '1',
  filled_qty: '1',
  fill_price: '1',
  funds: '1',
  fee: '0',
  pnl: '1',
  parent_order_id: 'entry',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  ...over,
})

describe('isClosedOrder', () => {
  test('is true only for a filled close leg', () => {
    expect(isClosedOrder(order({}))).toBe(true)
    expect(isClosedOrder(order({ status: 'FILLED' }))).toBe(true)
    expect(isClosedOrder(order({ action: 'open', parent_order_id: null }))).toBe(false)
    expect(isClosedOrder(order({ status: 'open' }))).toBe(false)
    expect(isClosedOrder(order({ status: 'cancelled' }))).toBe(false)
  })
})
