import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import TradingForm from './TradingForm'
import type { BotSettings } from './types'

const defaultSettings: BotSettings = { Amount: '100', Expires: '5', TakeProfit: '50.0', StopLoss: '35.0' }

describe('TradingForm', () => {
  it('renders bot strategy header', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Bot Strategy')).toBeInTheDocument()
  })

  it('renders all setting inputs', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Allocation')).toBeInTheDocument()
    expect(screen.getByText('Max Trades/Day')).toBeInTheDocument()
    expect(screen.getByText('Take Profit')).toBeInTheDocument()
    expect(screen.getByText('Stop Loss')).toBeInTheDocument()
  })

  it('renders start bot button', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Start Bot')).toBeInTheDocument()
  })

  it('shows estimated profit and loss', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Est. Profit')).toBeInTheDocument()
    expect(screen.getByText('Est. Loss')).toBeInTheDocument()
  })

  it('shows selected pair in summary', () => {
    render(<TradingForm selectedPair="SOL/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('SOL/USDT')).toBeInTheDocument()
  })

  it('does not display NaN when inputs are empty', () => {
    const empty: BotSettings = { Amount: '', Expires: '', TakeProfit: '', StopLoss: '' }
    render(<TradingForm selectedPair="BTC/USDT" botSettings={empty} onSettingChange={() => {}} />)
    expect(screen.queryByText('NaN')).not.toBeInTheDocument()
  })
})
