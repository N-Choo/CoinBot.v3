import { connectWallet, disconnectWallet } from '../services/connectWallet'
import { useState, useEffect, useCallback } from 'react'
import toast from 'react-hot-toast'
import axios from 'axios'

export const useAuth = () => {
  const [isLoading, setIsLoading] = useState(true)
  const [isAuthenticated, setIsAuthenticated] = useState(false)

  useEffect(() => {
    let cancelled = false
    const verify = async (retries = 2) => {
      try {
        const res = await axios.post('/api/user/verify', {}, { timeout: 3000 })
        if (!cancelled) setIsAuthenticated(res.status === 200)
      } catch (err: unknown) {
        if (cancelled) return
        if (axios.isAxiosError(err) && err.response?.status === 429 && retries > 0) {
          await new Promise(r => setTimeout(r, 2000))
          return verify(retries - 1)
        }
        setIsAuthenticated(false)
      } finally {
        if (!cancelled) setIsLoading(false)
      }
    }
    verify()
    return () => { cancelled = true }
  }, [])

  const login = useCallback(async (): Promise<boolean> => {
    setIsLoading(true)
    try {
      const success = await connectWallet()
      if (success) {
        setIsAuthenticated(true)
        toast.success('Wallet connected')
        return true
      }
    } catch (err: unknown) {
      const msg = err instanceof Error && err.message === 'NO_WALLET'
        ? 'No Web3 wallet detected'
        : 'Failed to connect wallet'
      toast.error(msg)
    } finally {
      setIsLoading(false)
    }
    return false
  }, [])

  const logout = useCallback(async () => {
    try {
      await disconnectWallet()
    } catch {
      // server might be offline, clear locally anyway
    }
    setIsAuthenticated(false)
    toast.success('Disconnected')
  }, [])

  return { isLoading, isAuthenticated, login, logout }
}
