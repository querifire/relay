import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  useProxyLists,
  useProxyCacheStats,
} from "../hooks/useProxyLists";
import type { ProxyListConfig } from "../types";

/* ── Icon components ─────────────────────────────────────────── */

function RefreshIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
    </svg>
  );
}

function EditIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <path d="M18 6 6 18M6 6l12 12" />
    </svg>
  );
}

function DeleteIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

/* ── Mixed-input parser ──────────────────────────────────────── */

function parseInputLines(raw: string): { urls: string[]; proxies: string[] } {
  const lines = raw
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
  const urls: string[] = [];
  const proxies: string[] = [];
  for (const line of lines) {
    if (/^https?:\/\//i.test(line)) {
      urls.push(line);
    } else {
      proxies.push(line);
    }
  }
  return { urls, proxies };
}

/* ── Format Guide Modal ──────────────────────────────────────── */

function FormatGuideModal({ onClose }: { onClose: () => void }) {
  const formats = [
    { format: "IP:PORT", example: "192.168.1.1:8080" },
    { format: "IP:PORT:USER:PASS", example: "192.168.1.1:8080:admin:secret" },
    { format: "USER:PASS@IP:PORT", example: "admin:secret@192.168.1.1:8080" },
    { format: "PROTOCOL://IP:PORT", example: "socks5://192.168.1.1:1080" },
    {
      format: "PROTOCOL://USER:PASS@IP:PORT",
      example: "http://admin:secret@192.168.1.1:8080",
    },
    { format: "IP:PORT@USER:PASS", example: "192.168.1.1:8080@admin:secret" },
    {
      format: "URL link to proxy list",
      example: "https://api.proxysource.io/v1/raw?key=abc",
    },
  ];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-surface-card border border-border rounded-card w-full max-w-lg shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <h3 className="text-[1rem] font-semibold">Supported Proxy Formats</h3>
          <button
            onClick={onClose}
            className="w-8 h-8 rounded-[0.5rem] flex items-center justify-center text-foreground-muted hover:text-foreground hover:bg-surface-hover transition-all cursor-pointer border-none bg-transparent"
          >
            <CloseIcon />
          </button>
        </div>
        <div className="px-6 py-5 space-y-4">
          {formats.map((f) => (
            <div key={f.format}>
              <div className="text-[0.8125rem] font-semibold text-foreground mb-1">
                {f.format}
              </div>
              <code className="text-[0.75rem] text-foreground-muted font-mono bg-surface-hover px-2 py-1 rounded-[0.375rem]">
                {f.example}
              </code>
            </div>
          ))}
          <div className="mt-2 pt-4 border-t border-border">
            <div className="text-[0.8125rem] font-semibold text-foreground mb-1">
              Mixed Format
            </div>
            <p className="text-[0.75rem] text-foreground-muted leading-relaxed">
              You can mix proxy addresses and URL links in the same input — each
              line is parsed independently. Lines starting with{" "}
              <code className="bg-surface-hover px-1 py-0.5 rounded-[0.25rem]">http://</code>{" "}
              or{" "}
              <code className="bg-surface-hover px-1 py-0.5 rounded-[0.25rem]">https://</code>{" "}
              are treated as remote list URLs; everything else is parsed as a
              proxy address.
            </p>
          </div>
        </div>
        <div className="px-6 py-4 border-t border-border flex justify-end">
          <button
            onClick={onClose}
            className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface cursor-pointer transition-all duration-200 border-none hover:opacity-80"
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}

/* ── Edit Source Modal ────────────────────────────────────────── */

function EditSourceModal({
  config,
  onSave,
  onDelete,
  onClose,
}: {
  config: ProxyListConfig;
  onSave: (updated: ProxyListConfig) => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  const [editName, setEditName] = useState(config.name);
  const [editData, setEditData] = useState(
    [...config.urls, ...config.inline_proxies].join("\n"),
  );
  const [confirmDelete, setConfirmDelete] = useState(false);

  function handleSave() {
    const { urls, proxies } = parseInputLines(editData);
    onSave({
      ...config,
      name: editName.trim() || config.name,
      urls,
      inline_proxies: proxies,
    });
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-surface-card border border-border rounded-card w-full max-w-lg shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <h3 className="text-[1rem] font-semibold">Edit Source</h3>
          <button
            onClick={onClose}
            className="w-8 h-8 rounded-[0.5rem] flex items-center justify-center text-foreground-muted hover:text-foreground hover:bg-surface-hover transition-all cursor-pointer border-none bg-transparent"
          >
            <CloseIcon />
          </button>
        </div>

        <div className="px-6 py-5 space-y-5">
          <div>
            <label className="block text-[0.8125rem] font-semibold text-foreground mb-2">
              Source Name
            </label>
            <input
              type="text"
              className="w-full py-3 px-4 border border-border rounded-button font-sans text-[0.875rem] transition-all duration-200 bg-surface-hover focus:outline-none focus:border-foreground focus:bg-surface"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
            />
          </div>

          <div>
            <label className="block text-[0.8125rem] font-semibold text-foreground mb-2">
              Raw List URL or Data
            </label>
            <textarea
              className="w-full py-3 px-4 border border-border rounded-button font-mono text-[0.875rem] transition-all duration-200 bg-surface-hover focus:outline-none focus:border-foreground focus:bg-surface min-h-[6.25rem] resize-y"
              value={editData}
              onChange={(e) => setEditData(e.target.value)}
            />
          </div>

          <div className="flex gap-6 text-[0.75rem] text-foreground-muted pt-2 border-t border-border">
            <span>URLs: {config.urls.length}</span>
            <span>Inline proxies: {config.inline_proxies.length}</span>
          </div>
        </div>

        <div className="px-6 py-4 border-t border-border flex justify-between">
          <div>
            {confirmDelete ? (
              <div className="flex items-center gap-2">
                <span className="text-[0.75rem] text-foreground-muted">Are you sure?</span>
                <button
                  onClick={onDelete}
                  className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-[#FF3B30] text-white cursor-pointer transition-all border-none hover:opacity-80"
                >
                  Delete
                </button>
                <button
                  onClick={() => setConfirmDelete(false)}
                  className="h-8 px-3 rounded-button text-[0.75rem] font-medium border border-border bg-transparent cursor-pointer hover:bg-surface-hover transition-all"
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                onClick={() => setConfirmDelete(true)}
                className="h-9 px-3 rounded-button text-[0.8125rem] font-medium text-[#FF3B30] bg-transparent cursor-pointer transition-all border-none hover:bg-[rgba(255,59,48,0.1)] flex items-center gap-1.5"
              >
                <DeleteIcon />
                Delete
              </button>
            )}
          </div>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="h-9 px-4 rounded-button text-[0.8125rem] font-medium border border-border bg-transparent cursor-pointer hover:bg-surface-hover transition-all"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface cursor-pointer transition-all duration-200 border-none hover:opacity-80"
            >
              Save Changes
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ── Page ─────────────────────────────────────────────────────── */

export default function ProxyListsPage() {
  const { lists, saveList, deleteList } = useProxyLists();
  const { stats, refresh: refreshStats } = useProxyCacheStats();

  // ── Form state ───────────────────────────────────────────────
  const [sourceName, setSourceName] = useState("");
  const [sourceData, setSourceData] = useState("");

  // ── Refresh state per custom list ────────────────────────────
  const [refreshingIds, setRefreshingIds] = useState<Set<string>>(new Set());

  // ── Modals ───────────────────────────────────────────────────
  const [editingConfig, setEditingConfig] = useState<ProxyListConfig | null>(null);
  const [showFormatGuide, setShowFormatGuide] = useState(false);

  // ── Computed ─────────────────────────────────────────────────
  const cachedTotal = stats?.total ?? 0;
  const totalProxies =
    cachedTotal +
    lists.reduce(
      (sum, l) => sum + l.inline_proxies.length + l.urls.length,
      0,
    );

  const lastSyncTs = stats?.last_updated ?? 0;
  const lastSyncDate = lastSyncTs > 0 ? new Date(lastSyncTs * 1000) : null;

  // ── Handlers ─────────────────────────────────────────────────

  async function handleAddSource() {
    if (!sourceName.trim() && !sourceData.trim()) return;

    const { urls, proxies } = parseInputLines(sourceData);

    const config: ProxyListConfig = {
      id: Date.now().toString(),
      name: sourceName.trim() || `Source ${lists.length + 1}`,
      urls,
      inline_proxies: proxies,
    };

    await saveList(config);
    setSourceName("");
    setSourceData("");
  }

  async function handleSaveEdit(updated: ProxyListConfig) {
    await saveList(updated);
    setEditingConfig(null);
  }

  async function handleDeleteSource() {
    if (!editingConfig) return;
    await deleteList(editingConfig.id);
    setEditingConfig(null);
  }

  async function handleRefreshCustomList(id: string) {
    if (refreshingIds.has(id)) return;
    setRefreshingIds((prev) => new Set(prev).add(id));
    try {
      await invoke("refresh_proxy_list", { id });
      await refreshStats();
    } catch (err) {
      console.error("Failed to refresh proxy list:", err);
    } finally {
      setRefreshingIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  }

  // ── Helper: display string for a config ──────────────────────
  function configDisplayUrl(config: ProxyListConfig): string {
    if (config.urls.length > 0) return config.urls[0];
    if (config.inline_proxies.length > 0)
      return `Static Text (${config.inline_proxies.length} items)`;
    return "—";
  }

  function configProxyCount(config: ProxyListConfig): string {
    const total = config.inline_proxies.length;
    const urlCount = config.urls.length;
    if (total > 0 && urlCount > 0)
      return `${total} inline + ${urlCount} URL${urlCount > 1 ? "s" : ""}`;
    if (total > 0) return `${total} Proxies`;
    if (urlCount > 0) return `${urlCount} URL${urlCount > 1 ? "s" : ""}`;
    return "Empty";
  }

  return (
    <div>
      {/* ── Modals ──────────────────────────────────────────────── */}
      {showFormatGuide && (
        <FormatGuideModal onClose={() => setShowFormatGuide(false)} />
      )}
      {editingConfig && (
        <EditSourceModal
          config={editingConfig}
          onSave={handleSaveEdit}
          onDelete={handleDeleteSource}
          onClose={() => setEditingConfig(null)}
        />
      )}

      {/* ── Header ──────────────────────────────────────────────── */}
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Proxies</span>
          <span>/</span>
          <span className="text-foreground">List Management</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Proxy Lists
        </h1>
      </header>

      {/* ── Grid layout ─────────────────────────────────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_20rem] gap-6 md:gap-8 max-w-[75rem]">
        {/* ── Left content ──────────────────────────────────────── */}
        <div>
          {/* ── Import Proxy Source card ─────────────────────────── */}
          <div className="bg-surface-card border border-border rounded-card p-6 mb-8">
            <div className="flex justify-between items-start mb-6">
              <div>
                <h2 className="text-[1.125rem] font-semibold">Import Proxy Source</h2>
                <p className="text-foreground-muted text-[0.8125rem]">
                  Add a raw text URL or paste proxy addresses directly.
                </p>
              </div>
              <button
                onClick={() => setShowFormatGuide(true)}
                className="h-7 px-[0.625rem] rounded-button text-[0.75rem] font-medium bg-surface-hover text-foreground flex items-center gap-2 cursor-pointer transition-all duration-200 border-none hover:opacity-80"
              >
                Format Guide
              </button>
            </div>

            <div className="mb-5">
              <label className="block text-[0.8125rem] font-semibold text-foreground mb-2">
                Source Name
              </label>
              <input
                type="text"
                className="w-full py-3 px-4 border border-border rounded-button font-sans text-[0.875rem] transition-all duration-200 bg-surface-hover focus:outline-none focus:border-foreground focus:bg-surface"
                placeholder="e.g. My Premium Residential List"
                value={sourceName}
                onChange={(e) => setSourceName(e.target.value)}
              />
            </div>

            <div className="mb-5">
              <label className="block text-[0.8125rem] font-semibold text-foreground mb-2">
                Raw List URL or Data
              </label>
              <textarea
                className="w-full py-3 px-4 border border-border rounded-button font-mono text-[0.875rem] transition-all duration-200 bg-surface-hover focus:outline-none focus:border-foreground focus:bg-surface min-h-[6.25rem] resize-y"
                placeholder={
                  "https://api.proxysource.io/v1/raw?key=...\n192.168.1.1:8080\nsocks5://user:pass@10.0.0.1:1080\n\nMix URLs and proxy addresses — each line is parsed independently."
                }
                value={sourceData}
                onChange={(e) => setSourceData(e.target.value)}
              />
              {sourceData.trim() &&
                (() => {
                  const { urls, proxies } = parseInputLines(sourceData);
                  return (
                    <div className="mt-2 text-[0.6875rem] text-foreground-muted flex gap-4">
                      {urls.length > 0 && (
                        <span>{urls.length} URL{urls.length !== 1 ? "s" : ""} detected</span>
                      )}
                      {proxies.length > 0 && (
                        <span>
                          {proxies.length} prox{proxies.length !== 1 ? "ies" : "y"} detected
                        </span>
                      )}
                    </div>
                  );
                })()}
            </div>

            <div className="flex items-center justify-end mt-2">
              <button
                onClick={handleAddSource}
                className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface flex items-center gap-2 cursor-pointer transition-all duration-200 border-none hover:opacity-80 hover:-translate-y-px hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]"
              >
                <PlusIcon />
                Add Source
              </button>
            </div>
          </div>

          {/* ── Managed Sources table ───────────────────────────── */}
          <div className="bg-surface-card border border-border rounded-card overflow-hidden">
            <div className="py-4 px-6 border-b border-border flex justify-between items-center bg-[#FAFAFA] dark:bg-surface-hover">
              <span className="text-[0.875rem] font-semibold">Managed Sources</span>
              <div className="text-[0.75rem] text-foreground-muted">
                {1 + lists.length} source{lists.length > 0 ? "s" : ""}
              </div>
            </div>

            {/* Built-in Default list */}
            <div className="py-4 px-6 border-b border-border grid grid-cols-[2fr_1fr_1fr_6.25rem] items-center transition-colors duration-200 hover:bg-surface-hover">
              <div className="flex flex-col">
                <div className="font-semibold text-foreground flex items-center gap-2">
                  Default
                  <span className="bg-[rgba(52,199,89,0.1)] text-[#34C759] py-[0.125rem] px-2 rounded-[0.375rem] text-[0.625rem] font-bold uppercase">
                    Built-in
                  </span>
                </div>
                <div className="text-[0.75rem] text-foreground-muted mt-[0.125rem]">
                  Auto-discovery from 35+ public sources
                </div>
              </div>
              <div className="text-[0.8125rem] text-foreground-muted">
                {cachedTotal > 0 ? `${cachedTotal.toLocaleString()} Cached` : "No cache yet"}
              </div>
              <div
                className={`text-[0.8125rem] ${cachedTotal > 0 ? "text-[#34C759]" : "text-foreground-muted"}`}
              >
                {cachedTotal > 0 ? "Active" : "Idle"}
              </div>
              <div className="flex justify-end gap-2">
                <button
                  onClick={() => refreshStats()}
                  className="w-8 h-8 rounded-[0.5rem] flex items-center justify-center text-foreground-muted border border-transparent bg-transparent cursor-pointer transition-all duration-200 hover:bg-surface hover:border-border hover:text-foreground hover:shadow-[0_2px_4px_rgba(0,0,0,0.05)]"
                  title="Refresh"
                >
                  <RefreshIcon />
                </button>
              </div>
            </div>

            {/* Custom sources */}
            {lists.map((config) => (
              <div
                key={config.id}
                className="py-4 px-6 border-b border-border last:border-b-0 grid grid-cols-[2fr_1fr_1fr_6.25rem] items-center transition-colors duration-200 hover:bg-surface-hover"
              >
                <div className="flex flex-col">
                  <div className="font-semibold text-foreground">{config.name}</div>
                  <div className="text-[0.75rem] text-foreground-muted font-mono mt-[0.125rem] whitespace-nowrap overflow-hidden text-ellipsis max-w-[17.5rem]">
                    {configDisplayUrl(config)}
                  </div>
                </div>
                <div className="text-[0.8125rem] text-foreground-muted">
                  {configProxyCount(config)}
                </div>
                <div className="text-[0.8125rem] text-foreground-muted">Custom</div>
                <div className="flex justify-end gap-2">
                  <button
                    onClick={() => handleRefreshCustomList(config.id)}
                    disabled={refreshingIds.has(config.id)}
                    className={`w-8 h-8 rounded-[0.5rem] flex items-center justify-center border border-transparent bg-transparent cursor-pointer transition-all duration-200 hover:bg-surface hover:border-border hover:text-foreground hover:shadow-[0_2px_4px_rgba(0,0,0,0.05)] ${refreshingIds.has(config.id) ? "text-foreground-tertiary" : "text-foreground-muted"}`}
                    title="Refresh proxies from this source"
                  >
                    <span className={refreshingIds.has(config.id) ? "animate-spin inline-flex" : "inline-flex"}>
                      <RefreshIcon />
                    </span>
                  </button>
                  <button
                    onClick={() => setEditingConfig(config)}
                    className="w-8 h-8 rounded-[0.5rem] flex items-center justify-center text-foreground-muted border border-transparent bg-transparent cursor-pointer transition-all duration-200 hover:bg-surface hover:border-border hover:text-foreground hover:shadow-[0_2px_4px_rgba(0,0,0,0.05)]"
                    title="Edit"
                  >
                    <EditIcon />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* ── Right stats panel ─────────────────────────────────── */}
        <div className="flex flex-col gap-6">
          {/* Total Active Proxies */}
          <div className="bg-surface-card border border-border rounded-card p-5">
            <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.05em] mb-3">
              Total Active Proxies
            </div>
            <div className="text-[1.5rem] font-semibold tracking-[-0.02em]">
              {totalProxies.toLocaleString()}
            </div>
            <div className="mt-3 h-1 bg-border rounded-[0.125rem]">
              {totalProxies > 0 && (
                <div
                  className="h-full rounded-[0.125rem]"
                  style={{
                    width: "100%",
                    background: "linear-gradient(90deg, var(--accent-mid), var(--accent-end))",
                  }}
                />
              )}
            </div>
          </div>

          {/* Sync Status */}
          <div className="bg-surface-card border border-border rounded-card p-5">
            <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.05em] mb-3">
              Sync Status
            </div>
            <div className="flex flex-col gap-3">
              <div className="flex justify-between text-[0.8125rem]">
                <span className="text-foreground-muted">Sources</span>
                <span className="font-semibold">{1 + lists.length} total</span>
              </div>
              <div className="flex justify-between text-[0.8125rem]">
                <span className="text-foreground-muted">Custom Lists</span>
                <span className="font-semibold">{lists.length}</span>
              </div>
              <div className="flex justify-between text-[0.8125rem]">
                <span className="text-foreground-muted">Last Sync</span>
                <span className="font-semibold">
                  {lastSyncDate
                    ? lastSyncDate.toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })
                    : "—"}
                </span>
              </div>
              {stats && stats.total > 0 && (
                <>
                  <div className="pt-2 border-t border-border" />
                  {stats.socks5 > 0 && (
                    <div className="flex justify-between text-[0.8125rem]">
                      <span className="text-foreground-muted">SOCKS5</span>
                      <span className="font-semibold">{stats.socks5.toLocaleString()}</span>
                    </div>
                  )}
                  {stats.socks4 > 0 && (
                    <div className="flex justify-between text-[0.8125rem]">
                      <span className="text-foreground-muted">SOCKS4</span>
                      <span className="font-semibold">{stats.socks4.toLocaleString()}</span>
                    </div>
                  )}
                  {stats.http > 0 && (
                    <div className="flex justify-between text-[0.8125rem]">
                      <span className="text-foreground-muted">HTTP</span>
                      <span className="font-semibold">{stats.http.toLocaleString()}</span>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>

          {/* Integration Tip */}
          <div className="border border-dashed border-border rounded-card p-5 bg-transparent opacity-80">
            <div className="text-[0.75rem] font-semibold text-foreground-muted uppercase tracking-[0.05em] mb-3">
              Integration Tip
            </div>
            <p className="text-[0.75rem] text-foreground-muted leading-relaxed">
              You can connect your GitHub Gists or private S3 buckets to auto-sync proxy lists in real-time.
            </p>
            <a
              href="#"
              className="block mt-3 text-[0.75rem] font-semibold text-foreground no-underline hover:opacity-70 transition-opacity"
            >
              View documentation →
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
