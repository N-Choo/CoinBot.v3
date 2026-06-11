import { useState, useEffect } from 'react'
import toast from 'react-hot-toast'
import type { BotSettings } from './types'
import { signContract } from '../../services/contract'
import { getBalance } from '../../services/transaction'
import { useAuth } from '../../hooks/useAuth'
import DepositModal from '../dashboard/DepositModal'

interface TradingFormProps {
  selectedPair: string
  botSettings: BotSettings
  onSettingChange: (key: keyof BotSettings, value: string) => void
}

const settingsConfig = [
  { key: 'Amount' as const, label: 'Allocation', placeholder: '100', suffix: 'USDT' },
  { key: 'TakeProfit' as const, label: 'Take Profit', placeholder: '50.0', suffix: '%' },
  { key: 'StopLoss' as const, label: 'Stop Loss', placeholder: '35.0', suffix: '%' },
]

export default function TradingForm({ selectedPair, botSettings, onSettingChange }: TradingFormProps) {
  const { isAuthenticated } = useAuth()
  const [balance, setBalance] = useState(0)
  const [showDeposit, setShowDeposit] = useState(false)

  useEffect(() => {
    if (!isAuthenticated) return
    getBalance().then(v => setBalance(parseFloat(v))).catch(() => {})
  }, [isAuthenticated])

  const calcPnl = (amount: string, pct: string) => {
    const a = parseFloat(amount)
    const p = parseFloat(pct)
    return (isNaN(a) || isNaN(p)) ? '0.00' : (a * p / 100).toFixed(2)
  }
  const estProfit = calcPnl(botSettings.Amount, botSettings.TakeProfit)
  const estLoss = calcPnl(botSettings.Amount, botSettings.StopLoss)

  const handleStart = async () => {
    try {
      const res = await signContract(botSettings);
      if (res.success) {
        toast.success('Contract signed successfully!')
      } else if (res.insufficientFunds) {
        toast.error('Insufficient funds — please deposit USDT to continue')
        setShowDeposit(true)
      } else {
        toast.error('Failed to sign contract: ' + res.message)
      }
    } catch (err: unknown) {
      toast.error('Error signing contract: ' + (err instanceof Error ? err.message : String(err)))
    }
  }

  return (
    <>
      <div className="flex flex-col bg-bg-panel h-full max-h-[calc(100vh-180px)] sm:max-h-none">
        <div className="flex items-center gap-1.5 px-4 py-2.5 border-b border-border-light">
          <span className="w-2 h-2 rounded-full bg-[#00d46a] shrink-0" />
          <span className="text-[13px] font-semibold text-text-main">Trade Contract</span>
        </div>

        <div className="p-4 space-y-3 overflow-y-auto" style={{ scrollbarWidth: 'thin' }}>
          {settingsConfig.map(setting => (
            <div key={setting.key} className="flex flex-col gap-1">
              <label className="text-[11px] font-semibold uppercase tracking-[0.5px] text-text-muted">{setting.label}</label>
              <div className="relative flex items-center">
                <input
                  type="text"
                  placeholder={setting.placeholder}
                  value={botSettings[setting.key]}
                  onChange={e => onSettingChange(setting.key, e.target.value)}
                  className="w-full bg-bg-input border border-border rounded text-xs text-text-main px-3 py-2 outline-none placeholder:text-text-muted pr-11"
                />
                {setting.suffix && (
                  <span className="absolute right-1.5 text-[10px] font-semibold uppercase tracking-[0.3px] text-text-muted bg-bg-panel px-1.5 py-0.5 rounded">{setting.suffix}</span>
                )}
              </div>
            </div>
          ))}
        </div>

        <div className="px-4 py-3 border-t border-border-light">
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-[10px] text-text-muted font-semibold uppercase tracking-[0.3px]">Available</span>
            <span className={`text-[10px] font-mono font-bold ${isAuthenticated ? 'text-text-main' : 'text-text-muted'}`}>
              {isAuthenticated ? `${balance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USDT` : '—'}
            </span>
          </div>
          <div className="w-full h-1.5 rounded-full bg-bg-input overflow-hidden">
            <div className="h-full rounded-full transition-all duration-300"
              style={{
                width: `${Math.min((parseFloat(botSettings.Amount) || 0) / (balance || 1) * 100, 100)}%`,
                background: (parseFloat(botSettings.Amount) || 0) > balance
                  ? 'var(--color-down, #ef4444)'
                  : 'var(--color-accent, #ff5722)'
              }} />
          </div>
          {(parseFloat(botSettings.Amount) || 0) > balance && isAuthenticated && (
            <span className="text-[9px] text-down mt-1 block">Exceeds available balance</span>
          )}
        </div>

        <div className="px-4 py-2 border-t border-border-light space-y-1.5">
          <div className="flex items-center justify-between">
            <span className="text-[11px] text-text-muted">Pair</span>
            <span className="text-[11px] font-semibold text-text-main">{selectedPair}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[11px] text-text-muted">Est. Profit</span>
            <span className="text-[11px] font-semibold text-up">+{estProfit} USDT</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[11px] text-text-muted">Est. Loss</span>
            <span className="text-[11px] font-semibold text-down">-{estLoss} USDT</span>
          </div>
        </div>

        <div className="p-4 mt-auto">
          <button className="w-full flex items-center justify-center gap-2 py-2 px-4 border-none rounded bg-accent text-[12px] font-bold text-bg-dark cursor-pointer transition-all hover:brightness-110 active:brightness-95" onClick={handleStart}>
            <svg viewBox="0 0 24 24" fill="currentColor" width="12" height="12"><polygon points="5 3 19 12 5 21 5 3" /></svg>
            Sign Contract
          </button>
        </div>
      </div>

      {showDeposit && (
        <DepositModal
          onClose={() => setShowDeposit(false)}
          onDepositComplete={() => toast.success('Deposit submitted! Try signing the contract again.')}
        />
      )}
    </>
  )
}
