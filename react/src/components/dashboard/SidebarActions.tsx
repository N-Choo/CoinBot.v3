import { useState, useEffect } from 'react'
import axios from 'axios'
import { getActivity, withdraw } from '../../services/transaction'
import type { ActivityItem } from '../../services/transaction'
import DepositModal from './DepositModal'

function formatUSDT(raw: string): string {
  const num = Number(raw) / 1_000_000
  if (num === 0) return '0'
  return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 })
}

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [amount, setAmount] = useState('')
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

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      if (modal === 'withdraw') {
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
    <div className="flex flex-col h-full overflow-y-auto lg:sticky lg:top-14 self-start lg:max-h-[calc(100vh-56px)] border-t lg:border-t-0 border-border-light">
      <div className="p-3 sm:p-5 border-b border-border-light">
        <div className="flex items-center gap-2 mb-4">
          <span className="w-1.5 h-1.5 rounded-full bg-neon-teal shrink-0" />
          <h3 className="m-0 text-text-main text-sm font-semibold">Wallet Actions</h3>
        </div>
        <div className="flex lg:flex-col gap-2 mt-4">
          <button className="flex-1 lg:w-full flex items-center justify-center gap-1.5 px-3 py-2.5 lg:py-2.5 rounded text-xs font-bold font-mono tracking-wide cursor-pointer transition-colors border border-neon-teal text-neon-teal hover:bg-neon-teal hover:text-black"
            onClick={() => setModal('deposit')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <polyline points="19 12 12 19 5 12" />
            </svg>
            DEPOSIT FUNDS
          </button>
          <button className="flex-1 lg:w-full flex items-center justify-center gap-1.5 px-3 py-2.5 lg:py-2.5 rounded text-xs font-bold font-mono tracking-wide cursor-pointer transition-colors border border-border-light text-text-muted hover:border-danger hover:text-danger"
            onClick={() => setModal('withdraw')}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="19" x2="12" y2="5" />
              <polyline points="5 12 12 5 19 12" />
            </svg>
            WITHDRAW ASSETS
          </button>
        </div>
        <div className="mt-3 text-xs text-text-muted text-center">
          Available: <strong className="text-text-main font-semibold">{user.available}</strong>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto min-h-[200px] p-3 sm:p-5">
        <h3 className="m-0 mb-0.5 text-sm font-semibold text-text-main">Recent Activity</h3>
        <div className="text-xs text-text-muted mb-3">Transaction History</div>
        <div className="flex flex-col gap-0.5">
          {activities.map(t => (
            <div key={t.id} className="grid grid-cols-[auto_1fr_auto] items-center gap-2.5 px-2 py-2.5 rounded font-mono cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.02)]"
              onClick={() => setDetail(t)}>
              <div className="w-7 h-7 rounded-full flex items-center justify-center shrink-0" style={{ background: 'rgba(255,87,34,0.1)', color: '#ff5722' }}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" />
                </svg>
              </div>
              <div className="min-w-0">
                <div className="text-xs font-semibold text-text-main">Deposit {t.ticker}</div>
                <div className="text-[10px] text-text-muted">{t.tx_hash.slice(0, 10)}...</div>
              </div>
              <div className="text-right">
                <div className={`text-[11px] font-semibold ${t.status === 'confirmed' ? 'text-up' : 'text-text-main'}`}>
                  {formatUSDT(t.amount)}
                </div>
                <div className="text-[10px] text-text-muted">{new Date(t.created_at).toLocaleString()}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {detail && (
        <div className="fixed inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center z-[9999]" onClick={() => setDetail(null)}>
          <div onClick={e => e.stopPropagation()} className="bg-bg-panel border border-border-light rounded-xl w-[440px] max-w-[calc(100vw-32px)] overflow-hidden">
            <div className="flex items-center justify-between px-6 py-5 border-b border-border-light">
              <div>
                <div className="text-[11px] text-text-muted font-semibold uppercase tracking-[0.3px] mb-1">Deposit</div>
                <div className="text-xl font-bold text-text-main">USDT — {formatUSDT(detail.amount)}</div>
              </div>
              <span className={`text-[11px] font-bold font-mono px-2.5 py-1 rounded ${detail.status === 'confirmed' ? 'text-up' : 'text-[#ffb700]'}`}
                style={{ background: detail.status === 'confirmed' ? 'rgba(255,87,34,0.1)' : 'rgba(255,183,0,0.1)' }}>
                {detail.status === 'confirmed' ? '✓ Confirmed' : '◷ Pending'}
              </span>
            </div>

            <div className="px-6 py-4 flex flex-col gap-2.5">
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Transaction Hash</span>
                <span className="text-xs text-text-main text-right font-medium font-mono break-all max-w-[260px]">{detail.tx_hash}</span>
              </div>
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Ticket ID</span>
                <span className="text-xs text-text-main text-right font-medium">{detail.id}</span>
              </div>
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Created</span>
                <span className="text-xs text-text-main text-right font-medium">{new Date(detail.created_at).toLocaleString()}</span>
              </div>
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Raw Amount</span>
                <span className="text-xs text-text-main text-right font-medium font-mono">{detail.amount}</span>
              </div>
            </div>

            <button onClick={() => setDetail(null)}
              className="block w-[calc(100%-48px)] mx-6 mb-5 py-2.5 border border-border-light rounded-lg bg-transparent text-text-main text-xs font-semibold cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.03)] hover:border-border-strong">
              Close
            </button>
          </div>
        </div>
      )}

      {modal === 'deposit' && (
        <DepositModal
          onClose={() => { close(); getActivity().then(setActivities).catch(() => {}) }}
          onDepositComplete={() => getActivity().then(setActivities).catch(() => {})}
        />
      )}

      {modal === 'withdraw' && (
        <div className="fixed inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center z-[9999]" onClick={close}>
          <div onClick={e => e.stopPropagation()} className="w-full max-w-[420px] p-6 bg-bg-panel border border-border-light rounded-lg">
            <h2 className="m-0 mb-1.5 text-base text-text-main font-bold">Withdraw Assets</h2>
            <p className="text-[11px] text-text-muted mb-4">Transfer funds to your external wallet.</p>

            <div className="bg-[rgba(255,255,255,0.02)] border border-border-light p-3 rounded mb-4 flex flex-col gap-1.5 text-[11px] text-text-muted">
              <div className="flex justify-between">
                <span>Available Balance</span>
                <span className="text-neon-teal">{user.available}</span>
              </div>
              <div className="flex justify-between">
                <span>24h Limit Remaining</span>
                <span>{user.dailyLimit}</span>
              </div>
            </div>

            <form onSubmit={submit}>
              <label className="block text-[10px] text-text-muted mb-1.5 font-semibold uppercase tracking-[0.3px]">Amount in USD</label>
              <div className="relative mb-4">
                <input
                  autoFocus required type="number" placeholder="0.00"
                  value={amount} onChange={e => setAmount(e.target.value)}
                  className="w-full bg-[rgba(255,255,255,0.03)] border border-border-light rounded px-3 py-2.5 pr-[50px] text-text-main text-sm outline-none font-mono"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[11px] text-text-muted">USD</span>
              </div>
              {error && <div className="text-[#ff3b3b] text-[11px] pb-3">{error}</div>}
              <div className="flex gap-2">
                <button type="button" onClick={close}
                  className="flex-1 py-2.5 px-4 bg-transparent border border-border-light rounded text-text-main text-[11px] font-semibold cursor-pointer"
                  disabled={loading}>
                  CANCEL
                </button>
                <button type="submit"
                  className="flex-[2] flex items-center justify-center gap-1.5 py-2.5 px-4 border-none rounded bg-gradient-to-r from-neon-teal to-primary-dark text-black text-[11px] font-bold cursor-pointer transition-opacity hover:opacity-90"
                  disabled={loading}>
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
