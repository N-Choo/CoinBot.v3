export interface NewsItem {
  id: string
  title: string
  url: string
  source: string
  publishedAt: string
}

/** Decode the handful of entities that appear in RSS titles. */
const decode = (s: string) =>
  s
    .replace(/&amp;/g, '&')
    .replace(/&#39;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')

/**
 * Fetch the latest crypto headlines from the CoinTelegraph RSS feed
 * (proxied through `/news-api` to avoid CORS). Returns `[]` on any failure.
 */
export async function fetchNews(limit = 8): Promise<NewsItem[]> {
  try {
    const res = await fetch('/news-api/rss')
    if (!res.ok) return []
    const xml = await res.text()
    const doc = new DOMParser().parseFromString(xml, 'application/xml')
    return Array.from(doc.querySelectorAll('item'))
      .slice(0, limit)
      .map((it, i) => ({
        id: it.querySelector('guid')?.textContent || String(i),
        title: decode(it.querySelector('title')?.textContent || ''),
        url: it.querySelector('link')?.textContent || '',
        source: 'CoinTelegraph',
        publishedAt: it.querySelector('pubDate')?.textContent || '',
      }))
      .filter(n => n.title && n.url)
  } catch {
    return []
  }
}
