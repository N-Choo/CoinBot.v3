import { useState } from 'react'

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [amount, setAmount] = useState('')

  const close = () => { setModal(null); setAmount('') }

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

      {modal && (
        <div className="modal-overlay" onClick={close}>
          <div onClick={e => e.stopPropagation()} className="dash-panel modal-panel">
            <h2 className="modal-title">
              {modal === 'deposit' ? 'Deposit Funds' : 'Withdraw Assets'}
            </h2>
            <p className="modal-desc">
              {modal === 'deposit'
                ? 'Transfer USD from your connected bank or wallet.'
                : 'Transfer funds to your external wallet.'}
            </p>

            <div className="modal-info-box">
              <div className="modal-info-row">
                <span>{modal === 'deposit' ? 'Connected Wallet' : 'Available Balance'}</span>
                <span className="modal-info-value">
                  {modal === 'deposit' ? user.wallet : user.available}
                </span>
              </div>
              {modal === 'withdraw' && (
                <div className="modal-info-row">
                  <span>24h Limit Remaining</span>
                  <span>{user.dailyLimit}</span>
                </div>
              )}
            </div>

            <form onSubmit={e => { e.preventDefault(); close() }}>
              <label className="modal-form-label">Amount in USD</label>
              <div className="modal-input-wrap">
                <input
                  autoFocus required type="number" placeholder="0.00"
                  value={amount} onChange={e => setAmount(e.target.value)}
                  className="modal-input"
                />
                <span className="modal-currency-suffix">USD</span>
              </div>
              <div className="modal-actions">
                <button type="button" onClick={close} className="modal-cancel-btn">
                  CANCEL
                </button>
                <button type="submit" className="start-bot-btn modal-confirm-btn">
                  {modal === 'deposit' ? 'CONFIRM DEPOSIT' : 'CONFIRM WITHDRAWAL'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  )
}
