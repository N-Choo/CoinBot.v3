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
        <div className="max-w-[1400px] mx-auto p-3 sm:p-5 flex flex-col gap-3 sm:gap-4">
          <div className="flex flex-wrap items-end justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-text-muted">
                Dashboard
                <span className="h-1 w-1 rounded-full bg-accent" aria-hidden="true" />
                Overview
              </div>
              <h1 className="mt-1.5 text-xl sm:text-2xl font-bold tracking-tight text-text-main">Portfolio</h1>
            </div>
            <div className="flex items-center gap-2 rounded-full border border-border-light bg-bg-panel px-3 py-1.5 font-mono text-[10px] text-text-muted">
              <span className="h-1.5 w-1.5 rounded-full bg-up" aria-hidden="true" />
              Live
            </div>
          </div>

          <BalanceStats />

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-[minmax(0,1fr)_340px] gap-3 sm:gap-4">
            <div className="min-w-0 md:col-span-2 lg:col-span-1 lg:col-start-1 lg:row-start-1 bg-bg-panel border border-border-light rounded-xl overflow-hidden backdrop-blur-xl transition-colors hover:border-accent/40">
              <PnlChart />
            </div>

            <div className="min-w-0 lg:col-start-2 lg:row-start-1 lg:row-span-2 bg-bg-panel border border-border-light rounded-xl overflow-hidden backdrop-blur-xl transition-colors hover:border-accent/40 lg:max-h-[calc(100vh-140px)]">
              <SidebarActions />
            </div>

            <div className="min-w-0 lg:col-start-1 lg:row-start-2">
              <ActiveContracts />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
