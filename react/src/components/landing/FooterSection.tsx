import { Link } from 'react-router-dom'

const GUIDES = [
  { label: 'How to deposit', to: '/how-to/deposit' },
  { label: 'Buy a contract', to: '/how-to/buy-contract' },
  { label: 'Sell or close a contract', to: '/how-to/sell-contract' },
  { label: 'How to withdraw', to: '/how-to/withdraw' },
]

export default function FooterSection() {
  return (
    <footer className="border-t border-border-light px-4 py-10 text-xs text-text-muted sm:px-6">
      <div className="mx-auto grid max-w-[var(--max-width,1200px)] gap-8 sm:grid-cols-2 lg:grid-cols-[1.6fr_1fr_1fr]">
        <div>
          <div className="bg-gradient-to-r from-accent to-[#ff8a65] bg-clip-text font-bold font-mono text-transparent">CoinBot</div>
          <p className="mt-3 max-w-[320px] text-[12px] leading-[1.7]">
            Risk-defined automated crypto contracts. Connect your wallet, choose your risk,
            and withdraw available funds at any time.
          </p>
          <p className="mt-3 max-w-[320px] text-[11px] leading-[1.7]">
            Trading with margin involves risk. Only commit capital you can afford to lose.
          </p>
        </div>

        <div>
          <h3 className="m-0 text-[10px] font-semibold uppercase tracking-[0.16em] text-text-main">Guides</h3>
          <ul className="mt-3 flex list-none flex-col gap-2 p-0">
            {GUIDES.map(item => (
              <li key={item.to}>
                <Link to={item.to} className="text-[12px] text-text-muted transition-colors hover:text-accent">
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="m-0 text-[10px] font-semibold uppercase tracking-[0.16em] text-text-main">Legal</h3>
          <ul className="mt-3 flex list-none flex-col gap-2 p-0">
            <li><Link to="/policy" className="text-[12px] text-text-muted transition-colors hover:text-accent">Platform policy</Link></li>
            <li><Link to="/policy#risk" className="text-[12px] text-text-muted transition-colors hover:text-accent">Risk disclosure</Link></li>
            <li><Link to="/policy#fees" className="text-[12px] text-text-muted transition-colors hover:text-accent">Fees</Link></li>
            <li><Link to="/policy#withdrawals" className="text-[12px] text-text-muted transition-colors hover:text-accent">Withdrawals</Link></li>
          </ul>
        </div>
      </div>

      <div className="mx-auto mt-8 max-w-[var(--max-width,1200px)] border-t border-border-light pt-5">
        <span className="text-[11px]">&copy; {new Date().getFullYear()} CoinBot &mdash; All rights reserved.</span>
      </div>
    </footer>
  )
}
