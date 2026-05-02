import React from 'react';

export default function SidebarActions() {
  const transactions = [
    { id: 1, type: 'Deposit', hash: '0x7a3B...f9E2', amount: '+$5,000', time: '2 min ago', isDeposit: true },
    { id: 2, type: 'Withdraw', hash: '0x4c1D...a3B7', amount: '-$1,200', time: '18 min ago', isDeposit: false },
    { id: 3, type: 'Deposit', hash: '0x9f2E...c4D8', amount: '+$8,500', time: '1h ago', isDeposit: true },
  ];

  return (
    <div className="dash-side-col">
      {/* SECTION 1: FINANCIAL ACTIONS */}
      <div className="dash-panel">
        <div className="settings-title">
          <div className="status-dot"></div>
          <h3>Wallet Actions</h3>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', marginTop: '20px' }}>
          <button className="start-bot-btn" style={{ width: '100%', justifyContent: 'center' }}>
            DEPOSIT FUNDS
          </button>
          <button className="disconnect-btn" style={{ width: '100%', padding: '10px' }}>
            WITHDRAW ASSETS
          </button>
        </div>
        <div style={{ marginTop: '15px', fontSize: '11px', color: 'var(--text-muted)', textAlign: 'center' }}>
          Available: <span style={{ color: 'var(--text-main)', fontWeight: 600 }}>$48,230.15</span>
        </div>
      </div>

      {/* SECTION 2: SPLIT TRANSACTION LOG */}
      <div className="dash-panel dash-scroll" style={{ flex: 1, overflowY: 'auto', minHeight: '300px' }}>
        <div className="settings-title" style={{ marginBottom: '4px' }}>
          <h3>Recent Activity</h3>
        </div>
        <span className="dash-title-sm" style={{ marginBottom: '20px', display: 'block' }}>History</span>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          {transactions.map(tx => (
            <div key={tx.id} className="tx-row" tabIndex="0">
              <div style={{ color: tx.isDeposit ? 'var(--color-primary)' : 'var(--color-danger)', fontSize: '16px', fontWeight: 'bold' }}>
                {tx.isDeposit ? '↓' : '↑'}
              </div>
              <div className="coin-info" style={{ flexDirection: 'column', alignItems: 'flex-start' }}>
                <span className="coin-name" style={{ fontSize: '13px' }}>{tx.type}</span>
                <span className="coin-base" style={{ fontSize: '10px' }}>{tx.hash}</span>
              </div>
              <div style={{ textAlign: 'right' }}>
                <div style={{ fontSize: '12px', fontWeight: 600, color: tx.isDeposit ? 'var(--color-primary)' : 'var(--text-main)' }}>
                  {tx.amount}
                </div>
                <div style={{ fontSize: '10px', color: 'var(--text-muted)' }}>{tx.time}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
