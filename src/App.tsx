import { HashRouter, Routes, Route } from "react-router-dom";
import { ProxyProvider } from "./contexts/ProxyContext";
import Layout from "./components/Layout";
import DashboardPage from "./pages/DashboardPage";
import SettingsPage from "./pages/SettingsPage";
import TorPage from "./pages/TorPage";
import ProxyDetailPage from "./pages/ProxyDetailPage";
import CreateProxyPage from "./pages/CreateProxyPage";
import ProxyPage from "./pages/ProxyPage";
import ProxyListsPage from "./pages/ProxyListsPage";
import LeakTestPage from "./pages/LeakTestPage";
import ProxyCheckerPage from "./pages/ProxyCheckerPage";

function App() {
  return (
    <ProxyProvider>
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
          </Route>
        </Routes>
      </HashRouter>
    </ProxyProvider>
  );
}

export default App;
