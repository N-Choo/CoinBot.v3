import { useState, useEffect, useRef } from 'react'
import type { TradingPair } from './types'

interface MarketTickerBarProps {
  pairs: TradingPair[]
  selectedPair: string
  onSelectPair: (pair: string) => void
}

export default function MarketTickerBar({ pairs, selectedPair, onSelectPair }: MarketTickerBarProps) {
  const [query, setQuery] = useState('')
  const [showDropdown, setShowDropdown] = useState(false)
  const selectorRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!showDropdown) return
    const handleClick = (e: MouseEvent) => {
      if (selectorRef.current && !selectorRef.current.contains(e.target as Node)) {
        setShowDropdown(false)
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [showDropdown])

  const select = (pair: string) => {
    onSelectPair(pair)
    setQuery('')
    setShowDropdown(false)
  }

  const filtered = query
    ? pairs.filter(p => p.pair.toLowerCase().includes(query.toLowerCase()))
    : pairs

  const active = pairs.find(p => p.pair === selectedPair) || pairs[0]
  if (!active) return null

  return (
    <div className="flex items-center h-11 px-2 sm:px-3 gap-0 bg-glass-bg-subtle border-b border-border-light flex-wrap sm:flex-nowrap">
      <div className="relative shrink-0" ref={selectorRef}>
        <button className="flex items-center gap-1.5 px-1.5 sm:px-2.5 py-1.5 border-none rounded bg-transparent text-text-main cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.04)]"
          onClick={() => setShowDropdown(p => !p)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-4 h-4 text-text-muted shrink-0 sm:inline hidden">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
          </svg>
          <span className="text-sm sm:text-[14px] font-bold whitespace-nowrap">{selectedPair}</span>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-3.5 h-3.5 text-text-muted shrink-0">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        {showDropdown && (
          <div className="absolute top-full left-0 mt-1 w-[calc(100vw-20px)] sm:w-[340px] bg-bg-panel border border-border rounded-lg z-[100] overflow-hidden">
            <div className="flex items-center gap-2 px-3 py-2.5 border-b border-border bg-bg-panel">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-3.5 h-3.5 text-text-muted shrink-0">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                type="text"
                placeholder="Search coins..."
                value={query}
                onChange={e => setQuery(e.target.value)}
                autoFocus
                className="w-full bg-bg-input border border-border rounded px-2 py-1.5 text-text-main text-xs outline-none placeholder:text-text-muted"
              />
            </div>
            <div className="max-h-80 overflow-y-auto p-1 bg-bg-panel" style={{ scrollbarWidth: 'thin' }}>
              {filtered.length === 0 ? (
                <div className="py-5 text-center text-xs text-text-muted">No results</div>
              ) : (
                filtered.map((coin, i) => (
                  <div
                    key={i}
                    className={`flex items-center justify-between px-2.5 py-2 rounded cursor-pointer transition-colors hover:bg-[rgba(255,255,255,0.04)] ${selectedPair === coin.pair ? 'bg-[rgba(0,212,170,0.08)]' : ''}`}
                    onClick={() => select(coin.pair)}
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-text-main">{coin.pair}</span>
                    </div>
                    <div className="flex items-center gap-2.5">
                      <span className="text-[11px] font-mono text-text-main">${coin.price}</span>
                      <span className={`text-[11px] font-semibold min-w-[44px] text-right ${coin.isUp ? 'text-up' : 'text-down'}`}>{coin.change}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      <div className="flex items-center gap-2 sm:gap-2.5 px-3 sm:px-4 shrink-0 h-7 border-r border-border-light">
        <span className="text-sm sm:text-base font-bold font-mono text-text-main">${active.price}</span>
        <span className={`flex items-center gap-1 text-xs sm:text-xs font-semibold ${active.isUp ? 'text-up' : 'text-down'}`}>
          <svg viewBox="0 0 24 24" fill="currentColor" width="10" height="10">
            {active.isUp
              ? <polygon points="12 5 19 16 5 16 12 5" />
              : <polygon points="12 19 5 8 19 8 12 19" />
            }
          </svg>
          {active.change}
        </span>
      </div>

      <div className="hidden sm:flex items-center gap-0 px-2 overflow-hidden">
        <div className="flex flex-col gap-0 px-2.5 min-w-0">
          <span className="text-[10px] uppercase tracking-[0.3px] text-text-muted font-semibold whitespace-nowrap">24h High</span>
          <span className="text-[11px] font-mono font-semibold text-text-main whitespace-nowrap">${active.high || '—'}</span>
        </div>
        <div className="flex flex-col gap-0 px-2.5 min-w-0">
          <span className="text-[10px] uppercase tracking-[0.3px] text-text-muted font-semibold whitespace-nowrap">24h Low</span>
          <span className="text-[11px] font-mono font-semibold text-text-main whitespace-nowrap">${active.low || '—'}</span>
        </div>
        <div className="flex flex-col gap-0 px-2.5 min-w-0">
          <span className="text-[10px] uppercase tracking-[0.3px] text-text-muted font-semibold whitespace-nowrap">24h Vol</span>
          <span className="text-[11px] font-mono font-semibold text-text-main whitespace-nowrap">{active.volume || '—'}</span>
        </div>
      </div>
    </div>
  )
}
