import { useState, useEffect, useCallback } from 'react'
import { createPortal } from 'react-dom'
import axios from 'axios'
import { getActivity, getWithdrawals } from '../../services/transaction'
import type { ActivityItem, WithdrawItem } from '../../services/transaction'
import { getUserStats } from '../../services/stats'
import DepositModal from './DepositModal'
import WithdrawModal from './WithdrawModal'

function formatUSDT(raw: string): string {
  const num = Number(raw)
  if (num === 0) return '0'
  return num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 6 })
}

async function loadActivity(): Promise<ActivityItem[]> {
  const [deposits, withdrawals] = await Promise.allSettled([
    getActivity(),
    getWithdrawals(),
  ])
  const depositItems = deposits.status === 'fulfilled' ? deposits.value : []
  const withdrawalItems = withdrawals.status === 'fulfilled' ? withdrawals.value.map(toActivity) : []

  return [...depositItems, ...withdrawalItems]
    .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
    .slice(0, 20)
}

function toActivity(withdrawal: WithdrawItem): ActivityItem {
  return {
    id: withdrawal.id,
    ticker: 'USDT',
    amount: withdrawal.amount,
    status: withdrawal.status,
    created_at: withdrawal.created_at,
    tx_hash: withdrawal.tx_hash ?? '',
    kind: 'withdrawal',
    failure_reason: withdrawal.failure_reason,
    fee: withdrawal.fee,
    total: withdrawal.total,
    completed_at: withdrawal.completed_at,
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'confirmed':
    case 'completed':
      return '✓ Confirmed'
    case 'submitted':
      return '◷ Sent'
    case 'processing':
      return '◷ Processing'
    case 'failed':
      return '✕ Failed'
    case 'expired':
      return '◷ Expired'
    case 'pending':
      return '◷ Pending'
    default:
      return status || 'Unknown'
  }
}

function statusClass(status: string): string {
  if (status === 'confirmed' || status === 'completed') return 'text-up'
  if (status === 'failed') return 'text-down'
  if (status === 'expired') return 'text-[#ffb700]'
  return 'text-text-main'
}

export default function SidebarActions() {
  const [modal, setModal] = useState<'deposit' | 'withdraw' | null>(null)
  const [activities, setActivities] = useState<ActivityItem[]>([])
  const [detail, setDetail] = useState<ActivityItem | null>(null)

  const [platformWallet, setPlatformWallet] = useState('')
  const [available, setAvailable] = useState('—')

  const refreshStats = useCallback(() => {
    getUserStats()
      .then(stats => setAvailable(
        `${Number(stats.free).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USDT`,
      ))
      .catch(() => setAvailable('—'))
  }, [])

  useEffect(() => {
    let cancelled = false

    const refreshActivity = () => {
      loadActivity().then(items => {
        if (!cancelled) setActivities(items)
      }).catch(() => {})
    }

    axios.get('/api/config').then(res => {
      setPlatformWallet(res.data.platform_wallet)
    }).catch(() => {})
    refreshStats()
    refreshActivity()

    const interval = window.setInterval(refreshActivity, 20_000)
    window.addEventListener('contracts:changed', refreshStats)
    window.addEventListener('contracts:changed', refreshActivity)
    window.addEventListener('focus', refreshActivity)

    return () => {
      cancelled = true
      window.clearInterval(interval)
      window.removeEventListener('contracts:changed', refreshStats)
      window.removeEventListener('contracts:changed', refreshActivity)
      window.removeEventListener('focus', refreshActivity)
    }
  }, [refreshStats])

  const close = () => setModal(null)

  const user = {
    available,
    wallet: platformWallet || 'Loading...',
    dailyLimit: '$10,000.00',
  }

  const canRetry = detail?.kind === 'withdrawal'
    && (detail.status === 'expired' || detail.status === 'failed')

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
          {activities.map(item => (
            <div key={`${item.kind}-${item.id}`} className="grid grid-cols-[auto_1fr_auto] items-center gap-2.5 px-2 py-2.5 rounded font-mono cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.02)]"
              onClick={() => setDetail(item)}>
              <div className="w-7 h-7 rounded-full flex items-center justify-center shrink-0" style={{ background: 'rgba(255,87,34,0.1)', color: '#ff5722' }}>
                {item.kind === 'withdrawal' ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="12" y1="19" x2="12" y2="5" /><polyline points="5 12 12 5 19 12" />
                  </svg>
                ) : (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="12" y1="5" x2="12" y2="19" /><polyline points="19 12 12 19 5 12" />
                  </svg>
                )}
              </div>
              <div className="min-w-0">
                <div className="text-xs font-semibold text-text-main">
                  {item.kind === 'withdrawal' ? 'Withdraw' : 'Deposit'} {item.ticker}
                </div>
                <div className="text-[10px] text-text-muted">
                  {item.kind === 'withdrawal' ? statusLabel(item.status) : `${item.tx_hash.slice(0, 10)}...`}
                </div>
              </div>
              <div className="text-right">
                <div className={`text-[11px] font-semibold ${statusClass(item.status)}`}>
                  {formatUSDT(item.amount)}
                </div>
                <div className="text-[10px] text-text-muted">{new Date(item.created_at).toLocaleString()}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {detail && createPortal(
        <div className="fixed inset-0 bg-[rgba(0,0,0,0.6)] flex items-center justify-center z-[9999]" onClick={() => setDetail(null)}>
          <div onClick={e => e.stopPropagation()} className="bg-bg-panel border border-border-light rounded-xl w-[440px] max-w-[calc(100vw-32px)] overflow-hidden">
            <div className="flex items-center justify-between px-6 py-5 border-b border-border-light">
              <div>
                <div className="text-[11px] text-text-muted font-semibold uppercase tracking-[0.3px] mb-1">
                  {detail.kind === 'withdrawal' ? 'Withdrawal' : 'Deposit'}
                </div>
                <div className="text-xl font-bold text-text-main">USDT — {formatUSDT(detail.amount)}</div>
              </div>
              <span className={`text-[11px] font-bold font-mono px-2.5 py-1 rounded ${statusClass(detail.status)}`}
                style={{ background: detail.status === 'failed' ? 'rgba(239,68,68,0.1)' : detail.status === 'expired' ? 'rgba(255,183,0,0.1)' : 'rgba(255,87,34,0.1)' }}>
                {statusLabel(detail.status)}
              </span>
            </div>

            <div className="px-6 py-4 flex flex-col gap-2.5">
              {detail.tx_hash && (
                <div className="flex justify-between items-start gap-3">
                  <span className="text-xs text-text-muted shrink-0">Transaction Hash</span>
                  <span className="text-xs text-text-main text-right font-medium font-mono break-all max-w-[260px]">{detail.tx_hash}</span>
                </div>
              )}
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">ID</span>
                <span className="text-xs text-text-main text-right font-medium break-all max-w-[260px]">{detail.id}</span>
              </div>
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Created</span>
                <span className="text-xs text-text-main text-right font-medium">{new Date(detail.created_at).toLocaleString()}</span>
              </div>
              <div className="flex justify-between items-start gap-3">
                <span className="text-xs text-text-muted shrink-0">Raw Amount</span>
                <span className="text-xs text-text-main text-right font-medium font-mono">{detail.amount}</span>
              </div>
              {detail.fee && (
                <div className="flex justify-between items-start gap-3">
                  <span className="text-xs text-text-muted shrink-0">Fee</span>
                  <span className="text-xs text-text-main text-right font-medium font-mono">{detail.fee}</span>
                </div>
              )}
              {detail.total && (
                <div className="flex justify-between items-start gap-3">
                  <span className="text-xs text-text-muted shrink-0">Total debited</span>
                  <span className="text-xs text-text-main text-right font-medium font-mono">{detail.total}</span>
                </div>
              )}
              {detail.completed_at && (
                <div className="flex justify-between items-start gap-3">
                  <span className="text-xs text-text-muted shrink-0">Completed</span>
                  <span className="text-xs text-text-main text-right font-medium">{new Date(detail.completed_at).toLocaleString()}</span>
                </div>
              )}
              {detail.failure_reason && (
                <div className="flex justify-between items-start gap-3">
                  <span className="text-xs text-text-muted shrink-0">Reason</span>
                  <span className="text-xs text-[#ff3b3b] text-right font-medium">{detail.failure_reason}</span>
                </div>
              )}
            </div>

            <div className="flex gap-2 mx-6 mb-5">
              {canRetry && (
                <button onClick={() => { setDetail(null); setModal('withdraw') }}
                  className="flex-1 py-2.5 border border-neon-teal rounded-lg bg-transparent text-neon-teal text-xs font-semibold cursor-pointer transition-colors hover:bg-neon-teal hover:text-black">
                  NEW WITHDRAWAL
                </button>
              )}
              <button onClick={() => setDetail(null)}
                className="flex-1 py-2.5 border border-border-light rounded-lg bg-transparent text-text-main text-xs font-semibold cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.03)] hover:border-border-strong">
                Close
              </button>
            </div>
          </div>
        </div>,
        document.body,
      )}

      {modal === 'deposit' && (
        <DepositModal
          onClose={close}
          onDepositComplete={() => { loadActivity().then(setActivities).catch(() => {}); refreshStats() }}
        />
      )}

      {modal === 'withdraw' && (
        <WithdrawModal
          onClose={close}
          onSubmitted={() => {
            window.dispatchEvent(new Event('contracts:changed'))
            loadActivity().then(setActivities).catch(() => {})
            refreshStats()
          }}
        />
      )}
    </div>
  )
}
