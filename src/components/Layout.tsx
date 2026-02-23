import { useState, useEffect } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import ThemeToggle from "./ThemeToggle";
import TitleBar from "./TitleBar";

interface NavItem {
  to: string;
  label: string;
  end: boolean;
  icon: React.ReactNode;
  disabled?: boolean;
}

const platformItems: NavItem[] = [
  {
    to: "/",
    label: "Dashboard",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="3" y="3" width="7" height="7" />
        <rect x="14" y="3" width="7" height="7" />
        <rect x="14" y="14" width="7" height="7" />
        <rect x="3" y="14" width="7" height="7" />
      </svg>
    ),
  },
  {
    to: "/proxies",
    label: "Proxy",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M20 7h-9" />
        <path d="M14 17H5" />
        <circle cx="17" cy="17" r="3" />
        <circle cx="7" cy="7" r="3" />
      </svg>
    ),
  },
  {
    to: "/lists",
    label: "Lists",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <line x1="8" y1="6" x2="21" y2="6" />
        <line x1="8" y1="12" x2="21" y2="12" />
        <line x1="8" y1="18" x2="21" y2="18" />
        <line x1="3" y1="6" x2="3.01" y2="6" />
        <line x1="3" y1="12" x2="3.01" y2="12" />
        <line x1="3" y1="18" x2="3.01" y2="18" />
      </svg>
    ),
  },
  {
    to: "/checker",
    label: "Checker",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
        <polyline points="22 4 12 14.01 9 11.01" />
      </svg>
    ),
  },
];

const securityItems: NavItem[] = [
  {
    to: "/leak-test",
    label: "Leak Test",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
];

const configItems: NavItem[] = [
  {
    to: "/settings",
    label: "Settings",
    end: false,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    ),
  },
];

function NavGroup({ label, items, onItemClick }: { label: string; items: NavItem[]; onItemClick: () => void }) {
  return (
    <div className="flex flex-col gap-1 mb-8">
      <div className="text-[0.6875rem] font-semibold text-foreground-tertiary uppercase tracking-[0.05em] pl-3 mb-2">
        {label}
      </div>
      {items.map((item) =>
        item.disabled ? (
          <span
            key={item.to}
            className="flex items-center gap-3 px-3 py-[0.625rem] rounded-button text-[0.875rem] font-medium text-foreground-tertiary cursor-not-allowed select-none"
          >
            {item.icon}
            {item.label}
          </span>
        ) : (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            onClick={onItemClick}
            className={({ isActive }) =>
              `flex items-center gap-3 px-3 py-[0.625rem] rounded-button text-[0.875rem] font-medium transition-all duration-200 ${
                isActive
                  ? "bg-surface-hover text-foreground"
                  : "text-foreground-muted hover:bg-surface-hover hover:text-foreground"
              }`
            }
          >
            {item.icon}
            {item.label}
          </NavLink>
        )
      )}
    </div>
  );
}

export default function Layout() {
  const location = useLocation();
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);

  // Close sidebar when route changes on mobile
  useEffect(() => {
    setIsSidebarOpen(false);
  }, [location.pathname]);

  // Determine current page title for breadcrumbs
  const getPageName = () => {
    if (location.pathname === "/") return "Dashboard";
    if (location.pathname === "/settings") return "Settings";
    if (location.pathname === "/lists") return "Proxy Lists";
    if (location.pathname === "/proxies") return "Proxies";
    if (location.pathname === "/proxy/new") return "Proxies / Create New";
    if (location.pathname.startsWith("/proxy/")) return "Proxy Detail";
    if (location.pathname === "/leak-test") return "Leak Test";
    if (location.pathname === "/checker") return "Proxy Checker";
    return "Page";
  };

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-background">
      <TitleBar onMenuClick={() => setIsSidebarOpen(!isSidebarOpen)} />
      
      <div className="flex flex-1 overflow-hidden relative">
        {/* Mobile Overlay */}
        {isSidebarOpen && (
          <div 
            className="fixed inset-0 bg-black/50 z-40 md:hidden transition-opacity"
            onClick={() => setIsSidebarOpen(false)}
          />
        )}

        {/* ── Sidebar ─────────────────────────────────────────────── */}
        <nav 
          className={`
            fixed inset-y-0 left-0 z-50 w-[15rem] bg-surface flex flex-col border-r border-border py-8 px-5 shrink-0
            transition-transform duration-300 ease-in-out
            md:relative md:translate-x-0
            ${isSidebarOpen ? "translate-x-0" : "-translate-x-full"}
          `}
        >
          {/* Logo and Mobile close button */}
          <div className="flex items-center justify-between pl-3 mb-10">
            <div className="flex items-center gap-[0.625rem]">
              <div
                className="w-5 h-5 rounded-[0.375rem]"
                style={{
                  background: "linear-gradient(135deg, var(--accent-mid), var(--accent-end))",
                }}
              />
              <span className="font-semibold text-[0.9375rem] tracking-[-0.01em]">
                Relay
              </span>
            </div>
            <button 
              onClick={() => setIsSidebarOpen(false)}
              className="md:hidden p-1 -mr-2 rounded-md hover:bg-surface-hover text-foreground-muted hover:text-foreground transition-colors"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>

          {/* Navigation */}
          <div className="flex-1 overflow-y-auto overflow-x-hidden -mx-2 px-2 scrollbar-none">
            <NavGroup label="Platform" items={platformItems} onItemClick={() => setIsSidebarOpen(false)} />
            <NavGroup label="Security" items={securityItems} onItemClick={() => setIsSidebarOpen(false)} />
            <NavGroup label="Configuration" items={configItems} onItemClick={() => setIsSidebarOpen(false)} />
          </div>

          {/* Theme toggle (bottom) */}
          <div className="pt-4 border-t border-border mt-auto">
            <ThemeToggle />
          </div>
        </nav>

        {/* ── Main content ────────────────────────────────────────── */}
        <main className="flex-1 overflow-y-auto py-6 px-4 sm:py-8 sm:px-8 md:py-10 md:px-12">
          <Outlet context={{ pageName: getPageName() }} />
        </main>
      </div>
    </div>
  );
}
