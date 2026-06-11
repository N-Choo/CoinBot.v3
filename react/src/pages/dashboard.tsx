import BalanceStats from '../components/dashboard/BalanceStats'
import PnlChart from '../components/dashboard/PnlChart'
import TradingContracts from '../components/dashboard/TradingContracts'
import SidebarActions from '../components/dashboard/SidebarActions'

export default function Dashboard() {
  return (
    <div className="min-h-screen pt-14 flex bg-bg-dark">
      <div className="flex flex-col w-full">
        <BalanceStats />
        <div className="flex flex-col lg:flex-row border-t border-border-light">
          <div className="flex-1 flex flex-col min-w-0 lg:border-r border-border-light">
            <PnlChart />
            <TradingContracts />
          </div>
          <div className="lg:w-[340px] shrink-0">
            <SidebarActions />
          </div>
        </div>
      </div>
    </div>
  )
}
