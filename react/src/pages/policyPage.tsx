import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'

const UPDATED = 'November 10, 2026'

function Section({ id, title, children }: { id?: string; title: string; children: ReactNode }) {
  return (
    <section id={id} className="border-b border-border-light pb-8">
      <h2 className="m-0 text-[19px] font-bold tracking-[-0.3px]">{title}</h2>
      <div className="mt-3 flex flex-col gap-3 text-[13px] leading-[1.8] text-text-muted">{children}</div>
    </section>
  )
}

export default function PolicyPage() {
  return (
    <div className="min-h-screen bg-bg-dark pt-14 text-text-main">
      <div className="mx-auto max-w-[860px] px-6 py-10 sm:py-14">
        <nav className="mb-8 flex items-center gap-2 text-[11px] text-text-muted">
          <Link to="/" className="hover:text-accent">Home</Link>
          <span>/</span>
          <span className="text-text-main">Platform policy</span>
        </nav>

        <header className="border-b border-border-light pb-8">
          <div className="text-[11px] font-mono uppercase tracking-[0.2em] text-accent">Legal</div>
          <h1 className="m-0 mt-3 text-[32px] font-bold leading-[1.15] tracking-[-0.8px] sm:text-[42px]">Platform policy</h1>
          <p className="mt-4 max-w-[700px] text-[14px] leading-[1.75] text-text-muted">
            A plain-language summary of how CoinBot handles funds, fees, risk, and data.
            This is a product summary, not legal advice; have counsel review it before launch.
          </p>
          <div className="mt-5 text-[10px] font-mono uppercase tracking-[0.14em] text-text-muted">Updated {UPDATED}</div>
        </header>

        <div className="mt-10 flex flex-col gap-8">
          <Section id="risk" title="Risk">
            <p>
              Trading with margin can lose money quickly. Every contract publishes a maximum
              loss, and the engine sizes positions to the committed fund, but market gaps,
              exchange outages, and platform risks can still affect outcomes.
            </p>
            <p>Only commit capital you can afford to lose. Past performance does not guarantee future results.</p>
          </Section>

          <Section id="fees" title="Fees">
            <ul className="m-0 flex list-disc flex-col gap-2 pl-5">
              <li>The platform charges <strong className="text-text-main">5% of net profit</strong> when a contract ends positive, after trading and borrow costs.</li>
              <li>There is no management fee and no deposit fee.</li>
              <li>Deposits pay the Ethereum network gas fee from the user&apos;s wallet.</li>
              <li>Withdrawals pay a network fee shown before signing. The amount entered is what the user receives; the fee is charged on top.</li>
            </ul>
          </Section>

          <Section title="Custody">
            <p>
              Deposited USDT is held at KuCoin under platform API credentials. The platform
              is not a bank and deposits are not insured. Users authenticate with their own
              wallet and withdraw only to that wallet.
            </p>
            <p>
              Contract capital is allocated to the trading engine and may be used as margin.
              The platform maintains an internal ledger for every balance change.
            </p>
          </Section>

          <Section id="withdrawals" title="Withdrawals">
            <ul className="m-0 flex list-disc flex-col gap-2 pl-5">
              <li>Only available (unlocked) funds can be withdrawn.</li>
              <li>Each withdrawal is signed with the user&apos;s wallet and sent only to that wallet.</li>
              <li>Requests expire if not processed in time; expired or failed requests release the locked funds.</li>
              <li>Withdrawals may pause automatically if platform liabilities exceed free exchange funds.</li>
            </ul>
          </Section>

          <Section title="Eligibility and compliance">
            <p>
              Access may be restricted in some jurisdictions. The platform may require identity
              verification or refuse service where required by law. Users are responsible for
              complying with their local rules and tax obligations.
            </p>
          </Section>

          <Section title="Data and privacy">
            <p>
              The platform identifies users by wallet address. It stores wallet addresses,
              on-chain transaction hashes, contract and withdrawal records, and related
              activity. It does not store wallet private keys, and signatures are not logged.
            </p>
          </Section>

          <Section title="Changes">
            <p>
              Fees, limits, supported networks, and these terms may change. Material changes
              will be reflected on this page and in the product before they take effect.
            </p>
          </Section>

          <Section title="Contact">
            <p>
              For questions about a deposit, contract, or withdrawal, use the support channel
              published in the app. Include the relevant transaction hash or contract ID.
            </p>
          </Section>
        </div>

        <div className="mt-10 rounded-xl border border-[color:var(--color-accent)]/20 bg-[linear-gradient(120deg,rgba(255,87,34,0.08),transparent)] p-6">
          <p className="m-0 text-[12px] leading-[1.75] text-text-muted">
            <strong className="text-text-main">Risk disclosure:</strong> automated trading and
            margin can result in the loss of your committed capital. Review each contract&apos;s
            maximum loss, fees, and settlement rules before signing.
          </p>
        </div>

        <div className="mt-8 flex flex-wrap gap-3">
          <Link to="/how-to/deposit" className="rounded-lg border border-border-light bg-bg-panel px-4 py-2 text-[12px] font-semibold text-text-main hover:border-accent hover:text-accent">
            How to deposit
          </Link>
          <Link to="/how-to/withdraw" className="rounded-lg border border-border-light bg-bg-panel px-4 py-2 text-[12px] font-semibold text-text-main hover:border-accent hover:text-accent">
            How to withdraw
          </Link>
          <Link to="/" className="rounded-lg border border-border-light bg-bg-panel px-4 py-2 text-[12px] font-semibold text-text-main hover:border-accent hover:text-accent">
            Back to home
          </Link>
        </div>
      </div>
    </div>
  )
}
