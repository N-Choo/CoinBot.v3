export default function FeaturesSection() {
  return (
    <section className="px-6 py-12 sm:py-16 max-w-[var(--max-width,1200px)] mx-auto w-full" id="features">
      <div className="text-[11px] tracking-[0.12em] text-accent uppercase font-semibold mb-3">How it works</div>
      <h2 className="text-[30px] sm:text-[22px] lg:text-[30px] font-bold tracking-[-0.5px] m-0 mb-2 leading-[1.2]">Three engines. One automated system.</h2>
      <p className="text-[14px] sm:text-[13px] lg:text-[14px] text-text-muted m-0 mb-8 lg:mb-10">Institutional-grade infrastructure, simplified for everyone.</p>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-4 lg:gap-6 max-w-[480px] sm:max-w-none mx-auto">
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">01</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Buy Low, Sell High</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Actively monitors the market to execute trades at optimal moments, capturing profit from price volatility.</p>
        </div>
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">02</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="3" y="11" width="18" height="11" rx="2" ry="2" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Auto-Stake</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Idle assets are instantly deployed to secure staking pools, earning passive APY between trades.</p>
        </div>
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">03</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10" /><path d="M12 6v6l4 2" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Zero Effort</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Connect once, configure your parameters, and let the bot navigate markets around the clock.</p>
        </div>
      </div>
    </section>
  )
}
