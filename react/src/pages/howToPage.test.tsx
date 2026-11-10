import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { describe, expect, test } from 'vitest'
import HowToPage from './howToPage'
import PolicyPage from './policyPage'

function renderRoute(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/how-to/:slug" element={<HowToPage />} />
        <Route path="/policy" element={<PolicyPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('guides', () => {
  test('renders the deposit guide', () => {
    renderRoute('/how-to/deposit')
    expect(screen.getByRole('heading', { level: 1, name: 'How to deposit USDT' })).toBeInTheDocument()
    expect(screen.getByText('What you need')).toBeInTheDocument()
    expect(screen.getByText('Step by step')).toBeInTheDocument()
  })

  test('renders the withdraw guide', () => {
    renderRoute('/how-to/withdraw')
    expect(screen.getByRole('heading', { level: 1, name: 'How to withdraw USDT' })).toBeInTheDocument()
    expect(screen.getByText('FAQ')).toBeInTheDocument()
  })

  test('renders the policy page', () => {
    renderRoute('/policy')
    expect(screen.getByRole('heading', { level: 1, name: 'Platform policy' })).toBeInTheDocument()
    expect(screen.getByText(/risk disclosure/i)).toBeInTheDocument()
  })

  test('shows a not-found guide for an unknown slug', () => {
    renderRoute('/how-to/unknown')
    expect(screen.getByText('Guide not found')).toBeInTheDocument()
  })
})
