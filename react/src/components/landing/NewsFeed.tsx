import { useState, useEffect } from 'react'
import { fetchNews, type NewsItem } from '../../services/news'

const timeAgo = (iso: string): string => {
  const t = new Date(iso).getTime()
  if (isNaN(t)) return ''
  const s = Math.max(1, Math.floor((Date.now() - t) / 1000))
  if (s < 60) return `${s}s ago`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

export default function NewsFeed() {
  const [news, setNews] = useState<NewsItem[]>([])

  useEffect(() => {
    let active = true
    const load = () => {
      fetchNews()
        .then(items => { if (active) setNews(items) })
        .catch(() => {})
    }
    load()
    const id = setInterval(load, 60_000)
    return () => { active = false; clearInterval(id) }
  }, [])

  const items = [...news, ...news]

  return (
    <div className="px-6 py-8 max-w-[var(--max-width,1200px)] mx-auto w-full">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.5px] text-text-muted mb-4">
        <span className="w-1.5 h-1.5 rounded-full bg-accent animate-pulse" data-testid="feed-dot" />
        Live News
        <span className="ml-auto text-[10px] normal-case tracking-normal text-text-muted">CoinTelegraph</span>
      </div>
      <div className="overflow-hidden">
        <div className="flex gap-6 w-max feed-track">
          {items.map((n, i) => (
            <a
              key={`${n.id}-${i}`}
              href={n.url}
              target="_blank"
              rel="noreferrer"
              data-testid="feed-item"
              className="inline-flex items-center gap-2 px-4 py-2 border border-border-light rounded-full text-xs font-semibold whitespace-nowrap bg-bg-panel transition-colors hover:border-accent"
            >
              <span className="uppercase px-1.5 py-0.5 rounded text-[10px] bg-[rgba(255,87,34,0.12)] text-accent">News</span>
              <span className="text-text-main">{n.title}</span>
              <span className="text-text-muted font-mono">{timeAgo(n.publishedAt)}</span>
            </a>
          ))}
        </div>
      </div>
    </div>
  )
}
