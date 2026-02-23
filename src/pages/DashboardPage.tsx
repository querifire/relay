import { useState, useEffect, useRef } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useProxies } from "../contexts/ProxyContext";
import ProxyCard from "../components/ProxyCard";
import OverviewCard from "../components/OverviewCard";
import CustomSelect from "../components/CustomSelect";

type FilterStatus = "all" | "running" | "stopped" | "error";

const filterOptions = [
  { value: "all", label: "All" },
  { value: "running", label: "Running" },
  { value: "stopped", label: "Stopped" },
  { value: "error", label: "Error" },
];

export default function DashboardPage() {
  const navigate = useNavigate();
  const {
    instances,
    loading,
    busyIds,
    startInstance,
    stopInstance,
    deleteInstance,
  } = useProxies();

  const [filterStatus, setFilterStatus] = useState<FilterStatus>("all");
  const [trafficData, setTrafficData] = useState<number[]>(new Array(12).fill(0));
  const prevRequestCountRef = useRef<number | null>(null);

  // ── Derive trafficData from instance stats (polled every 3s by ProxyContext) ──
  useEffect(() => {
    const runningInstances = instances.filter((i) => i.status === "Running");
    if (runningInstances.length === 0) {
      prevRequestCountRef.current = null;
      setTrafficData(new Array(12).fill(0));
      return;
    }

    let totalRequests = 0;
    for (const inst of runningInstances) {
      totalRequests += inst.stats?.total_requests ?? 0;
    }

    // First poll — just store the baseline, don't push a bar yet.
    if (prevRequestCountRef.current === null) {
      prevRequestCountRef.current = totalRequests;
      return;
    }

    const delta = Math.max(0, totalRequests - prevRequestCountRef.current);
    prevRequestCountRef.current = totalRequests;

    setTrafficData((prev) => [...prev.slice(1), delta]);
  }, [instances]);

  // ── Filter instances ──────────────────────────────────────────────────
  const filteredInstances = instances.filter((inst) => {
    if (filterStatus === "all") return true;
    if (filterStatus === "running") return inst.status === "Running" || inst.status === "Starting";
    if (filterStatus === "stopped") return inst.status === "Stopped";
    if (filterStatus === "error") return typeof inst.status === "object" && "Error" in inst.status;
    return true;
  });

  return (
    <div>
      {/* ── Header ──────────────────────────────────────────────── */}
      <header className="mb-10">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <Link to="/" className="hover:text-foreground transition-colors">Home</Link>
          <span>/</span>
          <span className="text-foreground">Dashboard</span>
        </div>
        <div className="flex items-center gap-4 flex-wrap">
          <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
            Proxy Overview
          </h1>
          <Link
            to="/proxies"
            className="text-[0.8125rem] text-foreground-muted hover:text-foreground transition-colors"
          >
            Proxies
          </Link>
        </div>
      </header>

      {/* ── Content ─────────────────────────────────────────────── */}
      {loading ? (
        <div className="flex items-center justify-center py-20">
          <p className="text-[0.875rem] text-foreground-muted">Loading...</p>
        </div>
      ) : (
        <div className="grid grid-cols-12 gap-4 md:gap-6">
          {/* Overview card */}
          <OverviewCard instances={instances} trafficData={trafficData} />

          {/* Section header */}
          <div className="col-span-12 flex justify-between items-end mt-5 mb-2 pb-3 border-b border-border">
            <h2 className="text-[1.125rem] font-semibold tracking-[-0.01em]">
              Active Proxies
            </h2>
            <div className="flex gap-3 items-center">
              <div className="w-[8.5rem]">
                <CustomSelect
                  options={filterOptions}
                  value={filterStatus}
                  onChange={(v) => setFilterStatus(v as FilterStatus)}
                  placeholder="Filter"
                />
              </div>
              <button
                onClick={() => navigate("/proxy/new")}
                className="h-9 px-4 rounded-button text-[0.8125rem] font-medium flex items-center gap-2 cursor-pointer transition-all duration-200 bg-foreground text-surface hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]"
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 12 12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M6 1V11M1 6H11" />
                </svg>
                New Proxy
              </button>
            </div>
          </div>

          {/* Proxy cards or empty state */}
          {filteredInstances.length === 0 ? (
            <div className="col-span-12 flex flex-col items-center justify-center py-20 text-center">
              <div className="w-12 h-12 mb-4 rounded-card bg-surface-hover border border-border flex items-center justify-center">
                <svg
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="text-foreground-muted"
                >
                  <rect x="3" y="3" width="7" height="7" rx="1" />
                  <rect x="14" y="3" width="7" height="7" rx="1" />
                  <rect x="3" y="14" width="7" height="7" rx="1" />
                  <rect x="14" y="14" width="7" height="7" rx="1" />
                </svg>
              </div>
              <h3 className="text-[0.875rem] font-medium mb-1">
                {instances.length === 0
                  ? "No proxy instances"
                  : "No matching proxies"}
              </h3>
              <p className="text-[0.75rem] text-foreground-muted mb-4">
                {instances.length === 0
                  ? "Create your first proxy instance to get started"
                  : "Try changing the filter to see more proxies"}
              </p>
              {instances.length === 0 && (
                <button
                  onClick={() => navigate("/proxy/new")}
                  className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 transition-all"
                >
                  Create Proxy
                </button>
              )}
            </div>
          ) : (
            filteredInstances.map((inst) => (
              <ProxyCard
                key={inst.id}
                instance={inst}
                busy={busyIds.has(inst.id)}
                onStart={startInstance}
                onStop={stopInstance}
                onDelete={deleteInstance}
              />
            ))
          )}
        </div>
      )}

    </div>
  );
}
