export default function HeroSection() {
  return (
    <section className="hero">
      <div className="hero-sparks">
        {Array.from({ length: 8 }).map((_, i) => <div key={i} className="spark" />)}
      </div>
      <div className="hero-body">
        <h1>
          Trade smarter.<br />
          <span className="hero-em">Earn passively.</span>
        </h1>
        <p className="hero-sub">
          CoinBot scans the market 24/7 &mdash; buying low, selling high, and staking idle
          capital for passive yield. No keys, no manual trades, zero hands-on effort.
        </p>
        <div className="hero-actions">
          <button className="btn-p">Launch Your Bot</button>
          <button className="btn-s">View Strategies</button>
        </div>
      </div>
    </section>
  )
}
