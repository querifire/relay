import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { BandwidthStatsDto } from "../types";

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx += 1;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[idx]}`;
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-surface-card border border-border rounded-card p-5">
      <div className="text-[0.75rem] text-foreground-muted">{label}</div>
      <div className="text-[1.75rem] font-semibold mt-1">{value}</div>
    </div>
  );
}

export default function BandwidthPage() {
  const [stats, setStats] = useState<BandwidthStatsDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const data = await invoke<BandwidthStatsDto>("get_bandwidth_stats");
      setStats(data);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh().catch(() => {});
    const id = setInterval(() => refresh().catch(() => {}), 2000);
    return () => clearInterval(id);
  }, [refresh]);

  const maxBytes = stats
    ? Math.max(1, ...stats.per_proxy.map((p) => p.total_bytes))
    : 1;

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Bandwidth</span>
        </div>
        <div className="flex items-center justify-between">
          <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">Bandwidth</h1>
          <div className="flex items-center gap-1.5 text-[0.75rem] text-foreground-muted">
            <span
              className="inline-block w-1.5 h-1.5 rounded-full bg-[#30D158] animate-pulse"
            />
            Live
          </div>
        </div>
      </header>

      {error && (
        <div className="mb-6 px-4 py-3 rounded-button bg-[rgba(255,59,48,0.1)] text-[#FF3B30] text-[0.8125rem]">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <StatCard
          label="Total traffic"
          value={formatBytes(stats?.total_bytes ?? 0)}
        />
        <StatCard
          label="Total requests"
          value={(stats?.total_requests ?? 0).toLocaleString()}
        />
        <StatCard
          label="Active proxies"
          value={String(stats?.per_proxy.filter((p) => p.total_requests > 0).length ?? 0)}
        />
      </div>

      <div className="bg-surface-card border border-border rounded-card p-5">
        <h3 className="text-[0.95rem] font-semibold mb-5">Per-proxy breakdown</h3>
        {!stats || stats.per_proxy.length === 0 ? (
          <p className="text-[0.8125rem] text-foreground-muted">
            No proxy instances available. Create a proxy to start tracking bandwidth.
          </p>
        ) : (
          <div className="flex flex-col gap-5">
            {stats.per_proxy.map((proxy) => {
              const barWidth = `${Math.max(2, (proxy.total_bytes / maxBytes) * 100)}%`;
              const successPct = proxy.total_requests > 0
                ? Math.round(proxy.success_rate * 100)
                : 0;
              return (
                <div key={proxy.id}>
                  <div className="flex justify-between items-baseline mb-1.5">
                    <span className="text-[0.875rem] font-medium">{proxy.name}</span>
                    <span className="text-[0.8125rem] font-mono text-foreground-muted">
                      {formatBytes(proxy.total_bytes)}
                    </span>
                  </div>

                  <div className="h-2 rounded-full bg-surface overflow-hidden mb-2">
                    <div
                      className="h-full rounded-full transition-all duration-500"
                      style={{
                        width: barWidth,
                        background: "linear-gradient(90deg, var(--color-foreground), rgba(var(--color-foreground-rgb), 0.6))",
                      }}
                    />
                  </div>

                  <div className="flex gap-4 text-[0.6875rem] text-foreground-muted">
                    <span>{proxy.total_requests.toLocaleString()} reqs</span>
                    <span>{proxy.successful_requests.toLocaleString()} ok</span>
                    <span>{successPct}% success</span>
                    <span>{proxy.avg_latency_ms}ms avg</span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
