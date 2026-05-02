// src/components/dashboard/TradingContracts.jsx
import React, { useState, useRef } from 'react';
import TradingContractCard from './TradingContractCard';

export default function TradingContracts() {
  const [filter, setFilter] = useState('All');
  const [viewMode, setViewMode] = useState('cards');
  const scrollContainerRef = useRef(null);

  const contracts = [
    { id: 1, pair: 'BTC/USD', status: 'Active', total: 15000, stake: 5000, trading: 8000, free: 2000 },
    { id: 2, pair: 'ETH/USD', status: 'Active', total: 8400, stake: 4000, trading: 3400, free: 1000 },
    { id: 3, pair: 'SOL/USD', status: 'Inactive', total: 3200, stake: 3200, trading: 0, free: 0 },
    { id: 4, pair: 'AVAX/USD', status: 'Active', total: 5600, stake: 2000, trading: 3000, free: 600 }
  ];

  const filteredContracts = contracts.filter(c => filter === 'All' || c.status === filter);

  const handleScroll = (dir) => {
    if (scrollContainerRef.current) {
      const amount = scrollContainerRef.current.clientWidth;
      scrollContainerRef.current.scrollBy({ left: dir === 'left' ? -amount : amount, behavior: 'smooth' });
    }
  };

  const btnStyle = (active) => ({
    background: active ? 'var(--bg-active)' : 'var(--bg-input)',
    border: `1px solid ${active ? 'var(--color-primary)' : 'var(--border-strong)'}`,
    color: active ? 'var(--color-primary)' : 'var(--text-main)',
    padding: '6px 14px',
    borderRadius: '6px',
    cursor: 'pointer',
    fontSize: '12px',
    fontWeight: 600,
    transition: 'all 0.2s ease'
  });

  return (
    <div className="dash-panel">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
        <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
          <div className="settings-title">
            <div className="status-dot"></div>
            <h3>Trading Contracts</h3>
          </div>

          {/* Fixed Filter Button Group spacing[cite: 1] */}
          <div style={{ display: 'flex', gap: '8px' }}>
            {['All', 'Active', 'Inactive'].map(type => (
              <button key={type} style={btnStyle(filter === type)} onClick={() => setFilter(type)}>
                {type}
              </button>
            ))}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '12px' }}>
          {/* View Toggle[cite: 1] */}
          <div style={{ display: 'flex', background: 'var(--bg-input)', borderRadius: '6px', padding: '2px' }}>
            <button
              style={{ ...btnStyle(viewMode === 'cards'), border: 'none' }}
              onClick={() => setViewMode('cards')}
            >Cards</button>
            <button
              style={{ ...btnStyle(viewMode === 'list'), border: 'none' }}
              onClick={() => setViewMode('list')}
            >List</button>
          </div>

          {viewMode === 'cards' && (
            <div style={{ display: 'flex', gap: '4px' }}>
              <button className="nav-link" onClick={() => handleScroll('left')}>←</button>
              <button className="nav-link" onClick={() => handleScroll('right')}>→</button>
            </div>
          )}
        </div>
      </div>

      {viewMode === 'cards' ? (
        <div ref={scrollContainerRef} className="cards-scroll-container">
          {filteredContracts.map(contract => (
            <div key={contract.id} className="static-card-wrapper">
              <TradingContractCard data={contract} />
            </div>
          ))}
        </div>
      ) : (
        /* Standard Excel List View[cite: 1] */
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', color: 'var(--text-main)' }}>
            <thead style={{ background: 'var(--bg-input)', fontSize: '11px', textTransform: 'uppercase' }}>
              <tr>
                <th style={{ padding: '12px', textAlign: 'left' }}>Asset</th>
                <th style={{ padding: '12px', textAlign: 'left' }}>Status</th>
                <th style={{ padding: '12px', textAlign: 'right' }}>Total</th>
              </tr>
            </thead>
            <tbody>
              {filteredContracts.map(c => (
                <tr key={c.id} style={{ borderBottom: '1px solid var(--border-light)' }}>
                  <td style={{ padding: '12px' }}>{c.pair}</td>
                  <td style={{ padding: '12px' }}><span className={`badge ${c.status === 'Active' ? 'active' : 'inactive'}`}>{c.status}</span></td>
                  <td style={{ padding: '12px', textAlign: 'right' }}>${c.total.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
