import { Route, Routes } from 'react-router-dom'
import { Toaster } from 'react-hot-toast'
import Topbar from './components/topbar'
import AuthGuard from './components/auth_guard'
import Landing from './pages/homePage'
import Trading from './pages/tradingPage'
import Dashboard from './pages/dashboard'
import './App.css'

export const PATHS = {
  HOME: '/',
  TRADE: '/trading',
  DASHBOARD: '/dashboard',
} as const

export default function AppRoutes() {
  return (
    <>
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
      <div className="app-content">
        <Routes>
          <Route path={PATHS.HOME} element={<Landing />} />
          <Route path={PATHS.TRADE} element={<AuthGuard><Trading /></AuthGuard>} />
          <Route path={PATHS.DASHBOARD} element={<AuthGuard><Dashboard /></AuthGuard>} />
        </Routes>
      </div>
    </>
  )
}
