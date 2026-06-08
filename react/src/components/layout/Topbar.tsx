import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTheme } from '../../hooks/useTheme'
import { useAuth } from '../../hooks/useAuth'
import '../../styles/topbar.css'

export default function Topbar() {
  const navigate = useNavigate()
  const [theme, toggleTheme] = useTheme()
  const { isAuthenticated, login, logout } = useAuth()
  const [menuOpen, setMenuOpen] = useState(false)

  const close = () => setMenuOpen(false)

  const navItems = [
    { name: 'Trading', action: () => { navigate('/trading'); close() } },
    { name: 'Dashboard', action: () => { navigate('/dashboard'); close() } },
    { name: 'Disconnect', action: () => { logout(); navigate('/'); close() }, cls: 'disconnect-btn' },
  ]

  return (
    <header className="modern-header">
      <div className="glass-container">
        <div className="brand-cluster" onClick={() => navigate('/')}>
          <span className="brand-title">CoinBot</span>
        </div>

        <nav className="nav-pills">
          {isAuthenticated ? (
            navItems.map(b => (
              <button key={b.name} className={`nav-link ${b.cls || ''}`} onClick={b.action}>
                {b.name}
              </button>
            ))
          ) : (
            <button className="connect-action-btn" onClick={login}>CONNECT</button>
          )}
          <button className="theme-toggle-btn nav-link nav-link-clickable" onClick={toggleTheme}>
            {theme === 'light' ? '\u{1F319} Dark' : '\u{2600}\u{FE0F} Light'}
          </button>
        </nav>

        <button className={`modern-toggle ${menuOpen ? 'is-active' : ''}`} onClick={() => setMenuOpen(!menuOpen)}>
          <span className="hamburger-line" />
          <span className="hamburger-line" />
          <span className="hamburger-line" />
        </button>
      </div>

      <div className={`mobile-menu-overlay ${menuOpen ? 'is-open' : ''}`}>
        <div className="mobile-nav-links">
          <button className="mobile-nav-link" onClick={() => { toggleTheme(); close() }}>
            {theme === 'light' ? '\u{1F319} Dark Mode' : '\u{2600}\u{FE0F} Light Mode'}
          </button>
          <hr className="mobile-divider" />
          {isAuthenticated ? (
            navItems.map(b => (
              <button key={b.name} className={`mobile-nav-link ${b.cls || ''}`} onClick={b.action}>
                {b.name}
              </button>
            ))
          ) : (
            <button className="connect-action-btn mobile-connect-btn" onClick={login}>CONNECT WALLET</button>
          )}
        </div>
      </div>
    </header>
  )
}
