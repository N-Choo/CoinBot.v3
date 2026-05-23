import { useEffect, useRef, useState } from 'react'

export function useScrollAnimation<T extends HTMLElement = HTMLDivElement>(options?: IntersectionObserverInit) {
  const ref = useRef<T>(null)
  const [isVisible, setIsVisible] = useState(
    () => typeof IntersectionObserver === 'undefined'
  )

  useEffect(() => {
    const el = ref.current
    if (!el) return

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true)
          observer.unobserve(el)
        }
      },
      { threshold: 0.15, ...options }
    )

    observer.observe(el)
    return () => observer.disconnect()
  }, [options])

  return { ref, isVisible }
}
