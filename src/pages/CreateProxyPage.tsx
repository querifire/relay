import { useState, useMemo, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useProxies } from "../contexts/ProxyContext";
import { useSettings } from "../hooks/useSettings";
import { fetchProxyLists } from "../hooks/useProxyLists";
import type { ProxyMode, ProxyProtocol, ProxyListConfig, Proxy } from "../types";
import CustomSelect from "../components/CustomSelect";

const MODE_OPTIONS: { value: ProxyMode; label: string; desc: string }[] = [
  {
    value: "Auto",
    label: "Auto",
    desc: "Automatically find & rotate upstream proxies",
  },
  {
    value: "Manual",
    label: "Manual",
    desc: "Provide an upstream proxy address manually",
  },
  {
    value: "Tor",
    label: "Tor",
    desc: "Route traffic through a dedicated Tor client instance",
  },
];

const PROTOCOL_OPTIONS = [
  { value: "Http", label: "HTTP/S" },
  { value: "Socks5", label: "SOCKS5" },
  { value: "Socks4", label: "SOCKS4" },
];

const UPSTREAM_PROTOCOL_OPTIONS = [
  { value: "Http", label: "HTTP" },
  { value: "Https", label: "HTTPS" },
  { value: "Socks4", label: "SOCKS4" },
  { value: "Socks5", label: "SOCKS5" },
];

export default function CreateProxyPage() {
  const navigate = useNavigate();
  const { instances, createInstance, startInstance } = useProxies();
  const { settings } = useSettings();

  const [name, setName] = useState("");
  const [bindAddr, setBindAddr] = useState(
    settings?.default_bind ?? "127.0.0.1",
  );
  const [port, setPort] = useState(settings?.default_port ?? 9051);
  const [mode, setMode] = useState<ProxyMode>("Auto");
  const [localProtocol, setLocalProtocol] = useState<ProxyProtocol>("Socks5");

  const [upstreamHost, setUpstreamHost] = useState("");
  const [upstreamPort, setUpstreamPort] = useState(8080);
  const [upstreamProtocol, setUpstreamProtocol] = useState("Socks5");

  const [authUsername, setAuthUsername] = useState("");
  const [authPassword, setAuthPassword] = useState("");

  const [customLists, setCustomLists] = useState<ProxyListConfig[]>([]);
  useEffect(() => {
    fetchProxyLists().then(setCustomLists);
  }, []);

  const proxyListOptions = useMemo(() => {
    const options = [{ value: "default", label: "Default (Built-in)" }];
    for (const s of customLists) {
      options.push({ value: s.id, label: s.name });
    }
    return options;
  }, [customLists]);

  const [autoRotate, setAutoRotate] = useState(false);
  const [autoRotateMinutes, setAutoRotateMinutes] = useState(5);
  const [proxyList, setProxyList] = useState("default");

  const [chainEnabled, setChainEnabled] = useState(false);
  const [chainProxies, setChainProxies] = useState<Proxy[]>([]);
  const [chainHost, setChainHost] = useState("");
  const [chainPort, setChainPort] = useState(1080);
  const [chainProtocol, setChainProtocol] = useState<ProxyProtocol>("Socks5");

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const portConflict = instances.find(
    (i) => i.port === port && i.bind_addr === bindAddr,
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (portConflict) return;
    setError(null);
    setBusy(true);

    try {
      const displayName = name || `Proxy :${port}`;

      const proxyChain = chainEnabled && chainProxies.length > 0
        ? { enabled: true, proxies: chainProxies }
        : null;

      await createInstance(
        displayName,
        bindAddr,
        port,
        mode,
        localProtocol,
        authUsername || null,
        authPassword || null,
        autoRotate,
        proxyList,
        autoRotate ? autoRotateMinutes : null,
        proxyChain,
      );

      if (mode === "Manual" && upstreamHost.trim()) {
        setTimeout(() => {
          const newest = [...instances].reverse().find(
            (i) => i.port === port && i.bind_addr === bindAddr,
          );
          if (newest) {
            startInstance(newest.id, {
              host: upstreamHost.trim(),
              port: upstreamPort,
              protocol: upstreamProtocol,
            });
          }
        }, 600);
      }

      navigate("/");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="max-w-4xl">
      {}
      <header className="mb-10">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <button
            onClick={() => navigate("/")}
            className="hover:text-foreground transition-colors"
          >
            Proxies
          </button>
          <span>/</span>
          <span className="text-foreground">Create New</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Create Proxy Instance
        </h1>
        <p className="text-[0.875rem] text-foreground-muted mt-2">
          Configure a new local proxy server with your preferred settings.
        </p>
      </header>

      <form onSubmit={handleSubmit} className="space-y-8">
        {}
        <section>
          <h2 className="text-[0.9375rem] font-semibold tracking-[-0.01em] mb-1">
            Basic Configuration
          </h2>
          <p className="text-[0.75rem] text-foreground-muted mb-5">
            Set the name, address, and operating mode for your proxy.
          </p>

          <div className="space-y-5">
            {}
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

            {}
            <div className="grid grid-cols-2 gap-4">
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
            </div>

            {}
            <div>
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Mode
              </label>
              <div className="grid grid-cols-3 gap-3">
                {MODE_OPTIONS.map((m) => (
                  <button
                    key={m.value}
                    type="button"
                    onClick={() => setMode(m.value)}
                    className={`text-left px-4 py-3.5 rounded-button border transition-all cursor-pointer ${
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

            {}
            {mode !== "Tor" && (
              <div>
                <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                  Protocol Type
                </label>
                <p className="text-[0.6875rem] text-foreground-tertiary mb-2">
                  Protocol used by the local proxy server that applications connect to.
                </p>
                <div className="w-48">
                  <CustomSelect
                    options={PROTOCOL_OPTIONS}
                    value={localProtocol}
                    onChange={(v) => setLocalProtocol(v as ProxyProtocol)}
                    placeholder="Select protocol"
                  />
                </div>
              </div>
            )}
          </div>
        </section>

        {}
        <div className="border-t border-border" />

        {}
        <section>
          <h2 className="text-[0.9375rem] font-semibold tracking-[-0.01em] mb-1">
            Network & Security
          </h2>
          <p className="text-[0.75rem] text-foreground-muted mb-5">
            {mode === "Manual"
              ? "Configure the upstream proxy and optional authentication."
              : mode === "Tor"
                ? "Tor will be launched as a SOCKS5 proxy directly on the port above."
                : "Set optional authentication for the local proxy server."}
          </p>

          <div className="space-y-5">
            {}
            {mode === "Manual" && (
              <div className="p-4 bg-surface-hover/50 rounded-card border border-border space-y-4">
                <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.04em]">
                  Upstream Proxy
                </div>

                <div className="grid grid-cols-2 gap-4">
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
                </div>

                <div>
                  <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                    Upstream Protocol
                  </label>
                  <div className="w-48">
                    <CustomSelect
                      options={UPSTREAM_PROTOCOL_OPTIONS}
                      value={upstreamProtocol}
                      onChange={setUpstreamProtocol}
                      placeholder="Select protocol"
                    />
                  </div>
                </div>
              </div>
            )}

            {}
            {mode === "Tor" && (
              <div className="p-4 bg-surface-hover/50 rounded-card border border-border space-y-3">
                <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.04em]">
                  Tor
                </div>
                <div className="flex items-center gap-2 text-[0.6875rem] text-foreground-tertiary">
                  <span className="font-mono bg-surface px-1.5 py-0.5 rounded border border-border">App</span>
                  <span>→</span>
                  <span className="font-mono bg-surface px-1.5 py-0.5 rounded border border-border">Tor SOCKS5 :{port}</span>
                  <span>→</span>
                  <span className="font-mono bg-surface px-1.5 py-0.5 rounded border border-border">Tor Network</span>
                </div>
                <p className="text-[0.6875rem] text-foreground-tertiary leading-relaxed">
                  A dedicated Tor process will listen on <span className="font-mono">{bindAddr}:{port}</span> as a SOCKS5 proxy.
                  Each Tor instance runs its own client — use different ports to run multiple.
                </p>
              </div>
            )}

            {}
            {mode !== "Tor" && <div>
              <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.04em] mb-3">
                Authentication (optional)
              </div>
              <p className="text-[0.6875rem] text-foreground-tertiary mb-3">
                Require credentials to connect to the local proxy server.
              </p>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                    Username
                  </label>
                  <input
                    type="text"
                    value={authUsername}
                    onChange={(e) => setAuthUsername(e.target.value)}
                    placeholder="Leave empty to disable"
                    className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors placeholder:text-foreground-tertiary"
                  />
                </div>
                <div>
                  <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                    Password
                  </label>
                  <input
                    type="password"
                    value={authPassword}
                    onChange={(e) => setAuthPassword(e.target.value)}
                    placeholder="Leave empty to disable"
                    className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors placeholder:text-foreground-tertiary"
                  />
                </div>
              </div>
            </div>}
          </div>
        </section>

        {}
        {mode !== "Tor" && <div className="border-t border-border" />}

        {}
        {mode !== "Tor" && <section>
          <h2 className="text-[0.9375rem] font-semibold tracking-[-0.01em] mb-1">
            Advanced Options
          </h2>
          <p className="text-[0.75rem] text-foreground-muted mb-5">
            Fine-tune rotation and proxy list behavior.
          </p>

          <div className="space-y-5">
            {}
            {mode === "Auto" && (
              <div className="p-4 bg-surface-hover/50 rounded-card border border-border space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-[0.875rem] font-medium">
                      Auto-Rotation
                    </div>
                    <p className="text-[0.6875rem] text-foreground-muted mt-0.5">
                      Periodically re-test and switch to the fastest available
                      proxy.
                    </p>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={autoRotate}
                    onClick={() => setAutoRotate(!autoRotate)}
                    className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                      autoRotate ? "bg-foreground" : "bg-border"
                    }`}
                  >
                    <span
                      className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-surface shadow ring-0 transition duration-200 ease-in-out ${
                        autoRotate ? "translate-x-5" : "translate-x-0"
                      }`}
                    />
                  </button>
                </div>
                {autoRotate && (
                  <div>
                    <label className="block text-[0.75rem] font-medium text-foreground-muted mb-1.5">
                      Rotate every (minutes)
                    </label>
                    <input
                      type="number"
                      value={autoRotateMinutes}
                      onChange={(e) =>
                        setAutoRotateMinutes(Math.max(1, Number(e.target.value) || 1))
                      }
                      min={1}
                      className="w-full max-w-[10rem] px-3 py-2 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
                    />
                  </div>
                )}
              </div>
            )}

            {}
            <div>
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Proxy List
              </label>
              <p className="text-[0.6875rem] text-foreground-tertiary mb-2">
                Source list used for auto-discovery.
              </p>
              <div className="w-48">
                <CustomSelect
                  options={proxyListOptions}
                  value={proxyList}
                  onChange={setProxyList}
                  placeholder="Select list"
                />
              </div>
            </div>

            {}
            <div className="p-4 bg-surface-hover/50 rounded-card border border-border space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[0.875rem] font-medium">
                    Proxy Chain (Multi-hop)
                  </div>
                  <p className="text-[0.6875rem] text-foreground-muted mt-0.5">
                    Route traffic through multiple proxies for increased anonymity.
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={chainEnabled}
                  onClick={() => setChainEnabled(!chainEnabled)}
                  className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                    chainEnabled ? "bg-foreground" : "bg-border"
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-surface shadow ring-0 transition duration-200 ease-in-out ${
                      chainEnabled ? "translate-x-5" : "translate-x-0"
                    }`}
                  />
                </button>
              </div>
              {chainEnabled && (
                <div className="space-y-3">
                  {chainProxies.length > 0 && (
                    <div className="space-y-1.5">
                      <div className="text-[0.6875rem] text-foreground-tertiary font-semibold uppercase">
                        Chain ({chainProxies.length} hop{chainProxies.length > 1 ? "s" : ""})
                      </div>
                      {chainProxies.map((p, idx) => (
                        <div key={idx} className="flex items-center gap-2 text-[0.8125rem]">
                          <span className="w-5 h-5 bg-surface rounded-full grid place-items-center text-[0.625rem] font-semibold text-foreground-muted">
                            {idx + 1}
                          </span>
                          <span className="font-mono text-[0.75rem] bg-surface px-1.5 py-0.5 rounded border border-border">
                            {p.protocol}:
                          </span>
                          {idx < chainProxies.length - 1 && <span className="text-foreground-tertiary">→</span>}
                          <button
                            type="button"
                            onClick={() => setChainProxies(chainProxies.filter((_, i) => i !== idx))}
                            className="text-[0.625rem] text-[#FF3B30] hover:underline ml-auto"
                          >
                            Remove
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                  <div className="grid grid-cols-[1fr_80px_140px_auto] gap-3 items-end">
                    <div>
                      <label className="block text-[0.625rem] text-foreground-muted mb-1">Host</label>
                      <input
                        type="text"
                        value={chainHost}
                        onChange={(e) => setChainHost(e.target.value)}
                        placeholder="proxy.example.com"
                        className="w-full px-2 py-1.5 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
                      />
                    </div>
                    <div>
                      <label className="block text-[0.625rem] text-foreground-muted mb-1">Port</label>
                      <input
                        type="number"
                        value={chainPort}
                        onChange={(e) => setChainPort(Number(e.target.value))}
                        min={1}
                        max={65535}
                        className="w-full px-2 py-1.5 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
                      />
                    </div>
                    <div>
                      <label className="block text-[0.625rem] text-foreground-muted mb-1">Protocol</label>
                      <CustomSelect
                        options={[
                          { value: "Socks5", label: "SOCKS5" },
                          { value: "Socks4", label: "SOCKS4" },
                          { value: "Http", label: "HTTP" },
                        ]}
                        value={chainProtocol}
                        onChange={(v) => setChainProtocol(v as ProxyProtocol)}
                        placeholder="Protocol"
                      />
                    </div>
                    <button
                      type="button"
                      onClick={() => {
                        if (chainHost.trim()) {
                          setChainProxies([...chainProxies, { host: chainHost.trim(), port: chainPort, protocol: chainProtocol }]);
                          setChainHost("");
                        }
                      }}
                      disabled={!chainHost.trim()}
                      className="h-[2.125rem] px-3 rounded-button text-[0.75rem] font-medium bg-foreground text-surface hover:opacity-80 transition-all disabled:opacity-50"
                    >
                      Add
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </section>}

        {}
        {error && (
          <div className="text-[0.75rem] text-[#FF3B30] bg-[rgba(255,59,48,0.1)] px-4 py-3 rounded-button">
            {error}
          </div>
        )}

        {}
        <div className="flex justify-end gap-3 pt-4 border-t border-border">
          <button
            type="button"
            onClick={() => navigate("/")}
            disabled={busy}
            className="h-10 px-5 rounded-button text-[0.8125rem] font-medium border border-border hover:bg-surface-hover transition-all cursor-pointer disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy || !!portConflict}
            className="h-10 px-5 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)] transition-all cursor-pointer disabled:opacity-50"
          >
            {busy ? "Creating…" : "Create Proxy Instance"}
          </button>
        </div>
      </form>
    </div>
  );
}
