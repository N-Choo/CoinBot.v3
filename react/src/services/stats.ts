import axios from 'axios'

export interface SystemStats {
  accounts: number
  active_contracts: number
  funds_deployed: string
  trades_executed: number
}

export async function getStats(): Promise<SystemStats> {
  const res = await axios.get('/api/stats')
  return res.data
}

export interface UserStats {
  total: string
  free: string
  locked: string
  realized_pnl: string
}

export async function getUserStats(): Promise<UserStats> {
  const res = await axios.get('/api/user/stats')
  return res.data
}
