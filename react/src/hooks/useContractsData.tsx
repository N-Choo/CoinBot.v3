/* eslint-disable react-refresh/only-export-components */

import { createContext, useContext, useState, useEffect, type ReactNode, type Dispatch, type SetStateAction } from 'react'
import { getMyContracts, getContractOrders, isClosedOrder, realizedPnlOf, type ContractItem, type ContractOrder } from '../services/contract'
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
  /** Realized close PnL plus live unrealized PnL for open entries. */
  contractPnl: (c: ContractItem) => number | null
  /** Live unrealized PnL across open entries. */
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

  // Poll while a cash-out is settling so `closing` becomes `cancelled`.
  const hasClosing = contracts.some(c => c.status === 'closing')
  useEffect(() => {
    if (!hasClosing) return
    const timer = setInterval(() => {
      getMyContracts().then(setContracts).catch(() => {})
    }, 5000)
    return () => clearInterval(timer)
  }, [hasClosing])

  // Stable key: the set of contract ids, used by the price sweep.
  const contractIds = contracts.map(c => c.id).join(',')
  // Orders are refetched when a contract's status changes (e.g. cash-out adds
  // close orders and ends in `cancelled`).
  const ordersKey = contracts.map(c => `${c.id}:${c.status}`).join(',')
  // Derived (not stored): true once the orders for the current key are in.
  const ordersReady = contracts.length === 0 || loadedKey === ordersKey

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
      setLoadedKey(ordersKey)
    })
    return () => { active = false }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ordersKey])

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

  const closedEntryIds = (orders: ContractOrder[]): Set<string> =>
    new Set(
      orders
        .filter(isClosedOrder)
        .map(o => o.parent_order_id)
        .filter((id): id is string => id != null),
    )

  /** Live PnL for an entry that has not been closed yet. */
  const entryUnrealized = (o: ContractOrder, ticker: string, closed: Set<string>): number | null => {
    if (o.action !== 'open' || closed.has(o.id)) return null
    if (o.status === 'cancelled' || o.status === 'failed') return null
    return pnlOf(o, ticker)
  }

  const contractPnl = (c: ContractItem): number | null => {
    const os = ordersByContract[c.id]
    if (!os || os.length === 0) return null
    const closed = closedEntryIds(os)
    let any = false
    let sum = 0
    for (const o of os) {
      const p = isClosedOrder(o) ? realizedPnlOf(o) : entryUnrealized(o, c.ticker, closed)
      if (p != null) { any = true; sum += p }
    }
    return any ? sum : null
  }

  // BalanceStats labels this "Unrealized PnL": closed legs carry realized PnL
  // and must not be counted here.
  const totalPnl = contracts.reduce((s, c) => {
    const os = ordersByContract[c.id]
    if (!os || os.length === 0) return s
    const closed = closedEntryIds(os)
    for (const o of os) {
      const p = entryUnrealized(o, c.ticker, closed)
      if (p != null) s += p
    }
    return s
  }, 0)

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
