import { useState, useEffect } from 'react'
import axios from 'axios'
import { sendCoin, getBalance, COINS, type CoinConfig } from '../../services/transaction'
import type { CoinSymbol } from '../../services/transaction'

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [coin, setCoin] = useState<CoinSymbol>('usdt')
  const [amount, setAmount] = useState('')
  const [balance, setBalance] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const close = () => { setModal(null); setCoin('usdt'); setAmount(''); setError('') }

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
      } else {
        await axios.post(`/api/transactions/withdraw`, { amount })
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

  const tx = [
    { id: 1, type: 'Deposit', hash: '0x7a3B...f9E2', amount: '+$5,000', time: '2 min ago', in: true },
    { id: 2, type: 'Withdraw', hash: '0x4c1D...a3B7', amount: '-$1,200', time: '18 min ago', in: false },
    { id: 3, type: 'Deposit', hash: '0x9f2E...c4D8', amount: '+$8,500', time: '1h ago', in: true },
  ]

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
          {tx.map(t => (
            <div key={t.id} className="tx-row">
              <div className={`tx-icon ${t.in ? 'tx-icon-in' : 'tx-icon-out'}`}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  {t.in ? (
                    <><line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" /></>
                  ) : (
                    <><line x1="12" y1="19" x2="12" y2="5" /><polyline points="5 12 12 5 19 12" /></>
                  )}
                </svg>
              </div>
              <div className="tx-body">
                <div className="tx-type">{t.type}</div>
                <div className="tx-hash">{t.hash}</div>
              </div>
              <div className="tx-amount-col">
                <div className={`tx-amount ${t.in ? 'tx-amount-in' : 'tx-amount-out'}`}>
                  {t.amount}
                </div>
                <div className="tx-time">{t.time}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

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
