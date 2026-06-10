import { useState } from 'react'
import toast from 'react-hot-toast'
import type { BotSettings } from './types'
import { signContract } from '../../services/contract'
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
  const [showDeposit, setShowDeposit] = useState(false)

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
      <div className="bot-strategy-panel">
        <div className="strategy-header">
          <span className="status-dot" />
          <span className="strategy-title">Trade Contract</span>
        </div>

        <div className="strategy-inputs">
          {settingsConfig.map(setting => (
            <div key={setting.key} className="trade-input-group">
              <label>{setting.label}</label>
              <div className="input-wrapper">
                <input
                  type="text"
                  placeholder={setting.placeholder}
                  value={botSettings[setting.key]}
                  onChange={e => onSettingChange(setting.key, e.target.value)}
                />
                {setting.suffix && <span className="currency-badge">{setting.suffix}</span>}
              </div>
            </div>
          ))}
        </div>

        <div className="strategy-summary">
          <div className="strategy-row">
            <span className="strategy-key">Pair</span>
            <span className="strategy-val">{selectedPair}</span>
          </div>
          <div className="strategy-row">
            <span className="strategy-key">Est. Profit</span>
            <span className="strategy-val profit">+{estProfit} USDT</span>
          </div>
          <div className="strategy-row">
            <span className="strategy-key">Est. Loss</span>
            <span className="strategy-val loss">-{estLoss} USDT</span>
          </div>
        </div>

        <button className="strategy-start-btn" onClick={handleStart}>
          <svg viewBox="0 0 24 24" fill="currentColor" width="14" height="14"><polygon points="5 3 19 12 5 21 5 3" /></svg>
          Sign Contract
        </button>
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
