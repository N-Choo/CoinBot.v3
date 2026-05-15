import { useState, useRef } from 'react'
import TradingContractCard from './TradingContractCard'

export default function TradingContracts() {
  const [filter, setFilter] = useState('All')
  const [viewMode, setViewMode] = useState<'cards' | 'list'>('cards')
  const scrollRef = useRef<HTMLDivElement>(null)

  const contracts = [
    { id: 1, pair: 'BTC/USD', status: 'Active', total: 15000, stake: 5000, trading: 8000, free: 2000 },
    { id: 2, pair: 'ETH/USD', status: 'Active', total: 8400, stake: 4000, trading: 3400, free: 1000 },
    { id: 3, pair: 'SOL/USD', status: 'Inactive', total: 3200, stake: 3200, trading: 0, free: 0 },
    { id: 4, pair: 'AVAX/USD', status: 'Active', total: 5600, stake: 2000, trading: 3000, free: 600 },
  ]

  const filtered = contracts.filter(c => filter === 'All' || c.status === filter)

  const scroll = (dir: 'left' | 'right') => {
    if (scrollRef.current) {
      const w = scrollRef.current.clientWidth
      scrollRef.current.scrollBy({ left: dir === 'left' ? -w : w, behavior: 'smooth' })
    }
  }

  return (
    <div className="dash-panel">
      <div className="contracts-toolbar">
        <div className="contracts-filters">
          <div className="settings-title">
            <div className="status-dot" />
            <h3>Trading Contracts</h3>
          </div>
          {['All', 'Active', 'Inactive'].map(t => (
            <button
              key={t}
              className={`filter-btn ${filter === t ? 'active' : ''}`}
              onClick={() => setFilter(t)}
            >
              {t}
            </button>
          ))}
        </div>
        <div className="contracts-actions">
          <div className="view-toggle">
            <button className={`vt-btn ${viewMode === 'cards' ? 'active' : ''}`}
              onClick={() => setViewMode('cards')}>Cards</button>
            <button className={`vt-btn ${viewMode === 'list' ? 'active' : ''}`}
              onClick={() => setViewMode('list')}>List</button>
          </div>
          {viewMode === 'cards' && (
            <div className="scroll-btns">
              <button className="scroll-btn" onClick={() => scroll('left')} aria-label="Scroll left">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="15 18 9 12 15 6" />
                </svg>
              </button>
              <button className="scroll-btn" onClick={() => scroll('right')} aria-label="Scroll right">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="9 18 15 12 9 6" />
                </svg>
              </button>
            </div>
          )}
        </div>
      </div>

      {viewMode === 'cards' ? (
        <div ref={scrollRef} className="cards-scroll-container">
          {filtered.map(c => (
            <div key={c.id} className="static-card-wrapper">
              <TradingContractCard data={c} />
            </div>
          ))}
          {filtered.length === 0 && (
            <div className="contracts-empty">No contracts found</div>
          )}
        </div>
      ) : (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Asset</th>
                <th>Status</th>
                <th className="text-right">Total</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map(c => (
                <tr key={c.id}>
                  <td className="table-cell-padded">{c.pair}</td>
                  <td className="table-cell-padded">
                    <span className={`badge ${c.status === 'Active' ? 'active' : 'inactive'}`}>{c.status}</span>
                  </td>
                  <td className="table-cell-right">${c.total.toLocaleString()}</td>
                </tr>
              ))}
              {filtered.length === 0 && (
                <tr><td colSpan={3} className="contracts-empty">No contracts found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
