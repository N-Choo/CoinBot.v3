import { describe, test, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useTheme } from './useTheme'

const store: Record<string, string> = {}
const localStorageMock = {
  getItem: vi.fn((key: string) => store[key] ?? null),
  setItem: vi.fn((key: string, value: string) => { store[key] = value }),
  removeItem: vi.fn((key: string) => { delete store[key] }),
  clear: vi.fn(() => { Object.keys(store).forEach(k => delete store[k]) }),
}

Object.defineProperty(window, 'localStorage', { value: localStorageMock })

Object.defineProperty(window, 'matchMedia', {
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
  writable: true,
})

beforeEach(() => {
  localStorageMock.clear()
  vi.clearAllMocks()
  document.documentElement.removeAttribute('data-theme')
})

describe('useTheme', () => {
  test('returns default theme "light"', () => {
    const { result } = renderHook(() => useTheme())
    expect(result.current[0]).toBe('light')
  })

  test('toggle switches theme from light to dark', () => {
    const { result } = renderHook(() => useTheme())
    expect(result.current[0]).toBe('light')

    act(() => { result.current[1]() })

    expect(result.current[0]).toBe('dark')
  })

  test('toggle switches theme from dark to light', () => {
    store.theme = 'dark'
    const { result } = renderHook(() => useTheme())
    expect(result.current[0]).toBe('dark')

    act(() => { result.current[1]() })

    expect(result.current[0]).toBe('light')
  })

  test('persists theme in localStorage', () => {
    const { result } = renderHook(() => useTheme())
    expect(localStorageMock.setItem).toHaveBeenCalledWith('theme', 'light')

    act(() => { result.current[1]() })

    expect(localStorageMock.setItem).toHaveBeenCalledWith('theme', 'dark')
  })

  test('sets data-theme attribute on dark theme', () => {
    const { result } = renderHook(() => useTheme())

    act(() => { result.current[1]() })

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  test('sets data-theme attribute on light theme', () => {
    store.theme = 'dark'
    const { result } = renderHook(() => useTheme())
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')

    act(() => { result.current[1]() })

    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })
})
