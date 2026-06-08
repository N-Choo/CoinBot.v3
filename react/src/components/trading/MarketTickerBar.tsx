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
    <div className="ticker-bar">
      <div className="ticker-pair-selector" ref={selectorRef}>
        <button className="pair-select-btn" onClick={() => setShowDropdown(p => !p)}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="star-icon">
            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
          </svg>
          <span className="pair-select-text">{selectedPair}</span>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="chevron-icon">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        {showDropdown && (
          <div className="pair-dropdown">
            <div className="pair-dropdown-search">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="search-icon-sm">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input
                type="text"
                placeholder="Search coins..."
                value={query}
                onChange={e => setQuery(e.target.value)}
                autoFocus
              />
            </div>
            <div className="pair-dropdown-list">
              {filtered.length === 0 ? (
                <div className="pair-dropdown-empty">No results</div>
              ) : (
                filtered.map((coin, i) => (
                  <div
                    key={i}
                    className={`pair-dropdown-item ${selectedPair === coin.pair ? 'active' : ''}`}
                    onClick={() => select(coin.pair)}
                  >
                    <div className="pdd-left">
                      <span className="pdd-pair">{coin.pair}</span>
                    </div>
                    <div className="pdd-right">
                      <span className="pdd-price">${coin.price}</span>
                      <span className={`pdd-change ${coin.isUp ? 'up' : 'down'}`}>{coin.change}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      <div className="ticker-price-area">
        <span className="ticker-big-price">${active.price}</span>
        <span className={`ticker-big-change ${active.isUp ? 'up' : 'down'}`}>
          <svg viewBox="0 0 24 24" fill="currentColor" width="10" height="10">
            {active.isUp
              ? <polygon points="12 5 19 16 5 16 12 5" />
              : <polygon points="12 19 5 8 19 8 12 19" />
            }
          </svg>
          {active.change}
        </span>
      </div>

      <div className="ticker-stats">
        <div className="ticker-stat">
          <span className="ts-label">24h High</span>
          <span className="ts-value">${active.high || '—'}</span>
        </div>
        <div className="ticker-stat">
          <span className="ts-label">24h Low</span>
          <span className="ts-value">${active.low || '—'}</span>
        </div>
        <div className="ticker-stat">
          <span className="ts-label">24h Vol</span>
          <span className="ts-value">{active.volume || '—'}</span>
        </div>
      </div>
    </div>
  )
}
