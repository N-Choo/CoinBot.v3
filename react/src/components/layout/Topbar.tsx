import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTheme } from '../../hooks/useTheme'
import { useAuth } from '../../hooks/useAuth'
import WalletAlert from '../ui/WalletAlert'

export default function Topbar() {
  const navigate = useNavigate()
  const [theme, toggleTheme] = useTheme()
  const { isAuthenticated, login, logout } = useAuth()
  const [menuOpen, setMenuOpen] = useState(false)
  const [showWalletAlert, setShowWalletAlert] = useState(false)

  const close = () => setMenuOpen(false)

  const handleConnect = async () => {
    if (!window.ethereum && !isAuthenticated) {
      setShowWalletAlert(true)
      return
    }
    await login()
  }

  const navItems = [
    { name: 'Trading', action: () => { navigate('/trading'); close() } },
    { name: 'Dashboard', action: () => { navigate('/dashboard'); close() } },
    { name: 'Disconnect', action: () => { logout(); navigate('/'); close() }, cls: 'disconnect-btn' },
  ]

  return (
    <>
      <header className="fixed top-0 left-0 right-0 h-14 sm:h-[56px] xs:h-12 flex items-center justify-center z-[1000] bg-bg-dark border-b border-border-light">
        <div className="w-full h-full flex items-center justify-between px-6 sm:px-8 xs:px-4">
          <div className="flex items-center gap-2.5 cursor-pointer shrink-0" onClick={() => navigate('/')}>
            <span className="text-text-main font-mono font-extrabold text-lg sm:text-lg xs:text-base tracking-[0.5px]">CoinBot</span>
          </div>

          <nav className="hidden md:flex items-center gap-0.5">
            {isAuthenticated ? (
              navItems.map(b => (
                <button
                  key={b.name}
                  className={`h-9 px-3.5 text-[14px] font-medium text-text-muted bg-transparent border-none rounded-lg cursor-pointer transition-all hover:text-text-main hover:bg-[rgba(255,255,255,0.04)] ${b.cls ? 'border border-border-light hover:bg-[rgba(255,59,59,0.1)] hover:text-down hover:border-[rgba(255,59,59,0.3)]' : ''}`}
                  onClick={b.action}
                >
                  {b.name}
                </button>
              ))
            ) : (
              <button
                className="h-9 min-w-[100px] px-5 text-[14px] font-semibold text-text-main bg-transparent border border-border rounded-lg cursor-pointer transition-all flex items-center justify-center hover:border-accent hover:bg-accent/10 hover:text-accent"
                onClick={handleConnect}
              >
                CONNECT
              </button>
            )}
            <button
              className="flex items-center gap-1.5 h-9 px-2.5 ml-2 text-[13px] font-medium text-text-muted bg-transparent border-none rounded-lg cursor-pointer transition-all hover:text-text-main hover:bg-[rgba(255,255,255,0.04)]"
              onClick={toggleTheme}
            >
              {theme === 'light' ? '\u{1F319} Dark' : '\u2600\u{FE0F} Light'}
            </button>
          </nav>

          <button
            className={`hidden-xs md:hidden bg-transparent border-none cursor-pointer flex flex-col justify-around w-[30px] h-6 p-0 z-20 ${menuOpen ? 'is-active' : ''}`}
            onClick={() => setMenuOpen(!menuOpen)}
          >
            <span className="hamburger-line" />
            <span className="hamburger-line" />
            <span className="hamburger-line" />
          </button>
        </div>

        <div
          className={`fixed top-14 sm:top-[56px] xs:top-12 left-0 right-0 bottom-0 z-[999] flex flex-col p-4 transition-all duration-300 bg-bg-dark ${menuOpen ? 'opacity-100 visible' : 'opacity-0 invisible'}`}
        >
          <div className="flex flex-col gap-1">
            <button
              className="h-12 px-4 text-base font-medium text-text-muted bg-transparent border-none rounded-lg text-left cursor-pointer transition-all hover:bg-[rgba(255,255,255,0.04)] hover:text-text-main"
              onClick={() => { toggleTheme(); close() }}
            >
              {theme === 'light' ? '\u{1F319} Dark Mode' : '\u2600\u{FE0F} Light Mode'}
            </button>
            <hr className="border-0 h-px bg-border-light my-2" />
            {isAuthenticated ? (
              navItems.map(b => (
                <button
                  key={b.name}
                  className={`h-12 px-4 text-base font-medium text-text-muted bg-transparent border-none rounded-lg text-left cursor-pointer transition-all hover:bg-[rgba(255,255,255,0.04)] hover:text-text-main ${b.cls ? 'hover:bg-[rgba(255,59,59,0.1)] hover:text-down' : ''}`}
                  onClick={b.action}
                >
                  {b.name}
                </button>
              ))
            ) : (
              <button
                className="w-full h-11 mt-4 text-[15px] font-semibold text-text-main bg-transparent border border-border rounded-lg cursor-pointer transition-all flex items-center justify-center hover:border-accent hover:bg-accent/10 hover:text-accent"
                onClick={handleConnect}
              >
                CONNECT WALLET
              </button>
            )}
          </div>
        </div>
      </header>
      {showWalletAlert && <WalletAlert onClose={() => setShowWalletAlert(false)} />}
    </>
  )
}
