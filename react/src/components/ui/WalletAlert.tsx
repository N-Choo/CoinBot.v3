import { useRef } from 'react'

interface WalletAlertProps {
  onClose: () => void
}

export default function WalletAlert({ onClose }: WalletAlertProps) {
  const overlayRef = useRef<HTMLDivElement>(null)

  return (
    <div className="modal-overlay" ref={overlayRef} onClick={e => { if (e.target === overlayRef.current) onClose() }}>
      <div className="bg-bg-panel border border-border-light rounded-xl p-8 max-w-[400px] w-[calc(100vw-32px)] text-center">
        <div className="w-14 h-14 rounded-full flex items-center justify-center mx-auto mb-4 text-[#ffb700]" style={{ background: 'rgba(255,183,0,0.1)' }}>
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="2" y="6" width="20" height="14" rx="2" />
            <path d="M16 10a2 2 0 1 1 0 4" />
          </svg>
        </div>
        <h3 className="text-lg font-bold mb-2 text-text-main">No Wallet Detected</h3>
        <p className="text-sm text-text-muted leading-relaxed mb-6">
          You need a Web3 wallet like MetaMask to start trading.
          Please install one to continue.
        </p>
        <div className="flex flex-col gap-2">
          <a
            href="https://metamask.io/download/"
            target="_blank"
            rel="noopener noreferrer"
            className="block px-6 py-3 bg-gradient-to-r from-neon-teal to-primary-dark text-black border-none rounded-lg text-sm font-bold cursor-pointer no-underline text-center hover:opacity-90"
          >
            Install MetaMask
          </a>
          <button
            className="py-2.5 bg-transparent border border-border-light rounded-lg text-text-muted text-sm font-semibold cursor-pointer hover:text-text-main hover:border-text-muted transition-colors"
            onClick={onClose}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  )
}
