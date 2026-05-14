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
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12, marginTop: 20 }}>
          <button className="start-bot-btn" style={{ width: '100%', justifyContent: 'center' }}
            onClick={() => setModal('deposit')}>
            DEPOSIT FUNDS
          </button>
          <button className="disconnect-btn" style={{ width: '100%', padding: 10, textAlign: 'center' }}
            onClick={() => setModal('withdraw')}>
            WITHDRAW ASSETS
          </button>
        </div>
        <div style={{ marginTop: 15, fontSize: 11, color: 'var(--text-muted)', textAlign: 'center' }}>
          Available: <span style={{ color: 'var(--text-main)', fontWeight: 600 }}>{user.available}</span>
        </div>
      </div>

      <div className="dash-panel dash-scroll" style={{ flex: 1, overflowY: 'auto', minHeight: 300 }}>
        <h3 style={{ margin: '0 0 4px', fontSize: 14 }}>Recent Activity</h3>
        <div className="dash-title-sm" style={{ marginBottom: 20 }}>History</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {tx.map(t => (
            <div key={t.id} className="tx-row">
              <div style={{
                color: t.in ? 'var(--color-primary)' : 'var(--color-danger)',
                fontSize: 16, fontWeight: 'bold'
              }}>
                {t.in ? '\u2193' : '\u2191'}
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-main)' }}>{t.type}</div>
                <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>{t.hash}</div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <div style={{ fontSize: 12, fontWeight: 600, color: t.in ? 'var(--color-primary)' : 'var(--text-main)' }}>
                  {t.amount}
                </div>
                <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>{t.time}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {modal && (
        <div className="modal-overlay" onClick={close}
          style={{
            position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 9999
          }}>
          <div onClick={e => e.stopPropagation()} className="dash-panel"
            style={{ width: '100%', maxWidth: 420, padding: 30 }}>
            <h2 style={{ margin: '0 0 8px', fontSize: 18 }}>
              {modal === 'deposit' ? 'Deposit Funds' : 'Withdraw Assets'}
            </h2>
            <p style={{ color: 'var(--text-muted)', fontSize: 12, marginBottom: 20 }}>
              {modal === 'deposit'
                ? 'Transfer USD from your connected bank or wallet.'
                : 'Transfer funds to your external wallet.'}
            </p>

            <div style={{
              background: 'transparent', border: '1px solid #2a2e39', padding: 15, borderRadius: 4,
              marginBottom: 20, display: 'flex', flexDirection: 'column', gap: 8,
              fontSize: 12, color: 'var(--text-muted)'
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>{modal === 'deposit' ? 'Connected Wallet' : 'Available Balance'}</span>
                <span style={{ color: 'var(--color-primary)' }}>
                  {modal === 'deposit' ? user.wallet : user.available}
                </span>
              </div>
              {modal === 'withdraw' && (
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>24h Limit Remaining</span>
                  <span>{user.dailyLimit}</span>
                </div>
              )}
            </div>

            <form onSubmit={e => { e.preventDefault(); close() }}>
              <label style={{ display: 'block', fontSize: 11, color: 'var(--text-muted)', marginBottom: 8, fontWeight: 600 }}>
                Amount in USD
              </label>
              <div style={{ position: 'relative', marginBottom: 20 }}>
                <input
                  autoFocus required type="number" placeholder="0.00"
                  value={amount} onChange={e => setAmount(e.target.value)}
                  style={{
                    width: '100%', background: 'transparent', border: '1px solid #2a2e39',
                    borderRadius: 4, padding: '12px 50px 12px 14px', color: 'var(--text-main)',
                    fontSize: 16, outline: 'none'
                  }}
                />
                <span style={{ position: 'absolute', right: 14, top: '50%', transform: 'translateY(-50%)', fontSize: 12, color: 'var(--text-muted)' }}>
                  USD
                </span>
              </div>
              <div style={{ display: 'flex', gap: 10 }}>
                <button type="button" onClick={close}
                  style={{
                    flex: 1, padding: 12, background: 'transparent', border: '1px solid #2a2e39',
                    borderRadius: 4, color: 'var(--text-main)', fontWeight: 600, cursor: 'pointer'
                  }}>
                  CANCEL
                </button>
                <button type="submit" className="start-bot-btn" style={{ flex: 2, justifyContent: 'center' }}>
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
