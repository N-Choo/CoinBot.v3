import { lazy, Suspense } from 'react'
import { Route, Routes } from 'react-router-dom'
import { Toaster } from 'react-hot-toast'
import { AuthProvider } from './hooks/useAuth'
import Topbar from './components/layout/Topbar'
import AuthGuard from './components/auth/AuthGuard'
import Landing from './pages/homePage'

const Trading = lazy(() => import('./pages/tradingPage'))
const Dashboard = lazy(() => import('./pages/dashboard'))

function PageLoader() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 'calc(100vh - 56px)', background: 'var(--bg-deep)' }}>
      <div className="page-loader" />
    </div>
  )
}

export const PATHS = {
  HOME: '/',
  TRADE: '/trading',
  DASHBOARD: '/dashboard',
} as const

export default function AppRoutes() {
  return (
    <AuthProvider>
      <Toaster
        position="top-center"
        toastOptions={{
          duration: 2000,
          style: {
            background: 'var(--bg-panel)',
            color: 'var(--text-main)',
            border: '1px solid var(--border-light)',
            borderRadius: '10px',
            fontSize: '13px',
          },
        }}
      />
      <Topbar />
      <Suspense fallback={<PageLoader />}>
        <Routes>
          <Route path={PATHS.HOME} element={<Landing />} />
          <Route path={PATHS.TRADE} element={<AuthGuard><Trading /></AuthGuard>} />
          <Route path={PATHS.DASHBOARD} element={<AuthGuard><Dashboard /></AuthGuard>} />
        </Routes>
      </Suspense>
    </AuthProvider>
  )
}
