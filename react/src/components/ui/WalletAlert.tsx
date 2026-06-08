import { useRef } from 'react'

interface WalletAlertProps {
  onClose: () => void
}

export default function WalletAlert({ onClose }: WalletAlertProps) {
  const overlayRef = useRef<HTMLDivElement>(null)

  return (
    <div className="modal-overlay" ref={overlayRef} onClick={e => { if (e.target === overlayRef.current) onClose() }}>
      <div className="wallet-alert">
        <div className="wallet-alert-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="2" y="6" width="20" height="14" rx="2" />
            <path d="M16 10a2 2 0 1 1 0 4" />
          </svg>
        </div>
        <h3 className="wallet-alert-title">No Wallet Detected</h3>
        <p className="wallet-alert-desc">
          You need a Web3 wallet like MetaMask to start trading.
          Please install one to continue.
        </p>
        <div className="wallet-alert-actions">
          <a
            href="https://metamask.io/download/"
            target="_blank"
            rel="noopener noreferrer"
            className="wallet-alert-btn"
          >
            Install MetaMask
          </a>
          <button className="wallet-alert-cancel" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  )
}
