import { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useProxies } from "../contexts/ProxyContext";
import { detectCountryFlag, getInitials } from "../utils/countryFlags";
import { fetchProxyLists } from "../hooks/useProxyLists";
import type { ProxyListConfig } from "../types";
import CustomSelect from "../components/CustomSelect";

const LATENCY_BAR_COUNT = 17;

interface LogEntry {
  time: string;
  method: string;
  dest: string;
  status: number;
  latency: string;
}

function parseLogEntry(line: string): LogEntry {
  const torMatch = line.match(/^\[tor\]\s*(.+?\[(\w+)\]\s*(.+))$/);
  if (torMatch) {
    return {
      time: "—",
      method: torMatch[2]?.toUpperCase() ?? "TOR",
      dest: torMatch[3] ?? torMatch[1],
      status: 0,
      latency: "—",
    };
  }

  const connectMatch = line.match(/CONNECT\s+([^\s:]+):?(\d*)/i);
  if (connectMatch) {
    return {
      time: "—",
      method: "CONNECT",
      dest: connectMatch[1],
      status: 200,
      latency: "—",
    };
  }
  return {
    time: "—",
    method: "LOG",
    dest: line,
    status: 0,
    latency: "—",
  };
}

function CopyIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-foreground-tertiary"
    >
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function ProxyListSelector({
  instanceId,
  currentList,
}: {
  instanceId: string | undefined;
  currentList: string;
}) {
  const [customLists, setCustomLists] = useState<ProxyListConfig[]>([]);
  const [selected, setSelected] = useState(currentList);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetchProxyLists().then(setCustomLists);
  }, []);

  // Keep local state in sync with prop
  useEffect(() => {
    setSelected(currentList);
  }, [currentList]);

  const options = useMemo(() => {
    const opts = [{ value: "default", label: "Default (Built-in)" }];
    for (const l of customLists) {
      opts.push({ value: l.id, label: l.name });
    }
    return opts;
  }, [customLists]);

  const currentLabel =
    options.find((o) => o.value === currentList)?.label ?? currentList;

  const handleApply = async () => {
    if (!instanceId || selected === currentList) return;
    setSaving(true);
    try {
      await invoke("update_instance_proxy_list", {
        id: instanceId,
        proxyList: selected,
      });
    } catch (err) {
      console.error("Failed to update proxy list:", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="mb-5">
      <div className="flex justify-between items-center mb-1">
        <span className="text-[0.8125rem] font-medium">Proxy List</span>
      </div>
      <p className="text-[0.75rem] text-foreground-muted mb-2">
        {currentLabel}
      </p>
      {options.length > 1 && (
        <div className="flex items-center gap-2 mt-2">
          <div className="flex-1">
            <CustomSelect
              options={options}
              value={selected}
              onChange={setSelected}
              placeholder="Select list"
            />
          </div>
          {selected !== currentList && (
            <button
              disabled={saving}
              onClick={handleApply}
              className="h-10 px-3 rounded-button text-[0.75rem] font-medium bg-foreground text-surface hover:opacity-80 transition-all disabled:opacity-50"
            >
              {saving ? "Saving…" : "Apply"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Page component
// ═══════════════════════════════════════════════════════════════════════════
export default function ProxyDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { instances, busyIds, startInstance, stopInstance, deleteInstance, toggleAutoRotate, updateAutoRotateMinutes } =
    useProxies();

  const instance = instances.find((i) => i.id === id);

  // ── Logs ───────────────────────────────────────────────────────────────
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    if (!id) return;
    const fetchLogs = async () => {
      try {
        const result = await invoke<string[]>("get_instance_logs", { id });
        setLogs(result);
      } catch {
        /* instance may not exist yet */
      }
    };
    fetchLogs();
    const interval = setInterval(fetchLogs, 2000);
    return () => clearInterval(interval);
  }, [id]);

  // ── Rename ─────────────────────────────────────────────────────────────
  const [editing, setEditing] = useState(false);
  const [newName, setNewName] = useState("");

  const handleRename = async () => {
    if (!id || !newName.trim()) return;
    try {
      await invoke("rename_instance", { id, name: newName.trim() });
      setEditing(false);
    } catch (err) {
      console.error("Rename failed:", err);
    }
  };

  // ── Latency history for the chart ──────────────────────────────────────
  const [latencyHistory, setLatencyHistory] = useState<number[]>(
    new Array(LATENCY_BAR_COUNT).fill(0),
  );

  useEffect(() => {
    if (!instance) return;
    const isActive = instance.status === "Running";
    if (!isActive) {
      setLatencyHistory(new Array(LATENCY_BAR_COUNT).fill(0));
      return;
    }

    // Use last_request_latency_ms for real per-request fluctuations on the chart.
    // Fall back to avg_latency_ms if no per-request data yet.
    const lastReqMs = instance.stats?.last_request_latency_ms ?? 0;
    const avgMs = instance.stats?.avg_latency_ms ?? 0;
    const dataPoint = lastReqMs > 0 ? lastReqMs : avgMs;
    setLatencyHistory((prev) => [...prev.slice(1), dataPoint]);
  }, [instance]);

  // ── Show all logs toggle ─────────────────────────────────────────────
  const [showAllLogs, setShowAllLogs] = useState(false);

  // ── Test connection & Change IP ──────────────────────────────────────
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<number | "fail" | null>(null);
  const [changingIp, setChangingIp] = useState(false);
  const [changeIpResult, setChangeIpResult] = useState<"ok" | "fail" | null>(null);

  const handleTestConnection = async () => {
    if (!id || testing) return;
    setTesting(true);
    setTestResult(null);
    try {
      const latency = await invoke<number>("test_connection", { id });
      setTestResult(latency);
      setTimeout(() => setTestResult(null), 4000);
    } catch {
      setTestResult("fail");
      setTimeout(() => setTestResult(null), 4000);
    } finally {
      setTesting(false);
    }
  };

  const handleChangeIp = async () => {
    if (!id || changingIp) return;
    setChangingIp(true);
    setChangeIpResult(null);
    try {
      await invoke("change_ip", { id });
      setChangeIpResult("ok");
      setTimeout(() => setChangeIpResult(null), 3000);
    } catch (err) {
      console.error("Change IP failed:", err);
      setChangeIpResult("fail");
      setTimeout(() => setChangeIpResult(null), 3000);
    } finally {
      setChangingIp(false);
    }
  };

  // ── Delete with confirmation ───────────────────────────────────────────
  const [confirmDelete, setConfirmDelete] = useState(false);

  const handleDelete = async () => {
    if (!id) return;
    await deleteInstance(id);
    navigate("/");
  };

  // ── Copy to clipboard ─────────────────────────────────────────────────
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  // ── Guard ──────────────────────────────────────────────────────────────
  if (!instance) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center">
        <p className="text-[0.875rem] text-foreground-muted mb-4">
          Instance not found
        </p>
        <button
          onClick={() => navigate("/")}
          className="text-[0.875rem] text-foreground-muted hover:text-foreground transition-colors"
        >
          Back to Dashboard
        </button>
      </div>
    );
  }

  // ── Derived state ──────────────────────────────────────────────────────
  const isRunning = instance.status === "Running";
  const isStarting = instance.status === "Starting";
  const isBusy = busyIds.has(instance.id);
  const hasError =
    typeof instance.status === "object" && "Error" in instance.status;
  const errorMsg = hasError
    ? (instance.status as { Error: string }).Error
    : null;

  const protocolMap: Record<string, string> = {
    Http: "HTTP/S",
    Https: "HTTP/S",
    Socks4: "SOCKS4",
    Socks5: "SOCKS5",
    Tor: "TOR",
  };

  const protocol = instance.mode === "Tor"
    ? "TOR"
    : instance.upstream
      ? (protocolMap[instance.upstream.protocol] ?? instance.upstream.protocol)
      : (protocolMap[instance.local_protocol] ?? instance.local_protocol);

  const statusLabel = isRunning
    ? "Active"
    : isStarting
      ? "Starting"
      : hasError
        ? "Error"
        : "Idle";

  const statusDotColor = isRunning
    ? "bg-[#34C759]"
    : isStarting
      ? "bg-[#FF9F0A]"
      : hasError
        ? "bg-[#FF3B30]"
        : "bg-foreground-muted";

  // ── Real performance stats ─────────────────────────────────────────────
  const stats = instance.stats;
  const avgLatency = stats?.avg_latency_ms ?? 0;
  const successRate = stats?.success_rate ?? 0;
  const totalBytes = stats?.total_bytes ?? 0;

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  // ── Latency bar heights (normalised) ──────────────────────────────────
  const maxLatency = Math.max(...latencyHistory, 1);
  const latencyBarHeights = latencyHistory.map(
    (v) => (v > 0 ? 15 + (v / maxLatency) * 80 : 4),
  );

  // Parse log entries for the activity table
  const parsedLogs = logs.map(parseLogEntry).reverse();
  const visibleLogs = showAllLogs ? parsedLogs : parsedLogs.slice(0, 10);

  // Country flag detection
  const countryFlag = detectCountryFlag(instance.name);
  const initials = getInitials(instance.name);

  // ════════════════════════════════════════════════════════════════════════
  // Render
  // ════════════════════════════════════════════════════════════════════════
  return (
    <div>
      {/* ── Header ──────────────────────────────────────────────────── */}
      <header className="mb-8 flex justify-between items-end">
        <div>
          {/* Breadcrumbs: Home / Proxies / Proxy */}
          <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
            <button
              type="button"
              onClick={() => navigate("/")}
              className="hover:text-foreground transition-colors"
            >
              Home
            </button>
            <span>/</span>
            <button
              type="button"
              onClick={() => navigate("/proxies")}
              className="hover:text-foreground transition-colors"
            >
              Proxies
            </button>
            <span>/</span>
            <span className="text-foreground">Proxy</span>
          </div>

          {/* Title (editable) */}
          {editing ? (
            <div className="flex items-center gap-2">
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRename();
                  if (e.key === "Escape") setEditing(false);
                }}
                className="text-[2rem] font-semibold tracking-[-0.03em] bg-surface border border-border rounded-button px-3 py-1 outline-none focus:border-border-focus"
              />
              <button
                onClick={handleRename}
                className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface"
              >
                Save
              </button>
              <button
                onClick={() => setEditing(false)}
                className="h-10 px-4 rounded-button text-[0.8125rem] font-medium border border-border hover:bg-surface-hover transition-colors"
              >
                Cancel
              </button>
            </div>
          ) : (
            <h1
              className="text-[2rem] font-semibold tracking-[-0.03em] flex items-center gap-3 cursor-pointer hover:opacity-70 transition-opacity"
              onClick={() => {
                setNewName(instance.name);
                setEditing(true);
              }}
              title="Click to rename"
            >
              <div className="w-6 h-6 bg-surface-hover rounded-full flex items-center justify-center text-[0.625rem] font-bold shrink-0 border border-border">
                {countryFlag ?? initials}
              </div>
              {instance.name}
            </h1>
          )}
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-3 shrink-0">
          {isRunning ? (
            <>
              {instance.mode !== "Tor" && (
                <>
                  <button
                    disabled={testing}
                    onClick={handleTestConnection}
                    className={`h-10 px-4 rounded-button text-[0.8125rem] font-medium border transition-all disabled:opacity-50 flex items-center gap-2 ${
                      testResult === "fail"
                        ? "bg-[rgba(255,59,48,0.1)] text-[#FF3B30] border-[rgba(255,59,48,0.3)]"
                        : testResult !== null
                          ? "bg-[rgba(52,199,89,0.1)] text-[#34C759] border-[rgba(52,199,89,0.3)]"
                          : "bg-surface-hover text-foreground border-border hover:border-border-focus"
                    }`}
                  >
                    {testing
                      ? "Testing…"
                      : testResult === "fail"
                        ? "Failed"
                        : testResult !== null
                          ? `${testResult}ms`
                          : "Test Connection"}
                  </button>
                  <button
                    disabled={changingIp}
                    onClick={handleChangeIp}
                    className={`h-10 px-4 rounded-button text-[0.8125rem] font-medium border transition-all disabled:opacity-50 flex items-center gap-2 ${
                      changeIpResult === "fail"
                        ? "bg-[rgba(255,59,48,0.1)] text-[#FF3B30] border-[rgba(255,59,48,0.3)]"
                        : changeIpResult === "ok"
                          ? "bg-[rgba(52,199,89,0.1)] text-[#34C759] border-[rgba(52,199,89,0.3)]"
                          : "bg-surface-hover text-foreground border-border hover:border-border-focus"
                    }`}
                  >
                    {changingIp
                      ? "Changing…"
                      : changeIpResult === "fail"
                        ? "Failed"
                        : changeIpResult === "ok"
                          ? "IP Changed"
                          : "Change IP"}
                  </button>
                </>
              )}
              <button
                disabled={isBusy}
                onClick={() => stopInstance(instance.id)}
                className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-surface-hover text-foreground border border-border hover:border-border-focus transition-all disabled:opacity-50 flex items-center gap-2"
              >
                {isBusy ? "Stopping…" : instance.mode === "Tor" ? "Stop Tor" : "Stop Proxy"}
              </button>
              {instance.mode !== "Tor" && (
                <button
                  disabled={isBusy}
                  onClick={() => {
                    stopInstance(instance.id);
                    setTimeout(() => startInstance(instance.id), 1000);
                  }}
                  className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 transition-all disabled:opacity-50 flex items-center gap-2"
                >
                  Restart
                </button>
              )}
            </>
          ) : isStarting ? (
            <button
              disabled={isBusy}
              onClick={() => stopInstance(instance.id)}
              className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-[#FF9F0A] text-white hover:opacity-80 transition-all disabled:opacity-50 flex items-center gap-2"
            >
              {isBusy ? "Stopping…" : "Cancel"}
            </button>
          ) : (
            <button
              disabled={isBusy}
              onClick={() => startInstance(instance.id)}
              className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px transition-all disabled:opacity-50 flex items-center gap-2"
            >
              Start Proxy
            </button>
          )}
        </div>
      </header>

      {/* ── Details grid ────────────────────────────────────────────── */}
      <div className="grid grid-cols-12 gap-4 md:gap-6">

        {/* ─── Connection Details (span 5) ────────────────────────── */}
        <div className="col-span-12 lg:col-span-5 bg-surface-card border border-border rounded-card p-6 shadow-card">
          <div className="text-[1rem] font-semibold mb-5">
            Connection Details
          </div>

          <div className="flex flex-col gap-4">
            {/* Status */}
            <div className="flex justify-between items-center pb-3 border-b border-border">
              <span className="text-foreground-muted text-[0.8125rem]">Status</span>
              <span className="flex items-center text-[0.8125rem] font-semibold">
                <span className={`w-2 h-2 rounded-full mr-2 ${statusDotColor}`} />
                {statusLabel}
              </span>
            </div>

            {/* IP Address */}
            <div className="flex justify-between items-center pb-3 border-b border-border">
              <span className="text-foreground-muted text-[0.8125rem]">IP Address</span>
              <span
                className="font-mono font-medium bg-surface-hover px-2 py-1 rounded-[0.375rem] text-[0.8125rem] cursor-pointer hover:bg-border transition-colors flex items-center gap-1.5"
                onClick={() => copyToClipboard(instance.bind_addr, "ip")}
                title="Click to copy"
              >
                {instance.bind_addr}
                {copiedField === "ip" ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#34C759" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : (
                  <CopyIcon />
                )}
              </span>
            </div>

            {/* Port */}
            <div className="flex justify-between items-center pb-3 border-b border-border">
              <span className="text-foreground-muted text-[0.8125rem]">Port</span>
              <span
                className="font-mono font-medium bg-surface-hover px-2 py-1 rounded-[0.375rem] text-[0.8125rem] cursor-pointer hover:bg-border transition-colors flex items-center gap-1.5"
                onClick={() => copyToClipboard(String(instance.port), "port")}
                title="Click to copy"
              >
                {instance.port}
                {copiedField === "port" ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#34C759" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : (
                  <CopyIcon />
                )}
              </span>
            </div>

            {/* Protocol */}
            <div className="flex justify-between items-center pb-3 border-b border-border">
              <span className="text-foreground-muted text-[0.8125rem]">Protocol</span>
              <span className="font-mono font-medium bg-surface-hover px-2 py-1 rounded-[0.375rem] text-[0.8125rem]">
                {protocol}
              </span>
            </div>

            {/* Mode */}
            <div className="flex justify-between items-center pb-3 border-b border-border">
              <span className="text-foreground-muted text-[0.8125rem]">Mode</span>
              <span className="font-mono font-medium bg-surface-hover px-2 py-1 rounded-[0.375rem] text-[0.8125rem]">
                {instance.mode}
              </span>
            </div>

            {/* Upstream / Tor */}
            <div className="flex justify-between items-center">
              <span className="text-foreground-muted text-[0.8125rem]">
                {instance.mode === "Tor" ? "Network" : "Upstream"}
              </span>
              <span className="font-mono font-medium bg-surface-hover px-2 py-1 rounded-[0.375rem] text-[0.75rem]">
                {instance.mode === "Tor"
                  ? "Tor Network"
                  : instance.upstream
                    ? `${instance.upstream.protocol.toLowerCase()}://${instance.upstream.host}:${instance.upstream.port}`
                    : "—"}
              </span>
            </div>
          </div>

          {/* Error message */}
          {errorMsg && (
            <div className="mt-5 text-[0.75rem] text-[#FF3B30] bg-[rgba(255,59,48,0.1)] px-4 py-3 rounded-button">
              {errorMsg}
            </div>
          )}
        </div>

        {/* ─── Performance Metrics (span 7) ───────────────────────── */}
        <div className="col-span-12 lg:col-span-7 bg-surface-card border border-border rounded-card p-6 shadow-card relative overflow-hidden metrics-glow flex flex-col">
          <div className="text-[1rem] font-semibold mb-5 relative z-10">
            Performance
          </div>

          {/* Stat row */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 sm:gap-5 mb-6 relative z-10">
            <div className="flex flex-col">
              <span className="text-[0.75rem] text-foreground-muted mb-1">Avg. Latency</span>
              <span className="text-[1.5rem] font-semibold">
                {isRunning && avgLatency > 0 ? `${avgLatency}ms` : "—"}
              </span>
            </div>
            <div className="flex flex-col">
              <span className="text-[0.75rem] text-foreground-muted mb-1">Success Rate</span>
              <span className="text-[1.5rem] font-semibold">
                {isRunning && (stats?.total_requests ?? 0) > 0
                  ? `${(successRate * 100).toFixed(1)}%`
                  : "—"}
              </span>
            </div>
            <div className="flex flex-col">
              <span className="text-[0.75rem] text-foreground-muted mb-1">Data Used</span>
              <span className="text-[1.5rem] font-semibold">
                {isRunning && totalBytes > 0 ? formatBytes(totalBytes) : "—"}
              </span>
            </div>
          </div>

          {/* Latency graph */}
          <span className="text-[0.75rem] text-foreground-muted relative z-10">
            Latency Fluctuations
          </span>
          <div className="flex-1 min-h-[7.5rem] flex items-end gap-[0.375rem] mt-[0.625rem] relative z-10">
            {latencyBarHeights.map((h, i) => (
              <div
                key={i}
                className={`latency-bar ${latencyHistory[i] > maxLatency * 0.7 && latencyHistory[i] > 0 ? "high" : ""}`}
                style={{ height: isRunning ? `${h}%` : "4px" }}
              />
            ))}
          </div>
        </div>

        {/* ─── Activity Logs (span 8) ─────────────────────────────── */}
        <div className="col-span-12 lg:col-span-8 bg-surface-card border border-border rounded-card p-6 shadow-card">
          <div className="text-[1rem] font-semibold mb-5 flex items-center justify-between">
            Activity Logs
            {parsedLogs.length > 10 && (
              <button
                type="button"
                onClick={() => setShowAllLogs(!showAllLogs)}
                className="text-[0.75rem] text-foreground-muted font-normal cursor-pointer hover:text-foreground transition-colors"
              >
                {showAllLogs ? "Show Less" : "View All"}
              </button>
            )}
          </div>

          {/* Table */}
          <div className={showAllLogs ? "overflow-auto" : "max-h-[18rem] overflow-auto"}>
            <table className="w-full">
              <thead>
                <tr>
                  <th className="text-left text-[0.75rem] text-foreground-muted font-medium pb-3 border-b border-border">
                    TIME
                  </th>
                  <th className="text-left text-[0.75rem] text-foreground-muted font-medium pb-3 border-b border-border">
                    METHOD
                  </th>
                  <th className="text-left text-[0.75rem] text-foreground-muted font-medium pb-3 border-b border-border">
                    DESTINATION
                  </th>
                  <th className="text-left text-[0.75rem] text-foreground-muted font-medium pb-3 border-b border-border">
                    STATUS
                  </th>
                  <th className="text-left text-[0.75rem] text-foreground-muted font-medium pb-3 border-b border-border">
                    LATENCY
                  </th>
                </tr>
              </thead>
              <tbody>
                {visibleLogs.length > 0 ? (
                  visibleLogs.map((row, i) => (
                    <tr key={i}>
                      <td className="py-4 border-b border-border text-[0.8125rem] text-foreground-muted">
                        {row.time}
                      </td>
                      <td className="py-4 border-b border-border text-[0.8125rem]">
                        <span className="method-badge">{row.method}</span>
                      </td>
                      <td className="py-4 border-b border-border text-[0.8125rem] font-mono">
                        {row.dest}
                      </td>
                      <td className="py-4 border-b border-border text-[0.8125rem]">
                        {row.status > 0 ? (
                          <span className={`font-mono font-semibold ${row.status < 300 ? "status-success" : row.status < 400 ? "status-warning" : "status-error"}`}>
                            {row.status}
                          </span>
                        ) : (
                          <span className="text-foreground-muted">—</span>
                        )}
                      </td>
                      <td className="py-4 border-b border-border text-[0.8125rem]">
                        {row.latency}
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td
                      colSpan={5}
                      className="py-8 text-center text-foreground-muted text-[0.8125rem]"
                    >
                      No activity — start the proxy to see traffic logs
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>

        {/* ─── Configuration (span 4) ─────────────────────────────── */}
        <div className="col-span-12 lg:col-span-4 bg-surface-card border border-border rounded-card p-6 shadow-card">
          <div className="text-[1rem] font-semibold mb-5">Configuration</div>

          {/* Auto-Rotation (only for Auto mode) */}
          {instance.mode === "Auto" && (
            <div className="mb-5">
              <div className="flex justify-between items-center mb-1">
                <span className="text-[0.8125rem] font-medium">Auto-Rotation</span>
                <div
                  className={`toggle-switch ${instance.auto_rotate ? "on" : ""}`}
                  onClick={() => {
                    if (id) toggleAutoRotate(id, !instance.auto_rotate);
                  }}
                />
              </div>
              <p className="text-[0.75rem] text-foreground-muted">
                Automatically rotate to the fastest proxy at a set interval.
              </p>
              {instance.auto_rotate && (
                <div className="mt-3">
                  <label className="block text-[0.75rem] font-medium text-foreground-muted mb-1.5">
                    Rotate every (minutes)
                  </label>
                  <input
                    type="number"
                    value={instance.auto_rotate_minutes ?? 5}
                    onChange={(e) => {
                      if (id) {
                        const val = Math.max(1, Number(e.target.value) || 1);
                        updateAutoRotateMinutes(id, val);
                      }
                    }}
                    min={1}
                    className="w-full px-3 py-2 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
                  />
                </div>
              )}
            </div>
          )}

          {/* Proxy List (not for Tor) */}
          {instance.mode !== "Tor" && (
            <ProxyListSelector instanceId={id} currentList={instance.proxy_list} />
          )}

          {/* Danger zone */}
          <div className="mt-8 pt-6 border-t border-border">
            {confirmDelete ? (
              <div className="flex flex-col gap-2">
                <span className="text-[0.75rem] text-foreground-muted text-center mb-1">
                  Are you sure? This action cannot be undone.
                </span>
                <button
                  onClick={handleDelete}
                  className="h-10 w-full rounded-button text-[0.8125rem] font-medium bg-[#FF3B30] text-white hover:bg-[#E5342B] transition-colors flex items-center justify-center"
                >
                  Yes, delete
                </button>
                <button
                  onClick={() => setConfirmDelete(false)}
                  className="h-10 w-full rounded-button text-[0.8125rem] font-medium border border-border hover:bg-surface-hover transition-colors"
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                disabled={isBusy}
                onClick={() => setConfirmDelete(true)}
                className="h-10 w-full rounded-button text-[0.8125rem] font-medium text-[#FF3B30] bg-[rgba(255,59,48,0.1)] hover:bg-[rgba(255,59,48,0.15)] transition-colors flex items-center justify-center disabled:opacity-50"
              >
                Delete Proxy Instance
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
