import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { useSettings } from "../hooks/useSettings";
import type { TorConfig, BridgeType } from "../types";
import CustomSelect from "../components/CustomSelect";

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={onChange}
      className={`w-9 h-5 rounded-full relative cursor-pointer transition-colors duration-300 flex-shrink-0 ${
        checked ? "bg-foreground" : "bg-border"
      }`}
    >
      <span
        className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full shadow-sm transition-transform duration-300 ${
          checked ? "bg-white dark:bg-[#1C1C1E]" : "bg-white"
        }`}
        style={{ transform: checked ? "translateX(1rem)" : "translateX(0)" }}
      />
    </button>
  );
}

function Card({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={`bg-surface-card border border-border rounded-card shadow-card p-5 ${className}`}>
      {children}
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[0.6875rem] font-semibold text-foreground-tertiary uppercase tracking-[0.05em] mb-3">
      {children}
    </div>
  );
}

const BRIDGE_TYPE_OPTIONS: { value: BridgeType; label: string }[] = [
  { value: "Obfs4", label: "obfs4 (recommended)" },
  { value: "MeekAzure", label: "meek-azure" },
  { value: "Snowflake", label: "Snowflake" },
  { value: "WebTunnel", label: "WebTunnel" },
  { value: "Custom", label: "Custom" },
];

const DEFAULT_TOR_CONFIG: TorConfig = {
  binary_path: null,
  socks_port: 9050,
  use_bridges: false,
  bridge_type: "Obfs4",
  custom_bridges: [],
  exit_nodes: null,
  entry_nodes: null,
  exclude_nodes: null,
  strict_nodes: false,
  custom_torrc: null,
};

export default function TorPage() {
  const { settings, loading, save } = useSettings();
  const [form, setForm] = useState<TorConfig>(DEFAULT_TOR_CONFIG);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [bridgesText, setBridgesText] = useState("");

  useEffect(() => {
    if (settings) {
      const cfg = settings.tor_config ?? DEFAULT_TOR_CONFIG;
      setForm(cfg);
      setBridgesText((cfg.custom_bridges ?? []).join("\n"));
    }
  }, [settings]);

  if (loading || !settings) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-[0.875rem] text-foreground-muted">Loading…</p>
      </div>
    );
  }

  const update = (patch: Partial<TorConfig>) =>
    setForm((prev) => ({ ...prev, ...patch }));

  const handleSave = async () => {
    setSaving(true);
    try {
      const bridges = bridgesText
        .split("\n")
        .map((l) => l.trim())
        .filter(Boolean);
      const newTorConfig: TorConfig = { ...form, custom_bridges: bridges };
      await save({ ...settings, tor_config: newTorConfig });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error("Failed to save Tor settings:", err);
    } finally {
      setSaving(false);
    }
  };

  const torNotFound = !form.binary_path;

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Tor</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">Tor</h1>
      </header>

      {}
      <SectionLabel>General</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">Tor Binary Path</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-4">
            Full path to the <code className="font-mono">tor</code> executable.
          </p>

          {torNotFound && (
            <div className="mb-4 px-3 py-2.5 rounded-button border border-[rgba(255,159,10,0.3)] bg-[rgba(255,159,10,0.08)] text-[0.75rem] text-[#FF9F0A] leading-relaxed">
              Tor binary not found. Install it via the{" "}
              <Link to="/plugins" className="underline font-medium">
                Tor Downloader
              </Link>{" "}
              plugin on the Plugins page.
            </div>
          )}

          <input
            type="text"
            value={form.binary_path ?? ""}
            onChange={(e) => update({ binary_path: e.target.value || null })}
            placeholder={
              navigator.platform.startsWith("Win")
                ? "C:\\Tor\\tor.exe"
                : "/usr/bin/tor"
            }
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors placeholder:text-foreground-muted/50 font-mono"
          />
        </Card>

        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">SOCKS Port</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-4">
            Local SOCKS5 port exposed by the Tor process.
          </p>
          <input
            type="number"
            value={form.socks_port}
            onChange={(e) => update({ socks_port: Number(e.target.value) })}
            min={1}
            max={65535}
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
        </Card>
      </div>

      {}
      <SectionLabel>Bridges</SectionLabel>
      <div className="grid grid-cols-1 gap-4 mb-8">
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div>
              <h3 className="text-[0.9375rem] font-semibold">Use Bridges</h3>
              <p className="text-[0.75rem] text-foreground-muted mt-0.5">
                Use bridge relays to bypass Tor censorship.
              </p>
            </div>
            <Toggle
              checked={form.use_bridges}
              onChange={() => update({ use_bridges: !form.use_bridges })}
            />
          </div>

          {form.use_bridges && (
            <div className="pt-4 border-t border-border space-y-4">
              <div>
                <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                  Transport Type
                </label>
                <CustomSelect
                  options={BRIDGE_TYPE_OPTIONS}
                  value={form.bridge_type}
                  onChange={(v) => update({ bridge_type: v as BridgeType })}
                  placeholder="Select transport"
                />
              </div>

              <div>
                <div className="flex items-center justify-between mb-2">
                  <label className="text-[0.75rem] font-medium text-foreground-muted">
                    Bridge Lines
                  </label>
                  <a
                    href="https://bridges.torproject.org"
                    target="_blank"
                    rel="noreferrer"
                    className="text-[0.6875rem] text-foreground-muted hover:text-foreground transition-colors underline"
                  >
                    Request Bridges ↗
                  </a>
                </div>
                <textarea
                  value={bridgesText}
                  onChange={(e) => setBridgesText(e.target.value)}
                  rows={5}
                  placeholder={"obfs4 1.2.3.4:1234 FINGERPRINT cert=... iat-mode=0\nobfs4 5.6.7.8:5678 FINGERPRINT cert=..."}
                  className="w-full px-3 py-2.5 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono resize-y"
                />
                <p className="text-[0.6875rem] text-foreground-muted mt-1.5">
                  One bridge line per row. Obtain bridges from{" "}
                  <a
                    href="https://bridges.torproject.org"
                    target="_blank"
                    rel="noreferrer"
                    className="underline"
                  >
                    bridges.torproject.org
                  </a>
                  .
                </p>
              </div>
            </div>
          )}
        </Card>
      </div>

      {}
      <SectionLabel>Advanced</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">Exit Nodes</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-3">
            Restrict exit nodes to specific countries (e.g.{" "}
            <code className="font-mono">{"{US,DE}"}</code>).
          </p>
          <input
            type="text"
            value={form.exit_nodes ?? ""}
            onChange={(e) => update({ exit_nodes: e.target.value || null })}
            placeholder="{US,DE,NL}"
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
        </Card>

        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">Entry Nodes</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-3">
            Restrict entry (guard) nodes to specific countries.
          </p>
          <input
            type="text"
            value={form.entry_nodes ?? ""}
            onChange={(e) => update({ entry_nodes: e.target.value || null })}
            placeholder="{US,DE}"
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
        </Card>

        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">Exclude Nodes</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-3">
            Exclude specific countries from being used as nodes.
          </p>
          <input
            type="text"
            value={form.exclude_nodes ?? ""}
            onChange={(e) => update({ exclude_nodes: e.target.value || null })}
            placeholder="{RU,CN}"
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
        </Card>

        <Card>
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-[0.9375rem] font-semibold">Strict Nodes</h3>
              <p className="text-[0.75rem] text-foreground-muted mt-0.5">
                Fail rather than use non-allowed nodes.
              </p>
            </div>
            <Toggle
              checked={form.strict_nodes}
              onChange={() => update({ strict_nodes: !form.strict_nodes })}
            />
          </div>
        </Card>
      </div>

      {}
      <SectionLabel>Custom torrc</SectionLabel>
      <div className="mb-8">
        <Card>
          <h3 className="text-[0.9375rem] font-semibold mb-1">Additional torrc Directives</h3>
          <p className="text-[0.75rem] text-foreground-muted mb-3">
            Raw torrc lines appended after all generated configuration. Use with caution.
          </p>
          <textarea
            value={form.custom_torrc ?? ""}
            onChange={(e) => update({ custom_torrc: e.target.value || null })}
            rows={6}
            placeholder={"# Example:\nMaxCircuitDirtiness 60\nNewCircuitPeriod 30"}
            className="w-full px-3 py-2.5 text-[0.8125rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono resize-y"
          />
        </Card>
      </div>

      {}
      <div className="flex justify-end">
        <button
          onClick={handleSave}
          disabled={saving}
          className="h-10 px-8 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)] transition-all duration-200 disabled:opacity-50"
        >
          {saving ? "Saving…" : saved ? "Saved!" : "Save Settings"}
        </button>
      </div>
    </div>
  );
}
