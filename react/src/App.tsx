import { Route, Routes } from "react-router-dom";
import Topbar from "./components/topbar";
import Trading from "./pages/tradingPage";
import { Toaster } from "react-hot-toast";
import CoinBot from "./pages/homePage";
import Dashboad from "./pages/dashboard";
import AuthGuard from "./components/auth_guard.jsx"; // Import the guard

export const PATHS = {
  HOME: "/",
  TRADE: "/trading",
  DASHBOARD: "/dashboard",
} as const;


function AppRoutes() {
  return (
    <>
      < Toaster
        containerStyle={{
          top: '50%',
          transform: 'translateY(-10%)',
        }}
        position="top-center"
        reverseOrder={false}
        toastOptions={{
          // Default options for all toasts
          duration: 1500,
          style: {
            background: '#333',
            color: '#fff',
            borderRadius: '10px',
            fontSize: '14px',
            padding: '16px',
          },

          // Specific styles for Success
          success: {
            duration: 1200,
            iconTheme: {
              primary: '#4ade80', // Tailwind-style green
              secondary: '#fff',
            },
          },

          // Specific styles for Error
          error: {
            iconTheme: {
              primary: '#ef4444', // Tailwind-style red
              secondary: '#fff',
            },
            style: {
              border: '1px solid #ef4444',
            },
          },
        }}
      />
      <Topbar />

      <div style={{
        display: "flex",
        flexDirection: "row",
        flex: 1,
        paddingTop: "75px"
      }}>

        <main style={{
          flex: 1,
          position: "relative",
          width: "100%",
          display: "flex",
          justifyContent: "center"
        }}>
          <Routes>
            {/* Public Route */}
            <Route path={PATHS.HOME} element={<CoinBot />} />

            {/* Protected Routes */}
            <Route
              path={PATHS.TRADE}
              element={
                <AuthGuard>
                  <Trading />
                </AuthGuard>
              }
            />
            <Route
              path={PATHS.DASHBOARD}
              element={
                <AuthGuard>
                  <Dashboad />
                </AuthGuard>
              }
            />
          </Routes>
        </main>
      </div>
    </>
  );
}

export default AppRoutes;
