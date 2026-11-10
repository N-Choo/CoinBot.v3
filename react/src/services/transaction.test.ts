import { describe, expect, test } from 'vitest'
import {
  WITHDRAW_DOMAIN,
  WITHDRAW_TYPES,
  addDecimals,
  buildWithdrawPayload,
  fitsPrecision,
  walletErrorMessage,
  withdrawErrorCode,
  type WithdrawQuote,
} from './transaction'

const QUOTE: WithdrawQuote = {
  destination: '0x2300dc555ba1dda773935a2fac059e3c8f1d14a7',
  currency: 'USDT',
  chain: 'eth',
  chain_id: 1,
  chain_name: 'ERC20',
  fee: '5.5',
  min: '30',
  precision: 6,
  available_balance: '100',
  max_withdrawable: '94.5',
}

describe('withdraw signing payload', () => {
  test('uses the EIP-712 domain expected by the backend', () => {
    expect(WITHDRAW_DOMAIN).toEqual({ name: 'CoinBot', version: '1', chainId: 1 })
  })

  test('declares the Withdrawal fields in backend order', () => {
    expect(WITHDRAW_TYPES.Withdrawal.map(field => field.name)).toEqual([
      'nonce',
      'currency',
      'chain',
      'chain_id',
      'destination',
      'amount',
      'fee',
      'total',
      'issued_at',
      'expires_at',
    ])
  })

  test('builds an exact total and a five-minute window', () => {
    const payload = buildWithdrawPayload(QUOTE, '30', 'nonce-1', 1_800_000_000)
    expect(payload.total).toBe('35.500000')
    expect(payload.expires_at).toBe(1_800_000_300)
    expect(payload.chain_id).toBe(1)
    expect(payload.destination).toBe(QUOTE.destination)
  })

  test('adds decimals without float drift', () => {
    expect(addDecimals('0.1', '0.2', 6)).toBe('0.300000')
    expect(addDecimals('30', '5.5', 6)).toBe('35.500000')
  })

  test('enforces the exchange precision', () => {
    expect(fitsPrecision('30.123456', 6)).toBe(true)
    expect(fitsPrecision('30.1234567', 6)).toBe(false)
    expect(fitsPrecision('30.0000000', 6)).toBe(true)
    expect(fitsPrecision('abc', 6)).toBe(false)
  })
})

describe('walletErrorMessage', () => {
  test('turns a rejected signature into a cancellation message', () => {
    const rejected = {
      code: 'ACTION_REJECTED',
      info: { error: { code: 4001, message: 'ethers-user-denied' } },
    }
    expect(walletErrorMessage(rejected)).toBe('Signature cancelled. No funds were moved.')
  })

  test('does not leak the raw ethers payload', () => {
    const message = 'user rejected action (action="signTypedData", info={ "payload": ... })'
    expect(walletErrorMessage(new Error(message))).toBe(
      'Signature cancelled. No funds were moved.',
    )
  })

  test('reads structured API errors', () => {
    expect(walletErrorMessage({ response: { data: { error: 'fee changed' } } })).toBe('fee changed')
    expect(
      walletErrorMessage({ response: { data: { error: { message: 'nested' } } } }),
    ).toBe('nested')
  })

  test('falls back for generic axios status messages', () => {
    expect(walletErrorMessage(new Error('Request failed with status code 500'), 'Failed')).toBe(
      'Failed',
    )
  })
})

describe('withdrawErrorCode', () => {
  test('extracts structured conflict codes', () => {
    expect(withdrawErrorCode({ response: { data: { code: 'FEE_CHANGED' } } })).toBe('FEE_CHANGED')
    expect(withdrawErrorCode({ response: { data: 'plain text' } })).toBeUndefined()
  })
})
