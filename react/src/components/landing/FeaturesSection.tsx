export default function FeaturesSection() {
  return (
    <section className="px-6 py-12 sm:py-16 max-w-[var(--max-width,1200px)] mx-auto w-full" id="features">
      <div className="text-[11px] tracking-[0.12em] text-accent uppercase font-semibold mb-3">How it works</div>
      <h2 className="text-[30px] sm:text-[22px] lg:text-[30px] font-bold tracking-[-0.5px] m-0 mb-2 leading-[1.2]">Three services. One wallet.</h2>
      <p className="text-[14px] sm:text-[13px] lg:text-[14px] text-text-muted m-0 mb-8 lg:mb-10">Your account, your trades, your funds — all built on your crypto wallet.</p>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-4 lg:gap-6 max-w-[480px] sm:max-w-none mx-auto">
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">01</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Wallet Accounts</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Every user gets their own account derived from their crypto wallet. No passwords, no middlemen — your wallet is your identity.</p>
        </div>
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">02</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18" /><polyline points="17 6 23 6 23 12" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Trading Service</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Buy trading contracts that put your capital to work — and withdraw your funds at any time. Nothing is locked up.</p>
        </div>
        <div className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
          <div className="text-[11px] font-bold font-mono text-text-muted mb-3">03</div>
          <div className="mb-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg>
          </div>
          <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">Automated Deposits</h3>
          <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">Deposits are handled automatically, giving you 100% control of your funds from the moment they land.</p>
        </div>
      </div>
    </section>
  )
}
