import { Link, useParams } from 'react-router-dom'
import { GUIDE_ORDER, GUIDES } from '../content/howTo'

const UPDATED = 'November 10, 2026'

function GuideNotFound() {
  return (
    <div className="min-h-screen bg-bg-dark pt-14 text-text-main">
      <div className="mx-auto max-w-[860px] px-6 py-20 text-center">
        <div className="text-[11px] font-mono uppercase tracking-[0.2em] text-accent">Guides</div>
        <h1 className="m-0 mt-3 text-[32px] font-bold tracking-[-0.5px]">Guide not found</h1>
        <p className="mt-3 text-[13px] text-text-muted">That how-to page does not exist. Pick another guide below.</p>
        <div className="mt-8 flex flex-wrap justify-center gap-3">
          {GUIDE_ORDER.map(slug => (
            <Link key={slug} to={`/how-to/${slug}`} className="rounded-lg border border-border-light bg-bg-panel px-4 py-2 text-[12px] font-semibold text-text-main hover:border-accent hover:text-accent">
              {GUIDES[slug].title}
            </Link>
          ))}
        </div>
      </div>
    </div>
  )
}

export default function HowToPage() {
  const { slug } = useParams<{ slug: string }>()
  const guide = slug ? GUIDES[slug] : undefined
  if (!guide) return <GuideNotFound />

  const related = GUIDE_ORDER.filter(item => item !== guide.slug)

  return (
    <div className="min-h-screen bg-bg-dark pt-14 text-text-main">
      <div className="mx-auto max-w-[860px] px-6 py-10 sm:py-14">
        <nav className="mb-8 flex flex-wrap items-center gap-2 text-[11px] text-text-muted">
          <Link to="/" className="hover:text-accent">Home</Link>
          <span>/</span>
          <span className="text-text-main">Guides</span>
          <span>/</span>
          <span className="text-text-main">{guide.title}</span>
        </nav>

        <header className="border-b border-border-light pb-8">
          <div className="text-[11px] font-mono uppercase tracking-[0.2em] text-accent">{guide.eyebrow}</div>
          <h1 className="m-0 mt-3 text-[32px] font-bold leading-[1.15] tracking-[-0.8px] sm:text-[42px]">{guide.title}</h1>
          <p className="mt-4 max-w-[680px] text-[14px] leading-[1.75] text-text-muted">{guide.intro}</p>
          <div className="mt-5 flex flex-wrap items-center gap-3 text-[10px] font-mono uppercase tracking-[0.14em] text-text-muted">
            <span>Updated {UPDATED}</span>
            <span className="h-1 w-1 rounded-full bg-border-strong" />
            <span>About 4 min read</span>
          </div>
        </header>

        <section className="mt-10 rounded-xl border border-border-light bg-bg-panel p-6">
          <h2 className="m-0 text-[15px] font-semibold">What you need</h2>
          <ul className="mt-4 grid gap-2 sm:grid-cols-2">
            {guide.requirements.map(item => (
              <li key={item} className="flex items-start gap-2.5 text-[12px] leading-[1.65] text-text-muted">
                <svg className="mt-0.5 shrink-0 text-up" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                {item}
              </li>
            ))}
          </ul>
        </section>

        <section className="mt-12">
          <h2 className="m-0 text-[20px] font-bold tracking-[-0.3px]">Step by step</h2>
          <div className="mt-6 flex flex-col gap-6">
            {guide.steps.map((step, index) => (
              <div key={step.title} className="grid grid-cols-[36px_1fr] gap-4">
                <div className="flex h-9 w-9 items-center justify-center rounded-full border border-accent/30 bg-bg-panel text-[11px] font-bold font-mono text-accent">
                  {String(index + 1).padStart(2, '0')}
                </div>
                <div className="border-b border-border-light pb-6">
                  <h3 className="m-0 text-[15px] font-semibold">{step.title}</h3>
                  <p className="mt-2 max-w-[640px] text-[13px] leading-[1.75] text-text-muted">{step.body}</p>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="mt-12 rounded-xl border border-[color:var(--color-accent)]/20 bg-[linear-gradient(120deg,rgba(255,87,34,0.08),transparent)] p-6">
          <h2 className="m-0 text-[15px] font-semibold">Good to know</h2>
          <ul className="mt-4 flex flex-col gap-3">
            {guide.notes.map(note => (
              <li key={note} className="flex items-start gap-2.5 text-[12px] leading-[1.7] text-text-muted">
                <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-accent" />
                {note}
              </li>
            ))}
          </ul>
        </section>

        <section className="mt-12">
          <h2 className="m-0 text-[20px] font-bold tracking-[-0.3px]">FAQ</h2>
          <div className="mt-4 flex flex-col gap-2">
            {guide.faqs.map(faq => (
              <details key={faq.question} className="group rounded-lg border border-border-light bg-bg-panel px-5 py-4">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-[13px] font-semibold">
                  {faq.question}
                  <span className="text-text-muted transition-transform group-open:rotate-45">+</span>
                </summary>
                <p className="m-0 mt-3 text-[12px] leading-[1.7] text-text-muted">{faq.answer}</p>
              </details>
            ))}
          </div>
        </section>

        <section className="mt-12 border-t border-border-light pt-8">
          <h2 className="m-0 text-[13px] font-semibold uppercase tracking-[0.14em] text-text-muted">Related guides</h2>
          <div className="mt-4 grid gap-3 sm:grid-cols-3">
            {related.map(item => (
              <Link key={item} to={`/how-to/${item}`} className="rounded-lg border border-border-light bg-bg-panel px-4 py-3 text-[12px] font-semibold text-text-main transition-colors hover:border-accent hover:text-accent">
                {GUIDES[item].title}
              </Link>
            ))}
            <Link to="/policy" className="rounded-lg border border-border-light bg-bg-panel px-4 py-3 text-[12px] font-semibold text-text-main transition-colors hover:border-accent hover:text-accent">
              Platform policy
            </Link>
          </div>
        </section>

        <p className="mt-8 text-[11px] leading-[1.7] text-text-muted">
          Trading involves risk. Only commit capital you can afford to lose. See the{' '}
          <Link to="/policy" className="text-accent hover:underline">platform policy</Link> for fees, custody, and risk disclosures.
        </p>
      </div>
    </div>
  )
}
