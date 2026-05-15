import type { BotSettings } from './types'

interface TradingSettingsProps {
  botSettings: BotSettings
  onSettingChange: (key: keyof BotSettings, value: string) => void
}

const settingsConfig = [
  { key: 'Amount' as const, label: 'Trade Amount', placeholder: '100', suffix: 'USDT' },
  { key: 'Expires' as const, label: 'Max Trades/Day', placeholder: '5' },
  { key: 'TakeProfit' as const, label: 'Take Profit', placeholder: '50.0', suffix: '%' },
  { key: 'StopLoss' as const, label: 'Stop Loss', placeholder: '35.0', suffix: '%' },
]

export default function TradingSettings({ botSettings, onSettingChange }: TradingSettingsProps) {
  return (
    <div className="bot-settings-integrated">
      <div className="settings-title">
        <div className="status-dot"></div>
        <h3>Trading Parameters</h3>
      </div>
      <div className="settings-grid">
        {settingsConfig.map((setting) => (
          <div key={setting.key} className="modern-input-group">
            <label>{setting.label}</label>
            <div className="input-wrapper">
              <input
                type="text"
                placeholder={setting.placeholder}
                value={botSettings[setting.key]}
                onChange={(e) => onSettingChange(setting.key, e.target.value)}
              />
              {setting.suffix && <span className="currency-badge">{setting.suffix}</span>}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
