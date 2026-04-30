import { Route, Routes } from "react-router-dom";
import Topbar from "./components/topbar";
import Trading from "./pages/tradingPage";
import Background from "./components/bg";
import CoinBotLandingPage from "./pages/homePage";
import { Toaster } from "react-hot-toast";

export const PATHS = {
  HOME: "/",
  TRADE: "/trading",
  DASHBOARD: "/dashboard",
} as const;

function AppRoutes() {
  return (
    <Background>
      <Toaster position="top-center" />
      <Topbar />

      <div style={{
        display: "flex",
        flexDirection: "row",
        flex: 1,
        paddingTop: "75px" // Creates space so the Topbar doesn't cover your content
      }}>


        {/* Your Page Content Area */}
        <main style={{ flex: 1, position: "relative" }}>
          <Routes>
            {/* Add more routes here later */}
            <Route path={PATHS.HOME} element={<CoinBotLandingPage />} />
            <Route path={PATHS.TRADE} element={<Trading />} />
          </Routes>
        </main>

      </div>
    </Background>
  );
}

export default AppRoutes;
