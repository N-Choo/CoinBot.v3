import { render, screen } from '@testing-library/react'
import { describe, expect, test, vi } from 'vitest'

import NewsFeed from './NewsFeed'
import { fetchNews } from '../../services/news'

vi.mock('../../services/news', () => ({ fetchNews: vi.fn() }))
const mockNews = vi.mocked(fetchNews)

describe('NewsFeed', () => {
  test('renders the Live News header with headlines', async () => {
    mockNews.mockResolvedValue([
      { id: '1', title: 'Bitcoin hits a new high', url: 'https://example.com/1', source: 'CoinTelegraph', publishedAt: new Date().toISOString() },
      { id: '2', title: 'Ethereum upgrade goes live', url: 'https://example.com/2', source: 'CoinTelegraph', publishedAt: new Date().toISOString() },
    ])
    render(<NewsFeed />)
    expect(screen.getByText('Live News')).toBeInTheDocument()
    expect(screen.getByTestId('feed-dot')).toBeInTheDocument()
    expect((await screen.findAllByText('Bitcoin hits a new high')).length).toBeGreaterThan(0)
  })

  test('renders nothing in the track when there are no headlines', async () => {
    mockNews.mockResolvedValue([])
    render(<NewsFeed />)
    expect(screen.getByText('Live News')).toBeInTheDocument()
    expect(screen.queryAllByTestId('feed-item')).toHaveLength(0)
  })
})
