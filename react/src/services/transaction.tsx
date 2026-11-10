import axios from 'axios'
import { ethers } from 'ethers'

const USDT_ADDRESS = '0xdAC17F958D2ee523a2206206994597C13D831ec7'
let PLATFORM_WALLET = '0x1cbabcafbfea9aa787b186d3c52a2c81c945ed4c'

export async function loadConfig(): Promise<void> {
  try {
    const res = await axios.get('/api/config')
    PLATFORM_WALLET = res.data.platform_wallet
  } catch {
    // will try again on first sendCoin call
  }
}

const ERC20_ABI = [
  'function transfer(address to, uint256 value) public returns (bool)',
  'function balanceOf(address owner) public view returns (uint256)',
]

export async function getBalance(): Promise<string> {
  if (!window.ethereum) throw new Error('MetaMask not installed')

  const provider = new ethers.BrowserProvider(window.ethereum)
  // Silent read of already-authorized accounts (no prompt); fall back to an
  // explicit connect request only when there is no authorized account.
  let accounts = await provider.send('eth_accounts', [])
  if (!Array.isArray(accounts) || accounts.length === 0) {
    accounts = await provider.send('eth_requestAccounts', [])
  }
  if (!Array.isArray(accounts) || accounts.length === 0) {
    throw new Error('Wallet not connected')
  }

  const contract = new ethers.Contract(USDT_ADDRESS, ERC20_ABI, provider)
  const balance = await contract.balanceOf(accounts[0])
  return ethers.formatUnits(balance, 6)
}

export async function sendUSDT(amount: string): Promise<boolean> {
  if (!window.ethereum) throw new Error('MetaMask not installed')

  const provider = new ethers.BrowserProvider(window.ethereum)
  await provider.send('eth_requestAccounts', [])
  const signer = await provider.getSigner()
  const sender = await signer.getAddress()

  const network = await provider.getNetwork()
  if (network.chainId !== 1n) {
    throw new Error('Please switch to Ethereum mainnet')
  }

  const amountParsed = ethers.parseUnits(amount, 6)
  const contract = new ethers.Contract(USDT_ADDRESS, ERC20_ABI, provider)
  const balance = await contract.balanceOf(sender)

  if (balance < amountParsed) {
    const formatted = ethers.formatUnits(balance, 6)
    throw new Error(`Insufficient USDT balance: you have ${formatted} USDT`)
  }

  const signerContract = contract.connect(signer) as ethers.Contract
  const tx = await signerContract.transfer(PLATFORM_WALLET, amountParsed)
  await tx.wait()
  await axios.post('/api/transactions/deposit', { tx_hash: tx.hash }, { timeout: 15000 })

  return true
}

export interface ActivityItem {
  id: string
  ticker: string
  amount: string
  status: string
  created_at: string
  tx_hash: string
  kind: 'deposit' | 'withdrawal'
  failure_reason?: string | null
  fee?: string
  total?: string
  completed_at?: string | null
}

type DepositActivity = Omit<ActivityItem, 'kind' | 'failure_reason'> & {
  reason?: string | null
}

export async function getActivity(): Promise<ActivityItem[]> {
  const res = await axios.get('/api/transactions')
  return (res.data as DepositActivity[])
    .slice(0, 20)
    .map(({ reason, ...deposit }) => ({
      ...deposit,
      kind: 'deposit' as const,
      failure_reason: reason ?? null,
    }))
}

/**
 * Turn wallet/API errors into a short, user-safe message.
 *
 * A rejected signature is a normal user choice, not a failure dump. Never show
 * the raw ethers `ACTION_REJECTED` payload (JSON-RPC params, typed data, etc.).
 */
export function walletErrorMessage(err: unknown, fallback = 'Transaction failed'): string {
  if (err && typeof err === 'object') {
    const e = err as {
      code?: string | number
      message?: string
      info?: { error?: { code?: number; message?: string } }
      response?: { data?: unknown }
    }

    if (e.code === 'ACTION_REJECTED' || e.code === 4001 || e.info?.error?.code === 4001) {
      return 'Signature cancelled. No funds were moved.'
    }

    const data = e.response?.data
    if (typeof data === 'string' && data.length > 0) return data
    if (data && typeof data === 'object') {
      const body = data as { error?: unknown; message?: unknown }
      if (typeof body.error === 'string' && body.error.length > 0) return body.error
      if (body.error && typeof body.error === 'object') {
        const nested = (body.error as { message?: unknown }).message
        if (typeof nested === 'string' && nested.length > 0) return nested
      }
      if (typeof body.message === 'string' && body.message.length > 0) return body.message
    }

    if (typeof e.message === 'string') {
      if (/user rejected|user denied|ethers-user-denied/i.test(e.message)) {
        return 'Signature cancelled. No funds were moved.'
      }
      // Axios' generic status text is not useful to the user.
      if (/request failed with status code \d+/i.test(e.message)) return fallback
      return e.message.length > 200 ? `${e.message.slice(0, 197)}...` : e.message
    }
  }

  return fallback
}

/** Machine-readable code from a structured API error (409/429). */
export function withdrawErrorCode(err: unknown): string | undefined {
  if (!err || typeof err !== 'object' || !('response' in err)) return undefined
  const data = (err as { response?: { data?: unknown } }).response?.data
  if (!data || typeof data !== 'object' || !('code' in data)) return undefined
  const code = (data as { code?: unknown }).code
  return typeof code === 'string' ? code : undefined
}

/** Exact decimal addition for UI display and the signed payload. */
export function addDecimals(left: string, right: string, precision: number): string {
  const scale = 10n ** BigInt(precision)
  const toScaled = (value: string): bigint => {
    const [whole = '0', fraction = ''] = value.trim().split('.')
    const padded = (fraction + '0'.repeat(precision)).slice(0, precision)
    return BigInt(whole || '0') * scale + BigInt(padded || '0')
  }
  const sum = toScaled(left) + toScaled(right)
  const whole = sum / scale
  const fraction = (sum % scale).toString().padStart(precision, '0')
  return `${whole}.${fraction}`
}

/** Whether `value` has no more than `precision` non-trailing decimals. */
export function fitsPrecision(value: string, precision: number): boolean {
  if (!/^\d+(\.\d+)?$/.test(value.trim())) return false
  const fraction = value.split('.')[1] ?? ''
  return fraction.replace(/0+$/, '').length <= precision
}

export const WITHDRAW_DOMAIN: ethers.TypedDataDomain = {
  name: 'CoinBot',
  version: '1',
  chainId: 1,
}

export const WITHDRAW_TYPES: Record<string, ethers.TypedDataField[]> = {
  Withdrawal: [
    { name: 'nonce', type: 'string' },
    { name: 'currency', type: 'string' },
    { name: 'chain', type: 'string' },
    { name: 'chain_id', type: 'uint256' },
    { name: 'destination', type: 'address' },
    { name: 'amount', type: 'string' },
    { name: 'fee', type: 'string' },
    { name: 'total', type: 'string' },
    { name: 'issued_at', type: 'uint256' },
    { name: 'expires_at', type: 'uint256' },
  ],
}

export interface WithdrawQuote {
  destination: string
  currency: string
  chain: string
  chain_id: number
  chain_name: string
  fee: string
  min: string
  precision: number
  available_balance: string
  max_withdrawable: string
}

export interface WithdrawItem {
  id: string
  amount: string
  fee: string
  total: string
  status: 'pending' | 'processing' | 'submitted' | 'completed' | 'failed' | 'expired'
  tx_hash: string | null
  failure_reason: string | null
  created_at: string
  completed_at: string | null
}

export async function getWithdrawQuote(): Promise<WithdrawQuote> {
  const res = await axios.get('/api/transactions/withdraw/quote')
  return res.data
}

export async function getWithdrawNonce(): Promise<string> {
  const res = await axios.get('/api/transactions/withdraw/nonce')
  return res.data.nonce
}

export async function submitWithdraw(body: {
  nonce: string
  message: string
  signature: string
}): Promise<void> {
  await axios.post('/api/transactions/withdraw', body, { timeout: 30000 })
}

export async function getWithdrawals(): Promise<WithdrawItem[]> {
  const res = await axios.get('/api/transactions/withdrawals')
  return res.data
}

export function buildWithdrawPayload(
  quote: WithdrawQuote,
  amount: string,
  nonce: string,
  now: number,
) {
  return {
    nonce,
    currency: quote.currency,
    chain: quote.chain,
    chain_id: quote.chain_id,
    destination: quote.destination,
    amount,
    fee: quote.fee,
    total: addDecimals(amount, quote.fee, quote.precision),
    issued_at: now,
    expires_at: now + 300,
  }
}

export async function signWithdraw(
  signer: ethers.Signer,
  quote: WithdrawQuote,
  amount: string,
): Promise<{ nonce: string; message: string; signature: string }> {
  const nonce = await getWithdrawNonce()
  const now = Math.floor(Date.now() / 1000)
  const payload = buildWithdrawPayload(quote, amount, nonce, now)
  const message = JSON.stringify(payload)
  const signature = await signer.signTypedData(WITHDRAW_DOMAIN, WITHDRAW_TYPES, payload)
  return { nonce, message, signature }
}
