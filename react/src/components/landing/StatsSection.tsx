export default function StatsSection() {
  return (
    <section className="stats">
      <div className="stats-inner">
        <div className="stat">
          <span className="stat-num">$450M+</span>
          <span className="stat-label">Trading Volume</span>
        </div>
        <div className="stat-div" />
        <div className="stat">
          <span className="stat-num">99.9%</span>
          <span className="stat-label">Uptime</span>
        </div>
        <div className="stat-div" />
        <div className="stat">
          <span className="stat-num">0.0%</span>
          <span className="stat-label">Liquidation Risk</span>
        </div>
        <div className="stat-div" />
        <div className="stat">
          <span className="stat-num">10K+</span>
          <span className="stat-label">Active Users</span>
        </div>
      </div>
    </section>
  )
}
