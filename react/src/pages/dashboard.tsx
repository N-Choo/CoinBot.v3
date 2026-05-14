import BalanceStats from '../components/dashboard/BalanceStats'
import PnlChart from '../components/dashboard/PnlChart'
import TradingContracts from '../components/dashboard/TradingContracts'
import SidebarActions from '../components/dashboard/SidebarActions'
import '../styles/dashboard.css'

export default function Dashboard() {
  return (
    <div className="dash-wrapper">
      <div className="dash-grid">
        <div className="dash-main-col">
          <BalanceStats />
          <PnlChart />
          <TradingContracts />
        </div>
        <SidebarActions />
      </div>
    </div>
  )
}
