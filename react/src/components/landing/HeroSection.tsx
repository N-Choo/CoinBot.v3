import { useNavigate } from 'react-router-dom'
import { useScrollAnimation } from '../../hooks/useScrollAnimation'
import HeroDashboardPreview from './HeroDashboardPreview'

export default function HeroSection() {
  const navigate = useNavigate()
  const { ref: heroRef, isVisible: heroVisible } = useScrollAnimation()
  const { ref: visualRef, isVisible: visualVisible } = useScrollAnimation()

  const scrollToFeatures = () => {
    document.getElementById('features')?.scrollIntoView({ behavior: 'smooth' })
  }

  return (
    <section className="hero">
      <div className="hero-sparks">
        {Array.from({ length: 8 }).map((_, i) => <div key={i} className="spark" />)}
      </div>

      <div className={`hero-inner ${heroVisible ? 'revealed' : ''}`} ref={heroRef}>
        <div className="hero-body">
          <div className="hero-status">
            <span className="hero-status-dot" />
            Bot Online — $127K P&L Today
          </div>

          <h1>
            Trade smarter.<br />
            <span className="hero-em">Earn passively.</span>
          </h1>

          <p className="hero-sub">
            CoinBot scans the market 24/7 — buying low, selling high, and staking idle
            capital for passive yield. No keys, no manual trades, zero hands-on effort.
          </p>

          <div className="hero-actions">
            <button className="btn-p btn-lg" onClick={() => navigate('/trading')}>
              Launch Your Bot
            </button>
            <button className="btn-s btn-lg" onClick={scrollToFeatures}>
              View Strategies
            </button>
          </div>

          <div className="hero-trust">
            <span className="trust-item">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
              Audited
            </span>
            <span className="trust-item">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
              </svg>
              $450M+ Volume
            </span>
            <span className="trust-item">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
              10K+ Users
            </span>
          </div>
        </div>

        <div className={`hero-visual ${visualVisible ? 'revealed' : ''}`} ref={visualRef}>
          <HeroDashboardPreview />
        </div>
      </div>
    </section>
  )
}
