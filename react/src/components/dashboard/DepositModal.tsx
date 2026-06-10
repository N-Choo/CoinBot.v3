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
    <div className="modal-overlay" onClick={onClose}>
      <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel">
        <h2 className="modal-title">Deposit USDT</h2>
        <p className="modal-desc">Send USDT to your platform wallet.</p>

        <div className="modal-info-box">
          <div className="modal-info-row">
            <span>Platform Wallet</span>
            <span className="modal-info-value" style={{ fontSize: 11 }}>{platformWallet || 'Loading...'}</span>
          </div>
          <div className="modal-info-row">
            <span>Your Balance</span>
            <span className="modal-info-value">{balance} USDT</span>
          </div>
        </div>

        <form onSubmit={submit}>
          <label className="modal-form-label">Amount (USDT)</label>
          <div className="modal-input-wrap">
            <input
              autoFocus required type="number" placeholder="0.00"
              value={amount} onChange={e => setAmount(e.target.value)}
              className="modal-input"
            />
            <span className="modal-currency-suffix">USDT</span>
          </div>
          {error && <div className="modal-error">{error}</div>}
          <div className="modal-actions">
            <button type="button" onClick={onClose} className="modal-cancel-btn" disabled={loading}>
              CANCEL
            </button>
            <button type="submit" className="start-bot-btn modal-confirm-btn" disabled={loading}>
              {loading ? 'PROCESSING...' : 'DEPOSIT'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
