import { useState, useEffect } from 'react'
import axios from 'axios'
import { sendUSDT, getBalance, getActivity, withdraw } from '../../services/transaction'
import type { ActivityItem } from '../../services/transaction'

function formatUSDT(raw: string): string {
  const num = Number(raw) / 1_000_000
  if (num === 0) return '0'
  return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 })
}

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [amount, setAmount] = useState('')
  const [balance, setBalance] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [activities, setActivities] = useState<ActivityItem[]>([])
  const [detail, setDetail] = useState<ActivityItem | null>(null)

  const [platformWallet, setPlatformWallet] = useState('')

  useEffect(() => {
    axios.get('/api/config').then(res => {
      setPlatformWallet(res.data.platform_wallet)
    }).catch(() => {})
  }, [])

  const close = () => { setModal(null); setAmount(''); setError('') }

  useEffect(() => {
    let cancelled = false
    getActivity().then(data => {
      if (!cancelled) setActivities(data)
    }).catch(() => {})
    return () => { cancelled = true }
  }, [])

  useEffect(() => {
    if (!modal) return
    getBalance().then(setBalance).catch(() => setBalance('—'))
  }, [modal])

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      if (modal === 'deposit') {
        await sendUSDT(amount)
        getActivity().then(setActivities).catch(() => {})
      } else {
        await withdraw(amount)
      }
      close()
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

  const user = {
    available: '$48,230.15',
    wallet: platformWallet || 'Loading...',
    dailyLimit: '$10,000.00',
  }

  return (
    <div className="dash-side-col">
      <div className="dash-panel">
        <div className="settings-title">
          <div className="status-dot" />
          <h3>Wallet Actions</h3>
        </div>
        <div className="wallet-actions">
          <button className="wallet-btn wallet-btn-deposit"
            onClick={() => setModal('deposit')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <polyline points="19 12 12 19 5 12" />
            </svg>
            DEPOSIT FUNDS
          </button>
          <button className="wallet-btn wallet-btn-withdraw"
            onClick={() => setModal('withdraw')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="19" x2="12" y2="5" />
              <polyline points="5 12 12 5 19 12" />
            </svg>
            WITHDRAW ASSETS
          </button>
        </div>
        <div className="wallet-available">
          Available: <strong>{user.available}</strong>
        </div>
      </div>

      <div className="dash-panel dash-scroll activity-section">
        <h3 className="activity-title">Recent Activity</h3>
        <div className="dash-title-sm activity-subtitle">Transaction History</div>
        <div className="activity-list">
          {activities.map(t => (
            <div key={t.id} className="tx-row" onClick={() => setDetail(t)} style={{ cursor: 'pointer' }}>
              <div className="tx-icon tx-icon-in">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" />
                </svg>
              </div>
              <div className="tx-body">
                <div className="tx-type">Deposit {t.ticker}</div>
                <div className="tx-hash">{t.tx_hash.slice(0, 10)}...</div>
              </div>
              <div className="tx-amount-col">
                <div className={`tx-amount ${t.status === 'confirmed' ? 'tx-amount-in' : 'tx-amount-out'}`}>
                  {formatUSDT(t.amount)}
                </div>
                <div className="tx-time">{new Date(t.created_at).toLocaleString()}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {detail && (
        <div className="modal-overlay" onClick={() => setDetail(null)}>
          <div onClick={e => e.stopPropagation()} className="tx-detail-modal">
            <div className="tx-detail-header">
              <div>
                <div className="tx-detail-label">Deposit</div>
                <div className="tx-detail-amount">USDT — {formatUSDT(detail.amount)}</div>
              </div>
              <span className={`tx-detail-status ${detail.status === 'confirmed' ? 'confirmed' : 'pending'}`}>
                {detail.status === 'confirmed' ? '✓ Confirmed' : '◷ Pending'}
              </span>
            </div>

            <div className="tx-detail-body">
              <div className="tx-detail-row">
                <span className="tx-detail-key">Transaction Hash</span>
                <span className="tx-detail-value mono break">{detail.tx_hash}</span>
              </div>
              <div className="tx-detail-row">
                <span className="tx-detail-key">Ticket ID</span>
                <span className="tx-detail-value">{detail.id}</span>
              </div>
              <div className="tx-detail-row">
                <span className="tx-detail-key">Created</span>
                <span className="tx-detail-value">{new Date(detail.created_at).toLocaleString()}</span>
              </div>
              <div className="tx-detail-row">
                <span className="tx-detail-key">Raw Amount</span>
                <span className="tx-detail-value mono">{detail.amount}</span>
              </div>
            </div>

            <button onClick={() => setDetail(null)} className="tx-detail-close">Close</button>
          </div>
        </div>
      )}

      {modal === 'deposit' && (
        <div className="modal-overlay" onClick={close}>
          <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel">
            <h2 className="modal-title">Deposit USDT</h2>
            <p className="modal-desc">Send USDT to your platform wallet.</p>

            <div className="modal-info-box">
              <div className="modal-info-row">
                <span>Platform Wallet</span>
                <span className="modal-info-value" style={{ fontSize: 11 }}>{user.wallet}</span>
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
                <button type="button" onClick={close} className="modal-cancel-btn" disabled={loading}>
                  CANCEL
                </button>
                <button type="submit" className="start-bot-btn modal-confirm-btn" disabled={loading}>
                  {loading ? 'PROCESSING...' : 'CONFIRM DEPOSIT'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {modal === 'withdraw' && (
        <div className="modal-overlay" onClick={close}>
          <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel">
            <h2 className="modal-title">Withdraw Assets</h2>
            <p className="modal-desc">Transfer funds to your external wallet.</p>

            <div className="modal-info-box">
              <div className="modal-info-row">
                <span>Available Balance</span>
                <span className="modal-info-value">{user.available}</span>
              </div>
              <div className="modal-info-row">
                <span>24h Limit Remaining</span>
                <span>{user.dailyLimit}</span>
              </div>
            </div>

            <form onSubmit={submit}>
              <label className="modal-form-label">Amount in USD</label>
              <div className="modal-input-wrap">
                <input
                  autoFocus required type="number" placeholder="0.00"
                  value={amount} onChange={e => setAmount(e.target.value)}
                  className="modal-input"
                />
                <span className="modal-currency-suffix">USD</span>
              </div>
              {error && <div className="modal-error">{error}</div>}
              <div className="modal-actions">
                <button type="button" onClick={close} className="modal-cancel-btn" disabled={loading}>
                  CANCEL
                </button>
                <button type="submit" className="start-bot-btn modal-confirm-btn" disabled={loading}>
                  {loading ? 'PROCESSING...' : 'CONFIRM WITHDRAWAL'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  )
}
