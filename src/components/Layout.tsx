import { useState, useEffect, useRef } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { motion } from "framer-motion";
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
  {
    to: "/plugins",
    label: "Plugins",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M20.24 12.24a6 6 0 0 0-8.49-8.49L5 10.5V19h8.5z" />
        <line x1="16" y1="8" x2="2" y2="22" />
        <line x1="17.5" y1="15" x2="9" y2="15" />
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
  {
    to: "/tor",
    label: "Tor",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        <line x1="2" y1="12" x2="22" y2="12" />
      </svg>
    ),
  },
];

const networkItems: NavItem[] = [
  {
    to: "/split-tunnel",
    label: "Split Tunnel",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M8 6H21" />
        <path d="M8 12H21" />
        <path d="M8 18H21" />
        <path d="M3 6l1 1-1 1" />
        <path d="M3 12l1 1-1 1" />
        <path d="M3 18l1 1-1 1" />
      </svg>
    ),
  },
  {
    to: "/bandwidth",
    label: "Bandwidth",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
      </svg>
    ),
  },
  {
    to: "/schedule",
    label: "Schedule",
    end: true,
    icon: (
      <svg viewBox="0 0 24 24" className="w-[1.125rem] h-[1.125rem]" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
        <line x1="16" y1="2" x2="16" y2="6" />
        <line x1="8" y1="2" x2="8" y2="6" />
        <line x1="3" y1="10" x2="21" y2="10" />
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
                  ? "nav-item-active"
                  : "nav-item-default"
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
  const isFirstRender = useRef(true);

  useEffect(() => {
    setIsSidebarOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    const t = setTimeout(() => {
      isFirstRender.current = false;
    }, 0);
    return () => clearTimeout(t);
  }, []);

  const getPageName = () => {
    if (location.pathname === "/") return "Dashboard";
    if (location.pathname === "/settings") return "Settings";
    if (location.pathname === "/lists") return "Proxy Lists";
    if (location.pathname === "/proxies") return "Proxies";
    if (location.pathname === "/proxy/new") return "Proxies / Create New";
    if (location.pathname.startsWith("/proxy/")) return "Proxy Detail";
    if (location.pathname === "/leak-test") return "Leak Test";
    if (location.pathname === "/checker") return "Proxy Checker";
    if (location.pathname === "/plugins") return "Plugins";
    if (location.pathname === "/tor") return "Tor";
    if (location.pathname === "/split-tunnel") return "Split Tunnel";
    if (location.pathname === "/bandwidth") return "Bandwidth";
    if (location.pathname === "/schedule") return "Schedule";
    return "Page";
  };

  return (
    <div className="flex flex-col h-screen overflow-hidden relative" style={{ background: "var(--color-surface)" }}>

      {}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          backgroundImage: "radial-gradient(circle, var(--layout-dot-color) 1px, transparent 1px)",
          backgroundSize: "28px 28px",
          maskImage: "radial-gradient(ellipse 85% 85% at 50% 35%, black 15%, transparent 100%)",
          WebkitMaskImage: "radial-gradient(ellipse 85% 85% at 50% 35%, black 15%, transparent 100%)",
          zIndex: 0,
        }}
      />
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E")`,
          backgroundSize: "180px 180px",
          opacity: "var(--layout-noise-opacity)",
          zIndex: 0,
        }}
      />
      <div
        className="absolute pointer-events-none rounded-full"
        style={{
          width: 600, height: 600,
          background: "radial-gradient(circle, var(--layout-orb1) 0%, transparent 70%)",
          filter: "blur(70px)",
          top: "-150px", left: "-150px",
          zIndex: 0,
        }}
      />
      <div
        className="absolute pointer-events-none rounded-full"
        style={{
          width: 500, height: 500,
          background: "radial-gradient(circle, var(--layout-orb2) 0%, transparent 70%)",
          filter: "blur(80px)",
          bottom: "-100px", right: "-100px",
          zIndex: 0,
        }}
      />

      {}
      <div className="relative flex flex-col h-full" style={{ zIndex: 1 }}>
        <TitleBar onMenuClick={() => setIsSidebarOpen(!isSidebarOpen)} />

        <div className="flex flex-1 overflow-hidden relative">
          {}
          {isSidebarOpen && (
            <div
              className="fixed inset-0 bg-black/60 z-40 md:hidden"
              onClick={() => setIsSidebarOpen(false)}
            />
          )}

          {}
          <nav
            className={`
              fixed inset-y-0 left-0 z-50 w-[15rem] flex flex-col py-8 px-5 shrink-0
              transition-transform duration-300 ease-in-out
              md:relative md:translate-x-0
              ${isSidebarOpen ? "translate-x-0" : "-translate-x-full"}
            `}
            style={{
              background: "var(--sidebar-bg)",
              backdropFilter: "blur(20px)",
              WebkitBackdropFilter: "blur(20px)",
              borderRight: "1px solid var(--sidebar-border)",
            }}
          >
            {}
            <div className="flex items-center justify-end pl-3 mb-10 md:hidden">
              <button
                onClick={() => setIsSidebarOpen(false)}
                className="p-1 -mr-2 rounded-md text-foreground-muted hover:text-foreground transition-colors"
                style={{ background: "rgba(232,158,107,0.06)" }}
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>

            {}
            <div className="flex-1 overflow-y-auto overflow-x-hidden -mx-2 px-2 scrollbar-none">
              <NavGroup label="Platform" items={platformItems} onItemClick={() => setIsSidebarOpen(false)} />
              <NavGroup label="Security" items={securityItems} onItemClick={() => setIsSidebarOpen(false)} />
              <NavGroup label="Network" items={networkItems} onItemClick={() => setIsSidebarOpen(false)} />
              <NavGroup label="Configuration" items={configItems} onItemClick={() => setIsSidebarOpen(false)} />
            </div>

            {}
            <div className="pt-4 mt-auto" style={{ borderTop: "1px solid var(--sidebar-border)" }}>
              <ThemeToggle />
            </div>
          </nav>

          {}
          <main className="flex-1 overflow-y-auto overscroll-none py-6 px-4 sm:py-8 sm:px-8 md:py-10 md:px-12">
            <motion.div
              key={location.pathname}
              className="h-full"
              initial={isFirstRender.current ? false : { opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.18, ease: "easeOut" }}
            >
              <Outlet context={{ pageName: getPageName() }} />
            </motion.div>
          </main>
        </div>
      </div>
    </div>
  );
}
