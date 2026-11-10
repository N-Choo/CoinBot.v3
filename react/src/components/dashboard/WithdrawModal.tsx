import { useCallback, useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { ethers } from 'ethers'
import {
  addDecimals,
  fitsPrecision,
  getWithdrawQuote,
  signWithdraw,
  submitWithdraw,
  walletErrorMessage,
  withdrawErrorCode,
  type WithdrawQuote,
} from '../../services/transaction'

interface WithdrawModalProps {
  onClose: () => void
  onSubmitted: () => void
}

export default function WithdrawModal({ onClose, onSubmitted }: WithdrawModalProps) {
  const [quote, setQuote] = useState<WithdrawQuote | null>(null)
  const [amount, setAmount] = useState('')
  const [loading, setLoading] = useState(false)
  const [loadingQuote, setLoadingQuote] = useState(false)
  const [error, setError] = useState('')

  const loadQuote = useCallback(() => {
    setLoadingQuote(true)
    setError('')
    getWithdrawQuote()
      .then(setQuote)
      .catch(err => {
        const status = (err as { response?: { status?: number } })?.response?.status
        setError(
          status === 401
            ? 'Your session expired. Please sign in again.'
            : walletErrorMessage(err, 'Unable to load the withdrawal fee'),
        )
      })
      .finally(() => setLoadingQuote(false))
  }, [])

  useEffect(() => {
    // Defer so the initial state update is not part of the effect body.
    const timer = window.setTimeout(loadQuote, 0)
    return () => window.clearTimeout(timer)
  }, [loadQuote])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !loading) onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [loading, onClose])

  const min = quote ? Number(quote.min) : 0
  const max = quote ? Number(quote.max_withdrawable) : 0
  const amountNumber = Number(amount)
  const precisionOk = quote ? fitsPrecision(amount, quote.precision) : false
  const positive = amount.length > 0 && Number.isFinite(amountNumber) && amountNumber > 0
  const total = quote && positive && precisionOk
    ? addDecimals(amount, quote.fee, quote.precision)
    : '—'
  const invalid = !quote || !positive || amountNumber < min || amountNumber > max || !precisionOk

  const submit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (loading || invalid || !quote) return

    setLoading(true)
    setError('')
    try {
      if (!window.ethereum) throw new Error('MetaMask not installed')

      const provider = new ethers.BrowserProvider(window.ethereum)
      const network = await provider.getNetwork()
      if (network.chainId !== 1n) {
        throw new Error('Please switch to Ethereum mainnet')
      }

      const signer = await provider.getSigner()
      const connected = await signer.getAddress()
      if (connected.toLowerCase() !== quote.destination.toLowerCase()) {
        throw new Error('Connected wallet does not match your signed-in wallet')
      }

      const signed = await signWithdraw(signer, quote, amount)
      await submitWithdraw(signed)
      onSubmitted()
      onClose()
    } catch (err: unknown) {
      const code = withdrawErrorCode(err)
      if (code === 'FEE_CHANGED') {
        const fresh = await getWithdrawQuote().catch(() => null)
        if (fresh) setQuote(fresh)
        setError('The network fee changed. Review the new total and sign again.')
      } else if (code === 'NONCE_CONFLICT') {
        setError('This request was already used or expired. Check your activity, then start a new withdrawal.')
        onSubmitted()
      } else {
        setError(walletErrorMessage(err, 'Withdrawal failed'))
      }
    } finally {
      setLoading(false)
    }
  }

  return createPortal(
    <div
      className="fixed inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center z-[9999]"
      onClick={() => { if (!loading) onClose() }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Withdraw USDT"
        onClick={event => event.stopPropagation()}
        className="w-full max-w-[420px] p-6 bg-bg-panel border border-border-light rounded-lg"
      >
        <h2 className="m-0 mb-1.5 text-base text-text-main font-bold">Withdraw Assets</h2>
        <p className="text-[11px] text-text-muted mb-4">USDT (ERC20) to your connected wallet.</p>

        {quote && (
          <div className="bg-[rgba(255,255,255,0.02)] border border-border-light p-3 rounded mb-4 flex flex-col gap-1.5 text-[11px] text-text-muted">
            <div className="flex justify-between gap-3">
              <span className="shrink-0">Destination</span>
              <span className="text-text-main font-mono break-all text-right max-w-[240px]">{quote.destination}</span>
            </div>
            <div className="flex justify-between">
              <span>Network</span>
              <span className="text-text-main">USDT ({quote.chain_name})</span>
            </div>
            <div className="flex justify-between">
              <span>Network fee</span>
              <span className="text-text-main">{quote.fee} USDT</span>
            </div>
            <div className="flex justify-between">
              <span>Minimum</span>
              <span>{quote.min} USDT</span>
            </div>
            <div className="flex justify-between">
              <span>Available</span>
              <span className="text-neon-teal">{quote.available_balance} USDT</span>
            </div>
          </div>
        )}

        <form onSubmit={submit}>
          <label
            htmlFor="withdraw-amount"
            className="block text-[10px] text-text-muted mb-1.5 font-semibold uppercase tracking-[0.3px]"
          >
            You receive
          </label>
          <div className="relative mb-3">
            <input
              id="withdraw-amount"
              autoFocus
              required
              type="number"
              min={min}
              max={max}
              step={quote ? 10 ** -quote.precision : 'any'}
              placeholder="0.00"
              value={amount}
              onChange={event => setAmount(event.target.value)}
              className="w-full bg-[rgba(255,255,255,0.03)] border border-border-light rounded px-3 py-2.5 pr-[50px] text-text-main text-sm outline-none font-mono"
            />
            <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[11px] text-text-muted">USDT</span>
          </div>

          <div className="flex justify-between text-[11px] text-text-muted mb-1">
            <span>Total debited</span>
            <span className="text-text-main font-mono">{total} USDT</span>
          </div>
          {quote && amount.length > 0 && !precisionOk && (
            <div className="text-[11px] text-[#ffb700] pb-2">
              Use at most {quote.precision} decimal places.
            </div>
          )}

          {error && <div className="text-[#ff3b3b] text-[11px] py-2">{error}</div>}

          {!quote && !loadingQuote && (
            <button
              type="button"
              onClick={loadQuote}
              className="w-full mb-2 py-2 border border-border-light rounded text-text-main text-[11px] font-semibold cursor-pointer"
            >
              RETRY QUOTE
            </button>
          )}

          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="flex-1 py-2.5 px-4 bg-transparent border border-border-light rounded text-text-main text-[11px] font-semibold cursor-pointer"
            >
              CANCEL
            </button>
            <button
              type="submit"
              disabled={loading || invalid}
              className="flex-[2] flex items-center justify-center gap-1.5 py-2.5 px-4 border-none rounded bg-gradient-to-r from-neon-teal to-primary-dark text-black text-[11px] font-bold cursor-pointer transition-opacity hover:opacity-90 disabled:opacity-50"
            >
              {loading ? 'SIGNING...' : loadingQuote ? 'LOADING...' : 'SIGN & WITHDRAW'}
            </button>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  )
}
