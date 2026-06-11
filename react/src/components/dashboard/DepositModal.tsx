import { useState, useEffect } from 'react'
import axios from 'axios'
import { sendUSDT, getBalance } from '../../services/transaction'

interface DepositModalProps {
  onClose: () => void
  onDepositComplete?: () => void
}

export default function DepositModal({ onClose, onDepositComplete }: DepositModalProps) {
  const [amount, setAmount] = useState('')
  const [balance, setBalance] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [platformWallet, setPlatformWallet] = useState('')

  useEffect(() => {
    axios.get('/api/config').then(res => {
      setPlatformWallet(res.data.platform_wallet)
    }).catch(() => {})
    getBalance().then(setBalance).catch(() => setBalance('—'))
  }, [])

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      await sendUSDT(amount)
      onDepositComplete?.()
      onClose()
    } catch (err: unknown) {
      let msg = 'Transaction failed'
      if (err instanceof Error) msg = err.message
      if (err && typeof err === 'object' && 'response' in err) {
        const resp = (err as { response: { data: { error?: string } } }).response
        if (resp?.data?.error) msg = resp.data.error
      }
      setError(msg)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center z-[9999]" onClick={onClose}>
      <div onClick={e => e.stopPropagation()} className="w-full max-w-[420px] p-6 bg-bg-panel border border-border-light rounded-lg">
        <h2 className="m-0 mb-1.5 text-base text-text-main font-bold">Deposit USDT</h2>
        <p className="text-[11px] text-text-muted mb-4">Send USDT to your platform wallet.</p>

        <div className="bg-[rgba(255,255,255,0.02)] border border-border-light p-3 rounded mb-4 flex flex-col gap-1.5 text-[11px] text-text-muted">
          <div className="flex justify-between">
            <span>Platform Wallet</span>
            <span className="text-text-main font-mono text-[11px]" style={{ fontSize: 11 }}>{platformWallet || 'Loading...'}</span>
          </div>
          <div className="flex justify-between">
            <span>Your Balance</span>
            <span className="text-text-main font-semibold">{balance} USDT</span>
          </div>
        </div>

        <form onSubmit={submit}>
          <label className="block text-[10px] text-text-muted mb-1.5 font-semibold uppercase tracking-[0.3px]">Amount (USDT)</label>
          <div className="relative mb-4">
            <input
              autoFocus required type="number" placeholder="0.00"
              value={amount} onChange={e => setAmount(e.target.value)}
              className="w-full bg-[rgba(255,255,255,0.03)] border border-border-light rounded px-3 py-2.5 pr-[50px] text-text-main text-sm outline-none font-mono"
            />
            <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[11px] text-text-muted">USDT</span>
          </div>
          {error && <div className="text-down text-[11px] pb-3">{error}</div>}
          <div className="flex gap-2">
            <button type="button" onClick={onClose} className="flex-1 py-2.5 px-4 bg-transparent border border-border-light rounded text-text-main text-[11px] font-semibold cursor-pointer" disabled={loading}>
              CANCEL
            </button>
            <button type="submit" className="flex-[2] flex items-center justify-center gap-1.5 py-2.5 px-4 border-none rounded bg-gradient-to-r from-accent to-[#e64a19] text-white text-[11px] font-bold cursor-pointer transition-opacity hover:opacity-90" disabled={loading}>
              {loading ? 'PROCESSING...' : 'DEPOSIT'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
