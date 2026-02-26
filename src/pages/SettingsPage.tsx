import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "../hooks/useSettings";
import { enable as autostartEnable, disable as autostartDisable, isEnabled as autostartIsEnabled } from "@tauri-apps/plugin-autostart";
import type { AppSettings, KillSwitchConfig as KillSwitchStatus, NotificationSettings, Profile, SaveProfileRequest } from "../types";
import CustomSelect from "../components/CustomSelect";

function Toggle({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={onChange}
      className={`w-9 h-5 rounded-full relative cursor-pointer transition-colors duration-300 flex-shrink-0 disabled:opacity-50 ${
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

function Card({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`bg-surface-card border border-border rounded-card shadow-card p-5 ${className}`}
    >
      {children}
    </div>
  );
}

function CardTitle({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return (
    <h3 className={`text-[0.9375rem] font-semibold tracking-[-0.01em] ${className}`}>
      {children}
    </h3>
  );
}

function CardDescription({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[0.75rem] text-foreground-muted leading-relaxed mt-2">
      {children}
    </p>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[0.6875rem] font-semibold text-foreground-tertiary uppercase tracking-[0.05em] mb-3">
      {children}
    </div>
  );
}

export default function SettingsPage() {
  const { settings, loading, save } = useSettings();
  const [form, setForm] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [appAutostart, setAppAutostart] = useState(false);
  const [autostartLoading, setAutostartLoading] = useState(false);
  const [killSwitchStatus, setKillSwitchStatus] = useState<KillSwitchStatus | null>(null);
  const [tlsHash, setTlsHash] = useState<string>("");

  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [profileName, setProfileName] = useState("");
  const [profileDesc, setProfileDesc] = useState("");
  const [profileSaving, setProfileSaving] = useState(false);
  const [loadingProfileId, setLoadingProfileId] = useState<string | null>(null);

  const [exporting, setExporting] = useState(false);
  const [exportedPath, setExportedPath] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState(false);
  const importFileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (settings) setForm({ ...settings });
  }, [settings]);

  useEffect(() => {
    autostartIsEnabled()
      .then(setAppAutostart)
      .catch(() => {});
  }, []);

  useEffect(() => {
    invoke<KillSwitchStatus>("get_kill_switch_status").then(setKillSwitchStatus).catch(() => {});
    invoke<string>("get_tls_fingerprint_hash").then(setTlsHash).catch(() => {});
    invoke<Profile[]>("list_profiles").then(setProfiles).catch(() => {});
  }, []);

  const handleAutostartToggle = async () => {
    setAutostartLoading(true);
    try {
      if (appAutostart) {
        await autostartDisable();
        setAppAutostart(false);
      } else {
        await autostartEnable();
        setAppAutostart(true);
      }
    } catch (err) {
      console.error("Failed to toggle autostart:", err);
    } finally {
      setAutostartLoading(false);
    }
  };

  if (loading || !form) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-[0.875rem] text-foreground-muted">
          Loading settings...
        </p>
      </div>
    );
  }

  const handleSave = async () => {
    setSaving(true);
    try {
      await save(form);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error("Failed to save settings:", err);
    } finally {
      setSaving(false);
    }
  };

  const update = (patch: Partial<AppSettings>) => {
    setForm((prev) => (prev ? { ...prev, ...patch } : prev));
  };

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Settings</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Settings
        </h1>
      </header>

      {}
      <SectionLabel>General</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 mb-8">
        <Card>
          <CardTitle className="mb-4">Default Port</CardTitle>
          <input
            type="number"
            value={form.default_port}
            onChange={(e) =>
              update({ default_port: Number(e.target.value) })
            }
            min={1}
            max={65535}
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
          <CardDescription>
            Port for the local proxy server (1–65535)
          </CardDescription>
        </Card>

        <Card>
          <CardTitle className="mb-4">Bind Address</CardTitle>
          <input
            type="text"
            value={form.default_bind}
            onChange={(e) => update({ default_bind: e.target.value })}
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
          <CardDescription>
            Network interface address to bind to
          </CardDescription>
        </Card>

        <Card>
          <CardTitle className="mb-4">Concurrency</CardTitle>
          <input
            type="number"
            value={form.concurrency}
            onChange={(e) =>
              update({ concurrency: Number(e.target.value) })
            }
            min={1}
            max={500}
            className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors font-mono"
          />
          <CardDescription>
            Number of concurrent proxy tests (1–500)
          </CardDescription>
        </Card>
      </div>

      {}
      <SectionLabel>Privacy & Security</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        {}
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-[0.625rem] bg-[#E89E6B]/15 flex items-center justify-center flex-shrink-0">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" fill="#E89E6B"/>
                </svg>
              </div>
              <CardTitle>DNS Protection</CardTitle>
            </div>
            <Toggle
              checked={form.dns_protection?.enabled ?? false}
              onChange={() =>
                update({
                  dns_protection: {
                    ...form.dns_protection,
                    enabled: !form.dns_protection?.enabled,
                  },
                })
              }
            />
          </div>
          <CardDescription>
            Encrypt DNS queries using DNS-over-HTTPS to prevent ISP snooping.
          </CardDescription>
          {form.dns_protection?.enabled && (
            <div className="mt-4 pt-4 border-t border-border">
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                DoH Server
              </label>
              <CustomSelect
                options={[
                  { value: "https://cloudflare-dns.com/dns-query", label: "Cloudflare (1.1.1.1)" },
                  { value: "https://dns.google/dns-query", label: "Google (8.8.8.8)" },
                  { value: "https://dns.quad9.net:5053/dns-query", label: "Quad9 (9.9.9.9)" },
                ]}
                value={form.dns_protection?.primary_server ?? "https://cloudflare-dns.com/dns-query"}
                onChange={(v) =>
                  update({
                    dns_protection: {
                      ...form.dns_protection,
                      primary_server: v,
                    },
                  })
                }
                placeholder="Select DoH server"
              />
            </div>
          )}
        </Card>

        {}
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-[0.625rem] bg-[#E89E6B]/15 flex items-center justify-center flex-shrink-0">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z" fill="#E89E6B"/>
                </svg>
              </div>
              <CardTitle>Kill-Switch</CardTitle>
            </div>
            <Toggle
              checked={form.kill_switch?.enabled ?? false}
              onChange={() =>
                update({
                  kill_switch: {
                    ...form.kill_switch,
                    enabled: !form.kill_switch?.enabled,
                  },
                })
              }
            />
          </div>
          <CardDescription>
            Block all traffic if the proxy connection drops to prevent IP leaks.
          </CardDescription>
          {killSwitchStatus?.active && (
            <div className="mt-3 flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-[#FF9F0A] animate-pulse" />
              <span className="text-[0.6875rem] text-[#FF9F0A] font-medium">
                Active — traffic is restricted
              </span>
            </div>
          )}
        </Card>

        {}
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-[0.625rem] bg-[#E89E6B]/15 flex items-center justify-center flex-shrink-0">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z" fill="#E89E6B"/>
                </svg>
              </div>
              <CardTitle>TLS Fingerprint</CardTitle>
            </div>
            <Toggle
              checked={form.tls_fingerprint?.enabled ?? false}
              onChange={() =>
                update({
                  tls_fingerprint: {
                    ...form.tls_fingerprint,
                    enabled: !form.tls_fingerprint?.enabled,
                  },
                })
              }
            />
          </div>
          <CardDescription>
            Randomize the TLS fingerprint to prevent DPI identification.
          </CardDescription>
          {form.tls_fingerprint?.enabled && (
            <div className="mt-4 pt-4 border-t border-border">
              <label className="block text-[0.75rem] font-medium text-foreground-muted mb-2">
                Preset
              </label>
              <CustomSelect
                options={[
                  { value: "Random", label: "Random" },
                  { value: "Chrome", label: "Chrome" },
                  { value: "Firefox", label: "Firefox" },
                  { value: "Safari", label: "Safari" },
                  { value: "Default", label: "Default (System)" },
                ]}
                value={form.tls_fingerprint?.preset ?? "Default"}
                onChange={(v) =>
                  update({
                    tls_fingerprint: {
                      ...form.tls_fingerprint,
                      preset: v as AppSettings["tls_fingerprint"]["preset"],
                    },
                  })
                }
                placeholder="Select preset"
              />
              {tlsHash && (
                <p className="text-[0.6875rem] text-foreground-tertiary mt-2.5 font-mono truncate">
                  Fingerprint: {tlsHash}
                </p>
              )}
            </div>
          )}
        </Card>

        {}
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-[0.625rem] bg-[#E89E6B]/15 flex items-center justify-center flex-shrink-0">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M13 3h-2v10h2V3zm4.83 2.17l-1.42 1.42A6.92 6.92 0 0119 12c0 3.87-3.13 7-7 7s-7-3.13-7-7c0-2.27 1.08-4.28 2.59-5.41L6.17 5.17A8.932 8.932 0 003 12a9 9 0 0018 0c0-2.74-1.23-5.18-3.17-6.83z" fill="#E89E6B"/>
                </svg>
              </div>
              <CardTitle>Autostart</CardTitle>
            </div>
            <Toggle
              checked={appAutostart}
              disabled={autostartLoading}
              onChange={handleAutostartToggle}
            />
          </div>
          <CardDescription>
            Start Relay automatically when you log in.
          </CardDescription>
          <div className="mt-4 pt-4 border-t border-border">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-[0.8125rem] font-medium">
                  Start Hidden
                </div>
                <p className="text-[0.6875rem] text-foreground-muted mt-0.5">
                  Launch minimized to system tray instead of showing the window.
                </p>
              </div>
              <Toggle
                checked={form.start_hidden}
                onChange={() => update({ start_hidden: !form.start_hidden })}
              />
            </div>
          </div>
        </Card>
      </div>

      {}
      <SectionLabel>Profiles</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        {}
        <Card>
          <CardTitle className="mb-3">Save Current as Profile</CardTitle>
          <CardDescription>
            Snapshot the current settings to quickly restore them later.
          </CardDescription>
          <div className="mt-4 space-y-3">
            <input
              type="text"
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              placeholder="Profile name"
              className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors"
            />
            <input
              type="text"
              value={profileDesc}
              onChange={(e) => setProfileDesc(e.target.value)}
              placeholder="Description (optional)"
              className="w-full px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors"
            />
            <button
              disabled={!profileName.trim() || profileSaving}
              onClick={async () => {
                if (!form || !profileName.trim()) return;
                setProfileSaving(true);
                try {
                  const req: SaveProfileRequest = {
                    id: null,
                    name: profileName.trim(),
                    description: profileDesc.trim() || null,
                    settings: form,
                    instances: [],
                  };
                  const created = await invoke<Profile>("save_profile", { req });
                  setProfiles((prev) => [...prev, created]);
                  setProfileName("");
                  setProfileDesc("");
                } catch (err) {
                  console.error("Failed to save profile:", err);
                } finally {
                  setProfileSaving(false);
                }
              }}
              className="h-9 px-5 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 transition-all disabled:opacity-50"
            >
              {profileSaving ? "Saving…" : "Save Profile"}
            </button>
          </div>
        </Card>

        {}
        <Card>
          <CardTitle className="mb-3">Saved Profiles</CardTitle>
          {profiles.length === 0 ? (
            <p className="text-[0.8125rem] text-foreground-muted">No profiles saved yet.</p>
          ) : (
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {profiles.map((p) => (
                <div
                  key={p.id}
                  className="flex items-center justify-between gap-2 px-3 py-2 rounded-button border border-border bg-surface"
                >
                  <div className="min-w-0">
                    <div className="text-[0.8125rem] font-medium truncate">{p.name}</div>
                    {p.description && (
                      <div className="text-[0.6875rem] text-foreground-muted truncate">
                        {p.description}
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0">
                    <button
                      disabled={loadingProfileId === p.id}
                      onClick={async () => {
                        setLoadingProfileId(p.id);
                        try {
                          const loaded = await invoke<AppSettings>("load_profile", { id: p.id });
                          setForm(loaded);
                        } catch (err) {
                          console.error("Failed to load profile:", err);
                        } finally {
                          setLoadingProfileId(null);
                        }
                      }}
                      className="h-7 px-2.5 rounded-button text-[0.6875rem] font-medium border border-border hover:bg-surface-hover transition-colors disabled:opacity-50"
                    >
                      {loadingProfileId === p.id ? "Loading…" : "Load"}
                    </button>
                    <button
                      onClick={async () => {
                        try {
                          await invoke("delete_profile", { id: p.id });
                          setProfiles((prev) => prev.filter((x) => x.id !== p.id));
                        } catch (err) {
                          console.error("Failed to delete profile:", err);
                        }
                      }}
                      className="h-7 px-2.5 rounded-button text-[0.6875rem] bg-[rgba(255,59,48,0.1)] text-[#FF3B30] transition-colors"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      {}
      <SectionLabel>Notifications</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        <Card>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-[0.625rem] bg-[#E89E6B]/15 flex items-center justify-center flex-shrink-0">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path d="M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.63-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.64 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z" fill="#E89E6B"/>
                </svg>
              </div>
              <CardTitle>Desktop Notifications</CardTitle>
            </div>
            <Toggle
              checked={form.notifications?.enabled ?? false}
              onChange={() =>
                update({
                  notifications: {
                    ...(form.notifications ?? {
                      enabled: false,
                      proxy_start: true,
                      proxy_stop: false,
                      proxy_error: true,
                      ip_changed: false,
                      kill_switch: true,
                      leak: true,
                      tor: false,
                    }),
                    enabled: !(form.notifications?.enabled ?? false),
                  },
                })
              }
            />
          </div>
          <CardDescription>
            Show OS notifications for important events.
          </CardDescription>
          {form.notifications?.enabled && (
            <div className="mt-4 pt-4 border-t border-border space-y-3">
              {(
                [
                  { key: "proxy_start", label: "Proxy started" },
                  { key: "proxy_stop", label: "Proxy stopped" },
                  { key: "proxy_error", label: "Proxy error" },
                  { key: "ip_changed", label: "IP changed" },
                  { key: "kill_switch", label: "Kill-switch activated" },
                  { key: "leak", label: "Leak detected" },
                  { key: "tor", label: "Tor events" },
                ] as Array<{ key: keyof NotificationSettings; label: string }>
              ).map(({ key, label }) => (
                <div key={key} className="flex items-center justify-between">
                  <span className="text-[0.8125rem]">{label}</span>
                  <Toggle
                    checked={(form.notifications?.[key] ?? false) as boolean}
                    onChange={() =>
                      update({
                        notifications: {
                          ...(form.notifications as NotificationSettings),
                          [key]: !(form.notifications?.[key] ?? false),
                        },
                      })
                    }
                  />
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      {}
      <SectionLabel>Import & Export</SectionLabel>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-8">
        <Card>
          <CardTitle className="mb-2">Export Configuration</CardTitle>
          <CardDescription>
            Save all settings, profiles, split-tunnel rules and schedules to a
            JSON file in your Downloads folder.
          </CardDescription>
          <div className="mt-4">
            <button
              disabled={exporting}
              onClick={async () => {
                setExporting(true);
                setExportedPath(null);
                try {
                  const result = await invoke<{ path: string }>("export_config");
                  setExportedPath(result.path);
                } catch (err) {
                  console.error("Export failed:", err);
                } finally {
                  setExporting(false);
                }
              }}
              className="h-9 px-5 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 disabled:opacity-50 transition-all"
            >
              {exporting ? "Exporting…" : "Export"}
            </button>
            {exportedPath && (
              <p className="text-[0.6875rem] text-foreground-muted mt-2 font-mono break-all">
                Saved to: {exportedPath}
              </p>
            )}
          </div>
        </Card>

        <Card>
          <CardTitle className="mb-2">Import Configuration</CardTitle>
          <CardDescription>
            Restore settings, profiles, rules and schedules from a previously
            exported JSON file. Current settings will be replaced.
          </CardDescription>
          <div className="mt-4 space-y-2">
            <input
              ref={importFileRef}
              type="file"
              accept=".json"
              className="hidden"
              onChange={async (e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                setImporting(true);
                setImportError(null);
                setImportSuccess(false);
                try {
                  const json = await file.text();
                  await invoke("import_config", { json });
                  setImportSuccess(true);
                  setTimeout(() => setImportSuccess(false), 3000);
                  
                  const updated = await invoke<AppSettings>("get_settings");
                  setForm(updated);
                } catch (err) {
                  setImportError(String(err));
                } finally {
                  setImporting(false);
                  if (importFileRef.current) importFileRef.current.value = "";
                }
              }}
            />
            <button
              disabled={importing}
              onClick={() => importFileRef.current?.click()}
              className="h-9 px-5 rounded-button text-[0.8125rem] font-medium border border-border hover:bg-surface-hover disabled:opacity-50 transition-all"
            >
              {importing ? "Importing…" : "Choose file…"}
            </button>
            {importSuccess && (
              <p className="text-[0.6875rem] text-[#30D158]">
                Import successful. Settings have been applied.
              </p>
            )}
            {importError && (
              <p className="text-[0.6875rem] text-[#FF3B30] break-all">
                Error: {importError}
              </p>
            )}
          </div>
        </Card>
      </div>

      {}
      <div className="flex justify-end">
        <button
          onClick={handleSave}
          disabled={saving}
          className="h-10 px-8 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)] transition-all duration-200 disabled:opacity-50"
        >
          {saving ? "Saving..." : saved ? "Saved!" : "Save Settings"}
        </button>
      </div>
    </div>
  );
}
