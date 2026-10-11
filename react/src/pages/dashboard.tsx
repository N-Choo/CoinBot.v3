import BalanceStats from '../components/dashboard/BalanceStats'
import PnlChart from '../components/dashboard/PnlChart'
import ActiveContracts from '../components/trading/ActiveContracts'
import SidebarActions from '../components/dashboard/SidebarActions'

export default function Dashboard() {
  return (
    <div className="relative min-h-screen pt-14 flex bg-bg-dark">
      <div
        className="pointer-events-none absolute inset-0 z-0"
        style={{
          backgroundImage:
            'radial-gradient(700px 500px at 95% -10%, rgba(255,87,34,0.12), transparent 65%), radial-gradient(600px 400px at -5% 110%, rgba(255,87,34,0.07), transparent 60%)',
        }}
      />
      <div className="relative z-[1] w-full">
        <div className="max-w-[1200px] mx-auto p-3 sm:p-5 flex flex-col gap-3 sm:gap-4">
          <BalanceStats />
          <div className="grid grid-cols-1 lg:grid-cols-[1fr_340px] gap-3 sm:gap-4">
            <div className="flex flex-col gap-3 sm:gap-4 min-w-0">
              <div className="bg-bg-panel border border-border-light rounded-xl overflow-hidden backdrop-blur-xl transition-colors hover:border-accent/40">
                <PnlChart />
              </div>
              <ActiveContracts />
            </div>
            <div className="bg-bg-panel border border-border-light rounded-xl overflow-hidden backdrop-blur-xl transition-colors hover:border-accent/40 lg:max-h-[calc(100vh-140px)]">
              <SidebarActions />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
