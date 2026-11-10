const SAFEGUARDS = [
  {
    title: 'Max loss, fixed upfront',
    body: 'Every contract locks a fixed amount. The engine sizes the position to that amount and caps realized loss at the committed fund.',
    accent: 'text-accent',
  },
  {
    title: '5% on net profit only',
    body: 'No management fee, no spread. The platform earns 5% only when a contract ends positive, after trading and borrow costs.',
    accent: 'text-up',
  },
  {
    title: 'Withdraw to your wallet',
    body: 'Available funds can be withdrawn at any time. Funds move margin → funding automatically, and the network fee is shown before you sign.',
    accent: 'text-text-main',
  },
]

export default function TrustSection() {
  return (
    <section className="px-6 py-12 sm:py-16 max-w-[var(--max-width,1200px)] mx-auto w-full" id="safeguards">
      <div className="text-[11px] tracking-[0.12em] text-accent uppercase font-semibold mb-3">Built-in safeguards</div>
      <h2 className="text-[30px] sm:text-[22px] lg:text-[30px] font-bold tracking-[-0.5px] m-0 mb-2 leading-[1.2]">Risk controls, not promises.</h2>
      <p className="text-[14px] sm:text-[13px] lg:text-[14px] text-text-muted m-0 mb-8 lg:mb-10 max-w-[640px]">
        Capital protection is part of the product, not an afterthought.
      </p>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-4 lg:gap-6 max-w-[480px] sm:max-w-none mx-auto">
        {SAFEGUARDS.map((item, index) => (
          <div key={item.title} className="border border-border-light rounded-xl p-6 sm:p-4 lg:p-6 bg-bg-panel transition-all hover:border-accent hover:shadow-[0_0_20px_var(--color-accent-glow,rgba(255,87,34,0.15))]">
            <div className={`text-[11px] font-bold font-mono mb-3 ${item.accent}`}>0{index + 1}</div>
            <h3 className="text-[17px] sm:text-[16px] lg:text-[17px] font-semibold m-0 mb-2">{item.title}</h3>
            <p className="text-[13px] sm:text-[12px] lg:text-[13px] text-text-muted leading-[1.65] m-0">{item.body}</p>
          </div>
        ))}
      </div>

      <div className="mt-6 flex items-start gap-3 rounded-xl border border-border-light bg-bg-panel px-5 py-4">
        <svg className="shrink-0 text-accent" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        </svg>
        <p className="m-0 text-[12px] leading-[1.7] text-text-muted">
          Every balance change is written to an append-only ledger, so your activity and the platform's books stay in sync.
        </p>
      </div>
    </section>
  )
}
