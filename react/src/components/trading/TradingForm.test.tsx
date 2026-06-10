import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import TradingForm from './TradingForm'
import type { BotSettings } from './types'

const defaultSettings: BotSettings = { Ticker: 'BTC/USDT', Amount: '100', TakeProfit: '50.0', StopLoss: '35.0' }

describe('TradingForm', () => {
  it('renders trade contract header', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Trade Contract')).toBeInTheDocument()
  })

  it('renders all setting inputs', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Allocation')).toBeInTheDocument()
    expect(screen.getByText('Take Profit')).toBeInTheDocument()
    expect(screen.getByText('Stop Loss')).toBeInTheDocument()
  })

  it('renders sign contract button', () => {
    render(<TradingForm selectedPair="BTC/USDT" botSettings={defaultSettings} onSettingChange={() => {}} />)
    expect(screen.getByText('Sign Contract')).toBeInTheDocument()
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
    const empty: BotSettings = { Amount: '', TakeProfit: '', StopLoss: '' }
    render(<TradingForm selectedPair="BTC/USDT" botSettings={empty} onSettingChange={() => {}} />)
    expect(screen.queryByText('NaN')).not.toBeInTheDocument()
  })
})
