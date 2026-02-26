import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useProxies } from "../contexts/ProxyContext";
import type { LeakTestResult } from "../types";
import CustomSelect from "../components/CustomSelect";

export default function LeakTestPage() {
  const { instances } = useProxies();
  const [selectedInstance, setSelectedInstance] = useState<string>("");
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<LeakTestResult | null>(null);

  const runningInstances = instances.filter((i) => i.status === "Running");

  const instanceOptions = useMemo(() => {
    const opts = [{ value: "", label: "Direct (no proxy)" }];
    for (const inst of runningInstances) {
      opts.push({
        value: inst.id,
        label: `${inst.name} — ${inst.bind_addr}:${inst.port}`,
      });
    }
    return opts;
  }, [runningInstances]);

  const handleRunTest = async () => {
    setTesting(true);
    setResult(null);
    try {
      const id = selectedInstance || undefined;
      const testResult = await invoke<LeakTestResult>("run_full_leak_test", {
        id: id || null,
      });
      setResult(testResult);
    } catch (err) {
      console.error("Leak test failed:", err);
    } finally {
      setTesting(false);
    }
  };

  return (
    <div>
      <header className="mb-10">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Leak Test</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Leak Test
        </h1>
        <p className="text-[0.875rem] text-foreground-muted mt-2">
          Check if your real IP or DNS servers are leaking when using a proxy.
        </p>
      </header>

      <div className="space-y-8 max-w-2xl">
        {}
        <div className="flex items-end gap-4">
          <div className="flex-1">
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Test through proxy instance
            </label>
            <CustomSelect
              options={instanceOptions}
              value={selectedInstance}
              onChange={setSelectedInstance}
              placeholder="Select instance"
            />
          </div>
          <button
            onClick={handleRunTest}
            disabled={testing}
            className="h-[2.625rem] px-6 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)] transition-all duration-200 disabled:opacity-50 shrink-0"
          >
            {testing ? "Testing..." : "Run Test"}
          </button>
        </div>

        {}
        {result && (
          <div className="space-y-6">
            {}
            <div className="bg-surface-card border border-border rounded-card p-6">
              <div className="flex items-center justify-between mb-5">
                <h2 className="text-[0.9375rem] font-semibold">IP Leak Test</h2>
                <StatusChip
                  ok={!result.ip.leak_detected}
                  labelOk="Protected"
                  labelBad="Leak Detected"
                />
              </div>
              <div className="space-y-3">
                <InfoRow label="Your Real IP" value={result.ip.real_ip ?? "Unknown"} />
                <InfoRow
                  label="Proxy IP"
                  value={result.ip.proxy_ip ?? "Not available"}
                />
                {result.ip.proxy_used && (
                  <InfoRow label="Proxy Used" value={result.ip.proxy_used} />
                )}
                <InfoRow
                  label="Status"
                  value={
                    result.ip.leak_detected
                      ? "Your real IP is visible — proxy is not masking your identity"
                      : "Your real IP is hidden behind the proxy"
                  }
                  highlight={result.ip.leak_detected ? "bad" : "ok"}
                />
              </div>
            </div>

            {}
            <div className="bg-surface-card border border-border rounded-card p-6">
              <div className="flex items-center justify-between mb-5">
                <h2 className="text-[0.9375rem] font-semibold">DNS Leak Test</h2>
                <StatusChip
                  ok={!result.dns.leak_detected}
                  labelOk="Protected"
                  labelBad="Leak Detected"
                />
              </div>
              <div className="space-y-3">
                <div className="flex justify-between text-[0.8125rem]">
                  <span className="text-foreground-muted">DNS Servers</span>
                  <div className="text-right">
                    {result.dns.dns_servers.map((s, i) => (
                      <div
                        key={i}
                        className="font-mono text-foreground bg-surface-hover px-1.5 py-0.5 rounded text-[0.75rem] mb-0.5"
                      >
                        {s}
                      </div>
                    ))}
                  </div>
                </div>
                <InfoRow
                  label="Status"
                  value={
                    result.dns.leak_detected
                      ? "System DNS is being used — enable DNS Protection in Settings"
                      : "DNS requests are encrypted"
                  }
                  highlight={result.dns.leak_detected ? "bad" : "ok"}
                />
              </div>
            </div>
          </div>
        )}

        {}
        {!result && !testing && (
          <div className="bg-surface-card border border-border rounded-card p-10 text-center">
            <div className="text-foreground-muted text-[0.875rem]">
              Click "Run Test" to check for IP and DNS leaks
            </div>
            <p className="text-foreground-tertiary text-[0.75rem] mt-2">
              Select a running proxy instance to test through, or test your
              direct connection.
            </p>
          </div>
        )}

        {testing && (
          <div className="bg-surface-card border border-border rounded-card p-10 text-center">
            <div className="text-foreground-muted text-[0.875rem]">
              Running leak tests...
            </div>
            <p className="text-foreground-tertiary text-[0.75rem] mt-2">
              This may take a few seconds.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function StatusChip({
  ok,
  labelOk,
  labelBad,
}: {
  ok: boolean;
  labelOk: string;
  labelBad: string;
}) {
  return (
    <span
      className={`inline-flex px-2.5 py-1 rounded-badge text-[0.6875rem] font-semibold ${
        ok
          ? "bg-[rgba(52,199,89,0.1)] text-[#34C759]"
          : "bg-[rgba(255,59,48,0.1)] text-[#FF3B30]"
      }`}
    >
      {ok ? labelOk : labelBad}
    </span>
  );
}

function InfoRow({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: "ok" | "bad";
}) {
  const valueColor = highlight === "ok"
    ? "text-[#34C759]"
    : highlight === "bad"
      ? "text-[#FF3B30]"
      : "text-foreground";

  return (
    <div className="flex justify-between text-[0.8125rem]">
      <span className="text-foreground-muted">{label}</span>
      <span className={`font-mono bg-surface-hover px-1.5 py-0.5 rounded text-[0.75rem] max-w-[20rem] text-right ${valueColor}`}>
        {value}
      </span>
    </div>
  );
}
