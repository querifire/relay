import { useState, useRef, useCallback, useEffect } from "react";
import { useProxyLists, useProxyCacheStats } from "../hooks/useProxyLists";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import CustomSelect from "../components/CustomSelect";

interface ProxyWithSpeed {
  proxy: { host: string; port: number; protocol: string };
  latency: number;
}

interface CheckerProgress {
  tested: number;
  working: number;
  failed: number;
  total: number;
  phase: string;
}

interface CheckerLog {
  message: string;
  level: string;
}

interface LogEntry {
  ts: Date;
  text: string;
  type: "info" | "success" | "error" | "warn";
}

export default function ProxyCheckerPage() {
  const { lists } = useProxyLists();
  const { stats: cacheStats, refresh: refreshCacheStats } =
    useProxyCacheStats();

  const [selectedList, setSelectedList] = useState("default");
  const [proxyType, setProxyType] = useState("auto");
  const [concurrency, setConcurrency] = useState(50);
  const [timeout, setTimeout_] = useState(5000);
  const [checkAnonymity, setCheckAnonymity] = useState(true);
  const [checkLocation, setCheckLocation] = useState(true);

  const [checking, setChecking] = useState(false);
  const [results, setResults] = useState("");
  const [workingCount, setWorkingCount] = useState(0);
  const [failedCount, setFailedCount] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [copied, setCopied] = useState(false);

  const [logs, setLogs] = useState<LogEntry[]>([]);
  const logEndRef = useRef<HTMLDivElement>(null);
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  function pushLog(text: string, type: LogEntry["type"] = "info") {
    setLogs((prev) => [...prev, { ts: new Date(), text, type }]);
  }

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  // Clean up event listeners on unmount
  useEffect(() => {
    return () => {
      for (const unlisten of unlistenRefs.current) unlisten();
      unlistenRefs.current = [];
    };
  }, []);

  const listOptions = [
    {
      value: "default",
      label: `Default (${cacheStats?.total ?? 0})`,
    },
    ...lists.map((l) => ({
      value: l.id,
      label: `${l.name} (${l.inline_proxies.length + l.urls.length})`,
    })),
  ];

  const typeOptions = [
    { value: "auto", label: "Auto Detect" },
    { value: "Http", label: "HTTP / HTTPS" },
    { value: "Socks5", label: "SOCKS5" },
    { value: "Socks4", label: "SOCKS4" },
  ];

  const protocolMap: Record<string, string | null> = {
    auto: null,
    Http: "Http",
    Socks5: "Socks5",
    Socks4: "Socks4",
  };

  const progress =
    totalCount > 0
      ? Math.round(((workingCount + failedCount) / totalCount) * 100)
      : 0;

  const handleStartCheck = useCallback(async () => {
    if (checking) return;

    setChecking(true);
    setResults("");
    setWorkingCount(0);
    setFailedCount(0);
    setTotalCount(0);
    setLogs([]);

    const protocol = protocolMap[proxyType] ?? null;
    const listLabel =
      selectedList === "default"
        ? "Default"
        : lists.find((l) => l.id === selectedList)?.name ?? selectedList;

    pushLog(`Starting proxy check — list: ${listLabel}`);

    // Subscribe to backend events
    for (const unlisten of unlistenRefs.current) unlisten();
    unlistenRefs.current = [];

    const unlistenLog = await listen<CheckerLog>("checker-log", (event) => {
      const { message, level } = event.payload;
      const type =
        level === "success"
          ? "success"
          : level === "error"
            ? "error"
            : level === "warn"
              ? "warn"
              : "info";
      pushLog(message, type as LogEntry["type"]);
    });
    unlistenRefs.current.push(unlistenLog);

    const unlistenProgress = await listen<CheckerProgress>(
      "checker-progress",
      (event) => {
        const p = event.payload;
        setWorkingCount(p.working);
        setFailedCount(p.failed);
        setTotalCount(p.total);
      },
    );
    unlistenRefs.current.push(unlistenProgress);

    try {
      if (selectedList !== "default") {
        pushLog(`Refreshing custom list "${listLabel}"...`);
        await invoke("refresh_proxy_list", { id: selectedList });
        pushLog("Custom list refreshed", "success");
        await refreshCacheStats();
      }

      const tested = await invoke<ProxyWithSpeed[]>("check_proxies_live", {
        protocol,
      });

      const lines = tested.map((p) => `${p.proxy.host}:${p.proxy.port}`);
      setResults(lines.join("\n"));
      setWorkingCount(tested.length);

      await refreshCacheStats();
    } catch (err) {
      const msg =
        typeof err === "string"
          ? err
          : (err as Error)?.message ?? "Unknown error";
      pushLog(`Error: ${msg}`, "error");
    } finally {
      // Unsubscribe from events
      for (const unlisten of unlistenRefs.current) unlisten();
      unlistenRefs.current = [];
      setChecking(false);
    }
  }, [checking, selectedList, proxyType, lists, refreshCacheStats]);

  const handleCopy = async () => {
    if (!results) return;
    try {
      await navigator.clipboard.writeText(results);
      setCopied(true);
      pushLog(`Copied ${workingCount} proxies to clipboard`, "success");
      globalThis.setTimeout(() => setCopied(false), 2000);
    } catch {
      pushLog("Failed to copy to clipboard", "error");
    }
  };

  const handleDownload = () => {
    if (!results) return;
    const blob = new Blob([results], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "working_proxies.txt";
    a.click();
    URL.revokeObjectURL(url);
    pushLog("Downloaded working_proxies.txt", "success");
  };

  const fmtTime = (d: Date) =>
    d.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

  const logColor: Record<LogEntry["type"], string> = {
    info: "text-foreground-muted",
    success: "text-[#34C759]",
    error: "text-[#FF3B30]",
    warn: "text-[#FF9F0A]",
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <header className="mb-8 shrink-0">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span>Proxies</span>
          <span>/</span>
          <span className="text-foreground">Checker</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Proxy Checker
        </h1>
      </header>

      {/* Checker layout */}
      <div className="grid grid-cols-[23.75rem_1fr] gap-6 flex-1 min-h-0">
        {/* Config panel */}
        <aside className="flex flex-col gap-6 overflow-y-auto">
          {/* Source Selection */}
          <div className="bg-surface-card border border-border rounded-card p-6 shadow-card">
            <div className="text-[0.875rem] font-semibold mb-4 flex items-center gap-2">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              Source Selection
            </div>

            <div className="mb-5">
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Proxy List
              </label>
              <CustomSelect
                options={listOptions}
                value={selectedList}
                onChange={setSelectedList}
                placeholder="Select proxy list"
              />
            </div>

            <div>
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Proxy Type
              </label>
              <CustomSelect
                options={typeOptions}
                value={proxyType}
                onChange={setProxyType}
                placeholder="Select type"
              />
            </div>
          </div>

          {/* Check Settings */}
          <div className="bg-surface-card border border-border rounded-card p-6 shadow-card">
            <div className="text-[0.875rem] font-semibold mb-4 flex items-center gap-2">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
              Check Settings
            </div>

            <div className="mb-5">
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Concurrency (Threads)
              </label>
              <input
                type="number"
                value={concurrency}
                onChange={(e) => setConcurrency(Number(e.target.value) || 1)}
                className="w-full h-10 px-3 rounded-button border border-border bg-surface-hover text-[0.8125rem] font-sans outline-none focus:border-border-focus transition-colors"
              />
            </div>

            <div className="mb-5">
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Timeout (ms)
              </label>
              <input
                type="number"
                value={timeout}
                onChange={(e) => setTimeout_(Number(e.target.value) || 1000)}
                className="w-full h-10 px-3 rounded-button border border-border bg-surface-hover text-[0.8125rem] font-sans outline-none focus:border-border-focus transition-colors"
              />
            </div>

            <div className="flex items-center justify-between mb-3">
              <span className="text-[0.75rem] font-medium text-foreground-muted">
                Check Anonymity
              </span>
              <div
                className={`toggle-switch ${checkAnonymity ? "on" : ""}`}
                onClick={() => setCheckAnonymity(!checkAnonymity)}
              />
            </div>

            <div className="flex items-center justify-between mb-5">
              <span className="text-[0.75rem] font-medium text-foreground-muted">
                Check Location
              </span>
              <div
                className={`toggle-switch ${checkLocation ? "on" : ""}`}
                onClick={() => setCheckLocation(!checkLocation)}
              />
            </div>

            <button
              onClick={handleStartCheck}
              disabled={checking}
              className="w-full h-[2.625rem] rounded-button text-[0.8125rem] font-medium bg-foreground text-surface flex items-center justify-center gap-2 cursor-pointer transition-all duration-200 border-none hover:opacity-90 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)] disabled:opacity-70"
            >
              {checking ? (
                <>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" className="animate-spin">
                    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                  </svg>
                  Checking...
                </>
              ) : (
                <>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                  Start Checking
                </>
              )}
            </button>
          </div>
        </aside>

        {/* Right panel: results + console */}
        <section className="flex flex-col min-h-0 gap-4">
          {/* Results card */}
          <div className="bg-surface-card border border-border rounded-card shadow-card flex-1 flex flex-col overflow-hidden min-h-0">
            <div className="px-6 py-4 border-b border-border flex justify-between items-center shrink-0">
              <div className="text-[0.875rem] font-semibold flex items-center gap-2">
                Working Proxies
              </div>
              <div className="flex gap-2">
                <button
                  onClick={handleCopy}
                  disabled={!results}
                  className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-surface-hover border border-border flex items-center gap-2 cursor-pointer transition-all duration-200 hover:bg-border disabled:opacity-40 disabled:cursor-default"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                  </svg>
                  {copied ? "Copied!" : "Copy"}
                </button>
                <button
                  onClick={handleDownload}
                  disabled={!results}
                  className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-surface-hover border border-border flex items-center gap-2 cursor-pointer transition-all duration-200 hover:bg-border disabled:opacity-40 disabled:cursor-default"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                  .txt
                </button>
              </div>
            </div>

            <textarea
              className="flex-1 w-full border-none p-6 font-mono text-[0.8125rem] leading-[1.6] resize-none outline-none bg-transparent min-h-0"
              placeholder="Results will appear here..."
              value={results}
              readOnly
            />

            <div className="px-6 py-4 border-t border-border flex justify-between items-center bg-surface-hover shrink-0">
              <div className="flex gap-6">
                <div className="flex flex-col">
                  <span className="text-[0.6875rem] uppercase tracking-[0.05em] text-foreground-muted font-semibold">
                    Working
                  </span>
                  <span className="text-[0.875rem] font-semibold text-[#34C759]">
                    {workingCount}
                  </span>
                </div>
                <div className="flex flex-col">
                  <span className="text-[0.6875rem] uppercase tracking-[0.05em] text-foreground-muted font-semibold">
                    Failed
                  </span>
                  <span className="text-[0.875rem] font-semibold text-[#FF3B30]">
                    {failedCount}
                  </span>
                </div>
                <div className="flex flex-col">
                  <span className="text-[0.6875rem] uppercase tracking-[0.05em] text-foreground-muted font-semibold">
                    Total
                  </span>
                  <span className="text-[0.875rem] font-semibold">
                    {totalCount}
                  </span>
                </div>
              </div>
              <div className="flex flex-col items-end">
                <span className="text-[0.6875rem] uppercase tracking-[0.05em] text-foreground-muted font-semibold">
                  Progress
                </span>
                <span className="text-[0.875rem] font-semibold">
                  {checking ? `${progress}%` : totalCount > 0 ? "100%" : "—"}
                </span>
              </div>
            </div>
          </div>

          {/* Console log */}
          <div className="bg-surface-card border border-border rounded-card shadow-card shrink-0 h-[11rem] flex flex-col overflow-hidden">
            <div className="px-5 py-3 border-b border-border flex items-center gap-2 shrink-0">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-foreground-muted">
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
              <span className="text-[0.8125rem] font-semibold">Console</span>
              {logs.length > 0 && (
                <button
                  onClick={() => setLogs([])}
                  className="ml-auto text-[0.6875rem] text-foreground-muted hover:text-foreground transition-colors cursor-pointer bg-transparent border-none"
                >
                  Clear
                </button>
              )}
            </div>
            <div className="flex-1 overflow-y-auto px-5 py-3 font-mono text-[0.75rem] leading-[1.7]">
              {logs.length === 0 && (
                <span className="text-foreground-tertiary">
                  Waiting for check to start...
                </span>
              )}
              {logs.map((entry, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-foreground-tertiary shrink-0">
                    {fmtTime(entry.ts)}
                  </span>
                  <span className={logColor[entry.type]}>{entry.text}</span>
                </div>
              ))}
              <div ref={logEndRef} />
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
