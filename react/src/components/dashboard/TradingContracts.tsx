import { useState, useEffect } from 'react'
import Skeleton from '../ui/Skeleton'

const logoUrl = (symbol: string) =>
  `https://cdn.jsdelivr.net/gh/atomiclabs/cryptocurrency-icons@master/128/color/${symbol.toLowerCase()}.png`

const contracts = [
  { id: 1, pair: 'BTC/USD', status: 'Active', total: 15000, stake: 5000, trading: 8000, free: 2000, pnl: 1240, roe: 24.8 },
  { id: 2, pair: 'ETH/USD', status: 'Active', total: 8400, stake: 4000, trading: 3400, free: 1000, pnl: -320, roe: -8.0 },
  { id: 3, pair: 'SOL/USD', status: 'Inactive', total: 3200, stake: 3200, trading: 0, free: 0, pnl: 0, roe: 0 },
  { id: 4, pair: 'AVAX/USD', status: 'Active', total: 5600, stake: 2000, trading: 3000, free: 600, pnl: 480, roe: 24.0 },
]

export default function TradingContracts() {
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState('All')

  useEffect(() => {
    const t = setTimeout(() => setLoading(false), 500)
    return () => clearTimeout(t)
  }, [])

  const filtered = contracts.filter(c => filter === 'All' || c.status === filter)

  return (
    <div className="dash-panel">
      <div className="contracts-toolbar">
        <div className="settings-title">
          <div className="status-dot" />
          <h3>Trading Contracts</h3>
        </div>
        <div className="contracts-filters">
          {['All', 'Active', 'Inactive'].map(t => (
            <button
              key={t}
              className={`filter-btn ${filter === t ? 'active' : ''}`}
              onClick={() => setFilter(t)}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Pair</th>
                <th>Status</th>
                <th className="text-right">Size</th>
                <th className="text-right">P&L</th>
                <th className="text-right">ROE</th>
              </tr>
            </thead>
            <tbody>
              {[1, 2, 3, 4].map(i => (
                <tr key={i}>
                  <td><Skeleton variant="text" width="100px" /></td>
                  <td><Skeleton variant="text-sm" width="60px" /></td>
                  <td><Skeleton variant="text" width="70px" /></td>
                  <td><Skeleton variant="text" width="70px" /></td>
                  <td><Skeleton variant="text" width="50px" /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Pair</th>
                <th>Status</th>
                <th className="text-right">Size</th>
                <th className="text-right">P&L</th>
                <th className="text-right">ROE</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 ? (
                <tr><td colSpan={5} className="contracts-empty">No contracts found</td></tr>
              ) : (
                filtered.map(c => (
                  <tr key={c.id}>
                    <td>
                      <div className="contract-pair-cell">
                        <img
                          className="contract-pair-logo"
                          src={logoUrl(c.pair.split('/')[0])}
                          alt={c.pair.split('/')[0]}
                          onError={e => { (e.target as HTMLImageElement).style.display = 'none' }}
                        />
                        <span className="contract-pair-full">{c.pair}</span>
                      </div>
                    </td>
                    <td>
                      <span className={`contract-status ${c.status === 'Active' ? 'active' : 'inactive'}`}>
                        <span className="status-bullet" />
                        {c.status}
                      </span>
                    </td>
                    <td className="text-right">${c.total.toLocaleString()}</td>
                    <td className="text-right">
                      <span className={`contract-pnl ${c.pnl > 0 ? 'up' : c.pnl < 0 ? 'down' : ''}`}>
                        {c.pnl > 0 ? '+' : ''}${c.pnl.toLocaleString()}
                      </span>
                    </td>
                    <td className="text-right">
                      <span className={`contract-roe ${c.roe > 0 ? 'up' : c.roe < 0 ? 'down' : ''}`}>
                        {c.roe > 0 ? '+' : ''}{c.roe}%
                      </span>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
