import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTheme } from '../hooks/useTheme';
import { useAuth } from '../hooks/useAuth';
import '../styles/topbar.css';

const Topbar = () => {
  const navigate = useNavigate();
  const [theme, toggleTheme] = useTheme();
  const { isLoading, isAuthenticated, login, logout } = useAuth();
  const [menuOpen, setMenuOpen] = useState(false);

  const closeMenu = () => setMenuOpen(false);

  // Centralized Navigation Config
  const navItems = [
    { name: "Trading", action: () => { navigate('/trading'); closeMenu(); } },
    { name: "Dashboard", action: () => { navigate('/dashboard'); closeMenu(); } },
    { name: "Disconnect", action: () => { logout(); navigate('/'); closeMenu(); }, class: "disconnect-btn" }
  ];

  return (
    <header className="modern-header">
      <div className={`glass-container ${isLoading ? 'is-loading' : ''}`}>

        {/* Brand Section */}
        <div className="brand-cluster" onClick={() => navigate('/')}>
          <span className="brand-title">CoinBot</span>
        </div>

        {/* Desktop Nav */}
        <nav className="nav-pills">
          {isAuthenticated ? (
            navItems.map(btn => (
              <button key={btn.name} className={`nav-link ${btn.class || ''}`} onClick={btn.action}>
                {btn.name}
              </button>
            ))
          ) : (
            !isLoading && <button className="connect-action-btn" onClick={login}>CONNECT</button>
          )}
          <button className="theme-toggle-btn nav-link" onClick={toggleTheme}>
            {theme === 'light' ? '🌙 Dark' : '☀️ Light'}
          </button>
        </nav>

        {/* Mobile Toggle */}
        <button className={`modern-toggle ${menuOpen ? 'is-active' : ''}`} onClick={() => setMenuOpen(!menuOpen)}>
          <span className="hamburger-line"></span>
          <span className="hamburger-line"></span>
          <span className="hamburger-line"></span>
        </button>
      </div>

      {/* Mobile Menu Overlay */}
      <div className={`mobile-menu-overlay ${menuOpen ? 'is-open' : ''}`}>
        <div className="mobile-nav-links">
          <button className="mobile-nav-link" onClick={() => { toggleTheme(); closeMenu(); }}>
            {theme === 'light' ? '🌙 Dark Mode' : '☀️ Light Mode'}
          </button>
          <hr className="mobile-divider" />
          {isAuthenticated ? (
            navItems.map(btn => (
              <button key={btn.name} className={`mobile-nav-link ${btn.class || ''}`} onClick={btn.action}>
                {btn.name}
              </button>
            ))
          ) : (
            <button className="connect-action-btn mobile-connect-btn" onClick={login}>CONNECT WALLET</button>
          )}
        </div>
      </div>
    </header>
  );
};

export default Topbar;
