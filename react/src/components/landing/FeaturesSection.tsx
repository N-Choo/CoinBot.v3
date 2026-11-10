const FEATURES = [
  {
    n: '01',
    title: 'Wallet-Native Account',
    body: 'Your wallet is your identity. No passwords, no email, no middleman — sign once and your account is ready.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
        <circle cx="12" cy="7" r="4" />
      </svg>
    ),
  },
  {
    n: '02',
    title: 'Defined-Risk Contracts',
    body: 'Set the capital and the stop before signing. The engine sizes the trade to the contract and never risks more than the committed fund.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        <path d="M9 12l2 2 4-4" />
      </svg>
    ),
  },
  {
    n: '03',
    title: 'Automated Execution',
    body: 'Validated signals become sized orders on KuCoin cross-margin. In-flight and reconciliation guards prevent duplicate entries and exits.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
        <polyline points="17 6 23 6 23 12" />
      </svg>
    ),
  },
  {
    n: '04',
    title: 'Signal Engine',
    body: 'News, RSI, and supply/demand structure are combined into one auditable decision per cycle — including the reason for no trade.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M3 12h4l3-8 4 16 3-8h4" />
      </svg>
    ),
  },
  {
    n: '05',
    title: 'Transparent Ledger',
    body: 'Every balance change is journaled: deposits, contract locks, payouts, withdrawals, and fees. Your activity matches the books.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M4 4h16v16H4z" />
        <path d="M8 9h8M8 13h8M8 17h5" />
      </svg>
    ),
  },
  {
    n: '06',
    title: 'Withdraw Anytime',
    body: 'Sign a withdrawal to your own wallet. Available funds move from the margin account back to funding automatically; the fee is shown before you sign.',
    icon: (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M12 3v12" />
        <polyline points="7 10 12 15 17 10" />
        <path d="M5 21h14" />
      </svg>
    ),
  },
]

export default function FeaturesSection() {
  return (
    <section className="px-6 py-12 sm:py-16 max-w-[var(--max-width,1200px)] mx-auto w-full" id="features">
      <div className="text-[11px] tracking-[0.12em] text-accent uppercase font-semibold mb-3">Platform capabilities</div>
      <h2 className="text-[30px] sm:text-[22px] lg:text-[30px] font-bold tracking-[-0.5px] m-0 mb-2 leading-[1.2]">Six capabilities. One wallet.</h2>
      <p className="text-[14px] sm:text-[13px] lg:text-[14px] text-text-muted m-0 mb-8 lg:mb-10">
        From signal to settlement, every part of the trade is visible, bounded, and controlled by your wallet.
      </p>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-4 lg:gap-6 max-w-[480px] sm:max-w-none mx-auto">
        {FEATURES.map(feature => (
          <div key={feature.n} className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
            <div className="text-[11px] font-bold font-mono text-text-muted mb-3">{feature.n}</div>
            <div className="mb-3 text-text-main">{feature.icon}</div>
            <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">{feature.title}</h3>
            <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">{feature.body}</p>
          </div>
        ))}
      </div>
    </section>
  )
}
