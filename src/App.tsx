import { useState } from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { ProxyProvider } from "./contexts/ProxyContext";
import Layout from "./components/Layout";
import WelcomeScreen, { hasBeenWelcomed } from "./components/WelcomeScreen";
import DashboardPage from "./pages/DashboardPage";
import SettingsPage from "./pages/SettingsPage";
import TorPage from "./pages/TorPage";
import ProxyDetailPage from "./pages/ProxyDetailPage";
import CreateProxyPage from "./pages/CreateProxyPage";
import ProxyPage from "./pages/ProxyPage";
import ProxyListsPage from "./pages/ProxyListsPage";
import LeakTestPage from "./pages/LeakTestPage";
import ProxyCheckerPage from "./pages/ProxyCheckerPage";
import PluginsPage from "./pages/PluginsPage";
import SplitTunnelPage from "./pages/SplitTunnelPage";
import BandwidthPage from "./pages/BandwidthPage";
import SchedulePage from "./pages/SchedulePage";

function App() {
  const [showWelcome, setShowWelcome] = useState(() => !hasBeenWelcomed());

  return (
    <ProxyProvider>
      {showWelcome && (
        <WelcomeScreen onContinue={() => setShowWelcome(false)} />
      )}
      <HashRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<DashboardPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="tor" element={<TorPage />} />
            <Route path="proxies" element={<ProxyPage />} />
            <Route path="lists" element={<ProxyListsPage />} />
            <Route path="proxy/new" element={<CreateProxyPage />} />
            <Route path="proxy/:id" element={<ProxyDetailPage />} />
            <Route path="leak-test" element={<LeakTestPage />} />
            <Route path="checker" element={<ProxyCheckerPage />} />
            <Route path="plugins" element={<PluginsPage />} />
            <Route path="split-tunnel" element={<SplitTunnelPage />} />
            <Route path="bandwidth" element={<BandwidthPage />} />
            <Route path="schedule" element={<SchedulePage />} />
          </Route>
        </Routes>
      </HashRouter>
    </ProxyProvider>
  );
}

export default App;
