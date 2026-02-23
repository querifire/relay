import { useState } from "react";
import { useProxies } from "../contexts/ProxyContext";
import { useSettings } from "../hooks/useSettings";
import type { ProxyMode } from "../types";
import CustomSelect from "./CustomSelect";
import { COUNTRY_OPTIONS } from "../utils/countryFlags";

interface Props {
  onClose: () => void;
}

const modes: { value: ProxyMode; label: string; desc: string }[] = [
  {
    value: "Auto",
    label: "Auto",
    desc: "Automatically find & rotate upstream proxies",
  },
  {
    value: "Manual",
    label: "Manual",
    desc: "You provide an upstream proxy address",
  },
];

const PROTOCOL_OPTIONS = [
  { value: "Http", label: "HTTP" },
  { value: "Https", label: "HTTPS" },
  { value: "Socks4", label: "SOCKS4" },
  { value: "Socks5", label: "SOCKS5" },
];

const countrySelectOptions = [
  { value: "", label: "No country" },
  ...COUNTRY_OPTIONS.map((c) => ({ value: c.value, label: c.label })),
];

export default function AddProxyDialog({ onClose }: Props) {
  const { instances, createInstance, startInstance } = useProxies();
  const { settings } = useSettings();

  const [name, setName] = useState("");
  const [bindAddr, setBindAddr] = useState(
    settings?.default_bind ?? "127.0.0.1",
  );
  const [port, setPort] = useState(settings?.default_port ?? 9051);
  const [mode, setMode] = useState<ProxyMode>("Auto");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Upstream fields (Manual mode)
  const [upstreamHost, setUpstreamHost] = useState("");
  const [upstreamPort, setUpstreamPort] = useState(8080);
  const [upstreamProtocol, setUpstreamProtocol] = useState("Http");

  // Country
  const [country, setCountry] = useState("");

  const portConflict = instances.find(
    (i) => i.port === port && i.bind_addr === bindAddr,
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (portConflict) return;
    setError(null);
    setBusy(true);
    try {
      // Build name: prepend country code if selected
      const countryFlag = country
        ? COUNTRY_OPTIONS.find((c) => c.value === country)
        : null;
      const displayName =
        (countryFlag ? `${countryFlag.flag} ` : "") +
        (name || `Proxy :${port}`);

      await createInstance(displayName, bindAddr, port, mode);

      // For Manual mode with upstream, immediately start with upstream params
      if (mode === "Manual" && upstreamHost.trim()) {
        // Need the newly created instance ID — find by matching port/bindAddr
        // The context will refresh; startInstance accepts upstream params
        const allInstances = instances;
        // Use a small delay so the backend has the new instance
        setTimeout(() => {
          // Find the instance that was just created
          const newest = allInstances.find(
            (i) => i.port === port && i.bind_addr === bindAddr,
          );
          if (newest) {
            startInstance(newest.id, {
              host: upstreamHost.trim(),
              port: upstreamPort,
              protocol: upstreamProtocol,
            });
          }
        }, 500);
      }

      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/40" onClick={onClose} />

      {/* Dialog */}
      <div className="relative w-full max-w-md mx-4 bg-surface-card border border-border rounded-card p-6 shadow-float">
        {/* Title row */}
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-[1rem] font-semibold tracking-[-0.01em]">
            New Proxy Instance
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-button text-foreground-muted hover:text-foreground hover:bg-surface-hover transition-colors"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-5">
          {/* Name */}
          <div>
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={`Proxy :${port}`}
              className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors placeholder:text-foreground-tertiary"
            />
          </div>

          {/* Bind Address */}
          <div>
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Bind Address
            </label>
            <input
              type="text"
              value={bindAddr}
              onChange={(e) => setBindAddr(e.target.value)}
              className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
            />
          </div>

          {/* Port */}
          <div>
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Port
            </label>
            <input
              type="number"
              value={port}
              onChange={(e) => setPort(Number(e.target.value))}
              min={1}
              max={65535}
              className={`w-full px-3 py-2.5 text-[0.875rem] bg-surface border rounded-button outline-none transition-colors font-mono ${
                portConflict
                  ? "border-[#FF3B30] focus:border-[#FF3B30]"
                  : "border-border focus:border-border-focus"
              }`}
            />
            {portConflict && (
              <p className="text-[0.75rem] text-[#FF3B30] mt-1.5">
                Port {port} is already used by "{portConflict.name}"
              </p>
            )}
          </div>

          {/* Mode — card selector */}
          <div>
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Mode
            </label>
            <div className="grid grid-cols-2 gap-3">
              {modes.map((m) => (
                <button
                  key={m.value}
                  type="button"
                  onClick={() => setMode(m.value)}
                  className={`text-left px-3 py-3 rounded-button border transition-all ${
                    mode === m.value
                      ? "border-foreground bg-surface-hover"
                      : "border-border hover:border-border-focus"
                  }`}
                >
                  <span className="block text-[0.875rem] font-medium">
                    {m.label}
                  </span>
                  <span className="block text-[0.6875rem] text-foreground-muted mt-1 leading-tight">
                    {m.desc}
                  </span>
                </button>
              ))}
            </div>
          </div>

          {/* Upstream fields (Manual mode only) */}
          {mode === "Manual" && (
            <div className="space-y-4 p-4 bg-surface rounded-button border border-border">
              <div className="text-[0.75rem] font-medium text-foreground-muted mb-1">
                Upstream Proxy
              </div>

              {/* Upstream Host */}
              <div>
                <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                  Host
                </label>
                <input
                  type="text"
                  value={upstreamHost}
                  onChange={(e) => setUpstreamHost(e.target.value)}
                  placeholder="proxy.example.com"
                  className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors placeholder:text-foreground-tertiary font-mono"
                />
              </div>

              {/* Upstream Port */}
              <div>
                <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                  Port
                </label>
                <input
                  type="number"
                  value={upstreamPort}
                  onChange={(e) => setUpstreamPort(Number(e.target.value))}
                  min={1}
                  max={65535}
                  className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
                />
              </div>

              {/* Upstream Protocol */}
              <div>
                <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                  Protocol
                </label>
                <CustomSelect
                  options={PROTOCOL_OPTIONS}
                  value={upstreamProtocol}
                  onChange={setUpstreamProtocol}
                  placeholder="Select protocol"
                />
              </div>
            </div>
          )}

          {/* Country */}
          <div>
            <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
              Country
            </label>
            <CustomSelect
              options={countrySelectOptions}
              value={country}
              onChange={setCountry}
              placeholder="No country"
            />
          </div>

          {/* Error */}
          {error && (
            <p className="text-[0.75rem] text-[#FF3B30] bg-[rgba(255,59,48,0.1)] px-4 py-3 rounded-button">
              {error}
            </p>
          )}

          {/* Buttons */}
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={busy}
              className="h-9 px-4 rounded-button text-[0.8125rem] font-medium border border-border hover:bg-surface-hover transition-all disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={busy || !!portConflict}
              className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px transition-all disabled:opacity-50"
            >
              {busy ? "Creating..." : "Create"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
