import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../../hooks/useAuth'
import LiveTicker from './LiveTicker'
import WalletAlert from '../ui/WalletAlert'

export default function HeroSection() {
  const navigate = useNavigate()
  const { isAuthenticated, login } = useAuth()
  const [showWalletAlert, setShowWalletAlert] = useState(false)

  const handleStart = async () => {
    if (!window.ethereum && !isAuthenticated) {
      setShowWalletAlert(true)
      return
    }
    if (isAuthenticated || await login()) {
      navigate('/trading')
    }
  }

  return (
    <section className="flex justify-center px-4 sm:px-6 pt-20 sm:pt-24 pb-12 sm:pb-16 relative min-h-[100vh] sm:min-h-screen">
      <div className="absolute -top-[200px] -left-[200px] w-[600px] h-[600px] pointer-events-none hidden sm:block"
        style={{ background: 'radial-gradient(circle, rgba(255,87,34,0.15) 0%, transparent 70%)', animation: 'heroGlow 4s ease-in-out infinite alternate' }} />
      <div className="absolute inset-0 opacity-[0.05] sm:opacity-[0.07] pointer-events-none z-0"
        style={{ backgroundImage: 'url("/f1.png")', backgroundSize: 'cover', backgroundPosition: 'center' }} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 lg:gap-16 max-w-[var(--max-width,1200px)] w-full items-center relative z-[1]">
        <div className="flex flex-col items-center lg:items-start gap-3 sm:gap-5">
          <div className="inline-flex items-center gap-2 text-xs font-bold text-accent border border-border-light px-4 py-1.5 rounded-full bg-accent/5 sm:text-[12px]">
            <span className="w-1.5 h-1.5 rounded-full bg-accent shadow-[0_0_8px_var(--color-accent)] animate-pulse" />
            Bot Online — $127K P&L Today
          </div>

          <h1 className="text-[34px] sm:text-[52px] font-extrabold leading-[1.06] tracking-[-1.5px] m-0 text-center lg:text-left text-balance">
            Your wealth.<br />
            <span className="bg-gradient-to-r from-accent to-[#ff8a65] bg-clip-text text-transparent">Automated.</span>
          </h1>

          <p className="text-[13px] sm:text-[15px] text-text-muted leading-[1.7] m-0 max-w-[480px] text-center lg:text-left">
            CoinBot scans the market 24/7 — buying low, selling high, and staking idle
            capital for passive yield. No keys, no manual trades, zero hands-on effort.
          </p>

          <div className="flex flex-wrap gap-3 mt-0 sm:mt-1 justify-center lg:justify-start w-full sm:w-auto">
            <button className="bg-gradient-to-r from-accent to-[#e64a19] text-white border-none px-5 sm:px-9 py-3 sm:py-3.5 text-[14px] sm:text-[15px] font-bold cursor-pointer rounded-lg transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_20px_var(--color-accent-glow,rgba(255,87,34,0.15))] w-full sm:w-auto" onClick={handleStart}>
              Start Trading
            </button>
            <button className="inline-flex items-center justify-center gap-2 bg-transparent text-text-main border border-border-light px-5 sm:px-6 py-3 sm:py-3.5 text-[13px] sm:text-[14px] font-semibold cursor-pointer rounded-lg transition-all hover:border-accent hover:text-accent w-full sm:w-auto" onClick={() => window.open('https://github.com/n-choo/CoinBot.v3', '_blank')}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/></svg>
              GitHub
            </button>
          </div>
        </div>

        <LiveTicker />
      </div>

      {showWalletAlert && <WalletAlert onClose={() => setShowWalletAlert(false)} />}
    </section>
  )
}
