import { Route, Routes } from "react-router-dom";
import Topbar from "./components/topbar";
import Trading from "./pages/tradingPage";
import { Toaster } from "react-hot-toast";
import CoinBot from "./pages/homePage";
import Dashboad from "./pages/dashboard";

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
        paddingTop: "75px" // Creates space so the Topbar doesn't cover your content
      }}>


        {/* Your Page Content Area */}
        <main style={{
          flex: 1,
          position: "relative",
          width: "100%",
          display: "flex",
          justifyContent: "center"
        }}>
          <Routes>
            {/* Add more routes here later */}
            <Route path={PATHS.HOME} element={<CoinBot />} />
            <Route path={PATHS.TRADE} element={<Trading />} />
            <Route path={PATHS.DASHBOARD} element={<Dashboad />} />
          </Routes>
        </main>

      </div>
    </>
  );
}

export default AppRoutes;
