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
      <Toaster position="top-center" />
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
