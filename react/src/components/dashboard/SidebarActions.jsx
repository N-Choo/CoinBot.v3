import React, { useState } from 'react';

export default function SidebarActions() {
  const [activeModal, setActiveModal] = useState(null); // 'deposit' | 'withdraw' | null
  const [amount, setAmount] = useState('');

  const transactions = [
    { id: 1, type: 'Deposit', hash: '0x7a3B...f9E2', amount: '+$5,000', time: '2 min ago', isDeposit: true },
    { id: 2, type: 'Withdraw', hash: '0x4c1D...a3B7', amount: '-$1,200', time: '18 min ago', isDeposit: false },
    { id: 3, type: 'Deposit', hash: '0x9f2E...c4D8', amount: '+$8,500', time: '1h ago', isDeposit: true },
  ];

  const userStats = {
    available: "$48,230.15",
    wallet: "0x7a3B...f9E2",
    dailyLimit: "$10,000.00",
  };

  const closeModal = () => {
    setActiveModal(null);
    setAmount('');
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    const action = activeModal === 'deposit' ? 'Depositing' : 'Withdrawing';
    console.log(`${action} ${amount} USD...`);
    closeModal();
  };

  return (
    <div className="dash-side-col">
      {/* SECTION 1: FINANCIAL ACTIONS */}
      <div className="dash-panel">
        <div className="settings-title">
          <div className="status-dot"></div>
          <h3>Wallet Actions</h3>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', marginTop: '20px' }}>
          <button
            className="start-bot-btn"
            style={{ width: '100%', justifyContent: 'center' }}
            onClick={() => setActiveModal('deposit')}
          >
            DEPOSIT FUNDS
          </button>
          <button
            className="disconnect-btn"
            style={{ width: '100%', padding: '10px' }}
            onClick={() => setActiveModal('withdraw')}
          >
            WITHDRAW ASSETS
          </button>
        </div>
        <div style={{ marginTop: '15px', fontSize: '11px', color: 'var(--text-muted)', textAlign: 'center' }}>
          Available: <span style={{ color: 'var(--text-main)', fontWeight: 600 }}>{userStats.available}</span>
        </div>
      </div>

      {/* SECTION 2: TRANSACTION LOG */}
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

      {/* DYNAMIC MODAL (Handles both Deposit & Withdraw) */}
      {activeModal && (
        <div style={overlayStyle} onClick={closeModal}>
          <div style={modalStyle} onClick={(e) => e.stopPropagation()}>
            <h2 style={{ marginBottom: '10px' }}>
              {activeModal === 'deposit' ? 'Deposit Funds' : 'Withdraw Assets'}
            </h2>
            <p style={{ color: 'var(--text-muted)', fontSize: '12px', marginBottom: '20px' }}>
              {activeModal === 'deposit'
                ? 'Transfer USD from your connected bank or wallet.'
                : 'Transfer funds to your external wallet.'}
            </p>

            <div style={statsContainerStyle}>
              <div style={statRowStyle}>
                <span>{activeModal === 'deposit' ? 'Connected Wallet' : 'Available Balance'}</span>
                <span style={{ color: 'var(--color-primary)' }}>
                  {activeModal === 'deposit' ? userStats.wallet : userStats.available}
                </span>
              </div>
              {activeModal === 'withdraw' && (
                <div style={statRowStyle}>
                  <span>24h Limit Remaining</span>
                  <span>{userStats.dailyLimit}</span>
                </div>
              )}
            </div>

            <form onSubmit={handleSubmit}>
              <div style={{ marginBottom: '20px' }}>
                <label style={labelStyle}>Amount in USD</label>
                <div style={{ position: 'relative' }}>
                  <input
                    autoFocus
                    required
                    type="number"
                    placeholder="0.00"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    style={inputStyle}
                  />
                  <span style={inputCurrencyStyle}>USD</span>
                </div>
              </div>

              <div style={{ display: 'flex', gap: '10px' }}>
                <button
                  type="button"
                  style={{ ...buttonBase, background: '#222', color: '#fff' }}
                  onClick={closeModal}
                >
                  CANCEL
                </button>
                <button
                  type="submit"
                  className="start-bot-btn"
                  style={{ ...buttonBase, flex: 2 }}
                >
                  {activeModal === 'deposit' ? 'CONFIRM DEPOSIT' : 'CONFIRM WITHDRAWAL'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

// --- Styles (Reused from before) ---
const overlayStyle = { position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, backgroundColor: 'rgba(0, 0, 0, 0.85)', display: 'flex', justifyContent: 'center', alignItems: 'center', zIndex: 9999, backdropFilter: 'blur(5px)' };
const modalStyle = { backgroundColor: '#111', border: '1px solid #333', borderRadius: '12px', padding: '30px', width: '100%', maxWidth: '420px', boxShadow: '0 20px 40px rgba(0,0,0,0.4)' };
const statsContainerStyle = { backgroundColor: '#1a1a1a', padding: '15px', borderRadius: '8px', marginBottom: '20px', display: 'flex', flexDirection: 'column', gap: '8px', border: '1px solid #222' };
const statRowStyle = { display: 'flex', justifyContent: 'space-between', fontSize: '12px', color: '#888' };
const labelStyle = { display: 'block', fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)', marginBottom: '8px', fontWeight: 600 };
const inputStyle = { width: '100%', backgroundColor: '#000', border: '1px solid #444', borderRadius: '6px', padding: '12px 50px 12px 12px', color: '#fff', fontSize: '16px', outline: 'none', boxSizing: 'border-box' };
const inputCurrencyStyle = { position: 'absolute', right: '12px', top: '50%', transform: 'translateY(-50%)', fontSize: '12px', color: '#555', fontWeight: 'bold' };
const buttonBase = { padding: '12px', borderRadius: '6px', border: 'none', cursor: 'pointer', fontSize: '12px', fontWeight: 'bold', flex: 1 };
