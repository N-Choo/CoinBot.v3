/* eslint-disable react-refresh/only-export-components */

import { createContext, useContext, useState, useEffect, type ReactNode, type Dispatch, type SetStateAction } from 'react'
import { getMyContracts, getContractOrders, type ContractItem, type ContractOrder } from '../services/contract'
import { fetchTickerPrice } from '../services/kucoin'

interface ContractsData {
  contracts: ContractItem[]
  setContracts: Dispatch<SetStateAction<ContractItem[]>>
  loading: boolean
  error: boolean
  ordersByContract: Record<string, ContractOrder[]>
  ordersReady: boolean
  prices: Record<string, number>
  /** Live PnL for an order: filled → vs fill_price; open → vs funds/requested_qty. */
  pnlOf: (o: ContractOrder, ticker: string) => number | null
  /** Sum of a contract's order PnL. */
  contractPnl: (c: ContractItem) => number | null
  /** Sum of the PnL across all of the user's contracts. */
  totalPnl: number
}

const Ctx = createContext<ContractsData | null>(null)

export function ContractsProvider({ children }: { children: ReactNode }) {
  const [contracts, setContracts] = useState<ContractItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(false)
  const [ordersByContract, setOrdersByContract] = useState<Record<string, ContractOrder[]>>({})
  const [loadedKey, setLoadedKey] = useState<string | null>(null)
  const [prices, setPrices] = useState<Record<string, number>>({})

  // Load the user's contracts.
  useEffect(() => {
    let active = true
    getMyContracts()
      .then(data => { if (active) setContracts(data) })
      .catch(() => { if (active) setError(true) })
      .finally(() => { if (active) setLoading(false) })
    return () => { active = false }
  }, [])

  // Refetch when a contract is created elsewhere (e.g. after signing).
  useEffect(() => {
    const handler = () => {
      getMyContracts().then(setContracts).catch(() => {})
    }
    window.addEventListener('contracts:changed', handler)
    return () => window.removeEventListener('contracts:changed', handler)
  }, [])

  // Stable key: the set of contract ids. Status-only changes (e.g. closing a
  // contract) don't change it, so orders aren't refetched and the sweep isn't
  // restarted on those updates.
  const contractIds = contracts.map(c => c.id).join(',')
  // Derived (not stored): true once the orders for the current id set are in.
  const ordersReady = contracts.length === 0 || loadedKey === contractIds

  // Load the orders for every contract (needed to compute PnL).
  useEffect(() => {
    if (contracts.length === 0) return
    let active = true
    Promise.all(
      contracts.map(c =>
        getContractOrders(c.id)
          .then(o => [c.id, o] as const)
          .catch(() => [c.id, [] as ContractOrder[]] as const),
      ),
    ).then(entries => {
      if (!active) return
      setOrdersByContract(Object.fromEntries(entries))
      setLoadedKey(contractIds)
    })
    return () => { active = false }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [contractIds])

  // Sweep every distinct ticker's price once per second — sequentially, chained
  // so runs never overlap.
  useEffect(() => {
    const tickers = Array.from(new Set(contracts.map(c => c.ticker)))
    if (tickers.length === 0) return
    let active = true
    let timer: ReturnType<typeof setTimeout>
    const sweep = async () => {
      for (const t of tickers) {
        if (!active) return
        const p = await fetchTickerPrice(t.replace('/', '-'))
        if (!active) return
        if (p != null) setPrices(prev => ({ ...prev, [t]: p }))
      }
      if (active) timer = setTimeout(sweep, 1000)
    }
    sweep()
    return () => { active = false; clearTimeout(timer) }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [contractIds])

  const pnlOf = (o: ContractOrder, ticker: string): number | null => {
    const price = prices[ticker]
    const st = o.status.toLowerCase()
    if ((st !== 'open' && st !== 'filled') || price == null) return null
    const qty = st === 'filled'
      ? (parseFloat(o.filled_qty) || parseFloat(o.requested_qty) || 0)
      : (parseFloat(o.requested_qty) || 0)
    const entry = o.fill_price != null
      ? parseFloat(o.fill_price)
      : qty > 0 ? (parseFloat(o.funds) || 0) / qty : NaN
    if (!qty || Number.isNaN(entry)) return null
    const fee = parseFloat(o.fee) || 0
    const diff = o.side.toLowerCase() === 'buy' ? price - entry : entry - price
    return diff * qty - fee
  }

  const contractPnl = (c: ContractItem): number | null => {
    const os = ordersByContract[c.id]
    if (!os || os.length === 0) return null
    let any = false
    let sum = 0
    for (const o of os) {
      const p = pnlOf(o, c.ticker)
      if (p != null) { any = true; sum += p }
    }
    return any ? sum : null
  }

  const totalPnl = contracts.reduce((s, c) => s + (contractPnl(c) ?? 0), 0)

  const value: ContractsData = {
    contracts, setContracts, loading, error,
    ordersByContract, ordersReady, prices,
    pnlOf, contractPnl, totalPnl,
  }

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useContractsData(): ContractsData {
  const ctx = useContext(Ctx)
  if (!ctx) throw new Error('useContractsData must be used within a ContractsProvider')
  return ctx
}
