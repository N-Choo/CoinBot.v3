import { useState, useEffect } from 'react'
import { sendCoin, getBalance, getActivity, withdraw, COINS, type CoinConfig } from '../../services/transaction'
import type { CoinSymbol, ActivityItem } from '../../services/transaction'

const DECIMALS: Record<string, number> = { USDT: 6, USDC: 6, ETH: 18 }

function formatAmount(raw: string, ticker: string): string {
  const dec = DECIMALS[ticker.toUpperCase()] ?? 18
  const num = Number(raw) / 10 ** dec
  if (num === 0) return '0'
  return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: dec })
}

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [coin, setCoin] = useState<CoinSymbol>('usdt')
  const [amount, setAmount] = useState('')
  const [balance, setBalance] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [activities, setActivities] = useState<ActivityItem[]>([])
  const [detail, setDetail] = useState<ActivityItem | null>(null)

  const close = () => { setModal(null); setCoin('usdt'); setAmount(''); setError('') }

  useEffect(() => {
    let cancelled = false
    getActivity().then(data => {
      if (!cancelled) setActivities(data)
    }).catch(() => {})
    return () => { cancelled = true }
  }, [])

  useEffect(() => {
    if (!modal) return
    getBalance(coin).then(setBalance).catch(() => setBalance('—'))
  }, [modal, coin])

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      if (modal === 'deposit') {
        await sendCoin(amount, coin)
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
    wallet: '0x7a3B...f9E2',
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
                  {formatAmount(t.amount, t.ticker)}
                </div>
                <div className="tx-time">{new Date(t.created_at).toLocaleString()}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {detail && (
        <div className="modal-overlay" onClick={() => setDetail(null)}>
          <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel" style={{ minWidth: 440, padding: 0 }}>
            <div style={{ padding: '20px 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderBottom: 'var(--border-hud)' }}>
              <div>
                <div className="section-label" style={{ marginBottom: 2 }}>Deposit</div>
                <div style={{ fontSize: 20, fontWeight: 700 }}>{detail.ticker} — {formatAmount(detail.amount, detail.ticker)}</div>
              </div>
              <span className={`badge ${detail.status === 'confirmed' ? 'badge-success' : 'badge-danger'}`}>
                {detail.status === 'confirmed' ? '✓ Confirmed' : '◷ Pending'}
              </span>
            </div>

            <div className="modal-info-box" style={{ margin: '16px 20px' }}>
              <div className="modal-info-row">
                <span>Transaction Hash</span>
                <span className="modal-info-value" style={{ fontSize: 11, wordBreak: 'break-all', maxWidth: 260, textAlign: 'right' }}>{detail.tx_hash}</span>
              </div>
              <div className="modal-info-row">
                <span>Ticket ID</span>
                <span className="modal-info-value" style={{ fontSize: 11 }}>{detail.id}</span>
              </div>
              <div className="modal-info-row">
                <span>Created</span>
                <span className="modal-info-value">{new Date(detail.created_at).toLocaleString()}</span>
              </div>
              <div className="modal-info-row">
                <span>Raw Amount</span>
                <span className="modal-info-value" style={{ fontFamily: 'var(--font-mono)' }}>{detail.amount}</span>
              </div>
            </div>

            <div className="modal-actions" style={{ padding: '0 20px 20px' }}>
              <button onClick={() => setDetail(null)} className="modal-cancel-btn">CLOSE</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'deposit' && (
        <div className="modal-overlay" onClick={close}>
          <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel">
            <h2 className="modal-title">Deposit Funds</h2>
            <p className="modal-desc">Select a coin and enter the amount to deposit.</p>

            <div className="coin-selector">
              {(Object.values(COINS) as CoinConfig[]).map(c => (
                <button
                  key={c.symbol}
                  className={`coin-option${coin === c.symbol ? ' coin-option-active' : ''}`}
                  onClick={() => setCoin(c.symbol)}
                >
                  {c.label}
                </button>
              ))}
            </div>

            <div className="modal-info-box">
              <div className="modal-info-row">
                <span>Connected Wallet</span>
                <span className="modal-info-value">{user.wallet}</span>
              </div>
              <div className="modal-info-row">
                <span>Balance</span>
                <span className="modal-info-value">{balance} {COINS[coin].label}</span>
              </div>
            </div>

            <form onSubmit={submit}>
              <label className="modal-form-label">Amount</label>
              <div className="modal-input-wrap">
                <input
                  autoFocus required type="number" placeholder="0.00"
                  value={amount} onChange={e => setAmount(e.target.value)}
                  className="modal-input"
                />
                <span className="modal-currency-suffix">{COINS[coin].label}</span>
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
