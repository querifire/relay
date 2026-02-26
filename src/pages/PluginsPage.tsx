import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "react-router-dom";
import type { PluginInfo, PluginStatus, PluginUiInfo } from "../types";

type PluginSchema = Record<string, unknown> | null;

interface BusyState {
  action: "install" | "uninstall" | "enable" | "disable";
  progress: number;
}

function deriveStatus(plugin: PluginInfo, busy?: BusyState): PluginStatus {
  if (busy) return "installing";
  if (plugin.last_error) return "error";
  if (plugin.enabled) return "enabled";
  if (plugin.installed) return "installed";
  return "not_installed";
}

function statusTone(status: PluginStatus): string {
  switch (status) {
    case "enabled":
      return "text-[#34C759]";
    case "installed":
      return "text-[#0A84FF]";
    case "installing":
      return "text-[#FF9F0A]";
    case "error":
      return "text-[#FF453A]";
    default:
      return "text-foreground-muted";
  }
}

function statusLabel(status: PluginStatus): string {
  switch (status) {
    case "enabled":
      return "Enabled";
    case "installed":
      return "Installed";
    case "installing":
      return "Working...";
    case "error":
      return "Error";
    default:
      return "Not installed";
  }
}

function pluginWithStatus(plugin: PluginInfo, busy?: BusyState): PluginUiInfo {
  const status = deriveStatus(plugin, busy);
  return { ...plugin, status };
}

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<Record<string, BusyState>>({});
  const [schemas, setSchemas] = useState<Record<string, PluginSchema>>({});
  const [schemaLoading, setSchemaLoading] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);

  const loadPlugins = useCallback(async () => {
    try {
      const data = await invoke<PluginInfo[]>("get_plugins");
      setPlugins(data);
      setError(null);
    } catch (err) {
      console.error("Failed to load plugins:", err);
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    loadPlugins().finally(() => setLoading(false));
  }, [loadPlugins]);

  const runAction = useCallback(
    async (pluginId: string, action: BusyState["action"]) => {
      setBusy((prev) => ({ ...prev, [pluginId]: { action, progress: 5 } }));
      const timer = window.setInterval(() => {
        setBusy((prev) => {
          const current = prev[pluginId];
          if (!current) return prev;
          const next = Math.min(current.progress + 10, 90);
          return { ...prev, [pluginId]: { ...current, progress: next } };
        });
      }, 250);

      try {
        if (action === "install") {
          await invoke("install_plugin", { id: pluginId });
        } else if (action === "uninstall") {
          await invoke("uninstall_plugin", { id: pluginId });
        } else if (action === "enable") {
          await invoke("enable_plugin", { id: pluginId });
        } else {
          await invoke("disable_plugin", { id: pluginId });
        }
        setBusy((prev) => ({ ...prev, [pluginId]: { action, progress: 100 } }));
      } catch (err) {
        console.error(`Failed to ${action} plugin ${pluginId}:`, err);
      } finally {
        window.clearInterval(timer);
        await loadPlugins();
        window.setTimeout(() => {
          setBusy((prev) => {
            const next = { ...prev };
            delete next[pluginId];
            return next;
          });
        }, 220);
      }
    },
    [loadPlugins]
  );

  const openPluginsFolder = async () => {
    try {
      await invoke("open_plugins_folder");
    } catch (err) {
      console.error("Failed to open plugins folder:", err);
      setError(String(err));
    }
  };

  const loadSchema = async (pluginId: string) => {
    if (schemas[pluginId] || schemaLoading[pluginId]) return;
    setSchemaLoading((prev) => ({ ...prev, [pluginId]: true }));
    try {
      const schema = await invoke<PluginSchema>("get_plugin_settings_schema", { id: pluginId });
      setSchemas((prev) => ({ ...prev, [pluginId]: schema }));
    } catch (err) {
      console.error("Failed to get plugin schema:", err);
    } finally {
      setSchemaLoading((prev) => ({ ...prev, [pluginId]: false }));
    }
  };

  const builtins = useMemo(
    () => plugins.filter((p) => p.plugin_type === "builtin"),
    [plugins]
  );
  const installedExternal = useMemo(
    () => plugins.filter((p) => p.plugin_type === "external"),
    [plugins]
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-[0.875rem] text-foreground-muted">
        Loading plugins...
      </div>
    );
  }

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <Link to="/" className="hover:text-foreground transition-colors">Home</Link>
          <span>/</span>
          <span className="text-foreground">Plugins</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">Plugins</h1>
      </header>

      {error && (
        <div className="mb-6 px-4 py-3 rounded-card border border-[#FF453A]/50 bg-[#FF453A]/10 text-[0.8125rem] text-[#FF453A]">
          {error}
        </div>
      )}

      <section className="mb-8">
        <div className="text-[0.6875rem] font-semibold text-foreground-tertiary uppercase tracking-[0.05em] mb-3">
          Built-in
        </div>
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
          {builtins.map((plugin) => {
            const pluginBusy = busy[plugin.id];
            const ui = pluginWithStatus(plugin, pluginBusy);
            return (
              <article
                key={plugin.id}
                className="bg-surface-card border border-border rounded-card shadow-card p-5"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <h3 className="text-[0.95rem] font-semibold">{plugin.name}</h3>
                    <p className="text-[0.75rem] text-foreground-muted mt-1">{plugin.version}</p>
                  </div>
                  <span className={`text-[0.6875rem] font-semibold uppercase tracking-[0.04em] ${statusTone(ui.status)}`}>
                    {statusLabel(ui.status)}
                  </span>
                </div>

                <p className="text-[0.8125rem] text-foreground-muted mt-3 min-h-[2.5rem]">
                  {plugin.description}
                </p>

                {pluginBusy && (
                  <div className="mt-3">
                    <div className="h-1.5 rounded-full bg-surface-hover overflow-hidden">
                      <div
                        className="h-full bg-foreground transition-all duration-200"
                        style={{ width: `${pluginBusy.progress}%` }}
                      />
                    </div>
                    <p className="text-[0.6875rem] text-foreground-tertiary mt-1">
                      {pluginBusy.action}... {pluginBusy.progress}%
                    </p>
                  </div>
                )}

                {plugin.last_error && (
                  <p className="mt-3 text-[0.75rem] text-[#FF453A]">{plugin.last_error}</p>
                )}

                <div className="flex flex-wrap gap-2 mt-4">
                  {!plugin.installed ? (
                    <button
                      type="button"
                      onClick={() => runAction(plugin.id, "install")}
                      disabled={Boolean(pluginBusy)}
                      className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-foreground text-white dark:bg-white dark:text-[#1C1C1E] disabled:opacity-50"
                    >
                      Install
                    </button>
                  ) : (
                    <button
                      type="button"
                      onClick={() => runAction(plugin.id, "uninstall")}
                      disabled={Boolean(pluginBusy)}
                      className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-surface-hover border border-border disabled:opacity-50"
                    >
                      Uninstall
                    </button>
                  )}

                  {plugin.installed && !plugin.enabled && (
                    <button
                      type="button"
                      onClick={() => runAction(plugin.id, "enable")}
                      disabled={Boolean(pluginBusy)}
                      className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-[#34C759]/20 text-[#34C759] border border-[#34C759]/50 disabled:opacity-50"
                    >
                      Enable
                    </button>
                  )}

                  {plugin.enabled && (
                    <button
                      type="button"
                      onClick={() => runAction(plugin.id, "disable")}
                      disabled={Boolean(pluginBusy)}
                      className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-[#FF9F0A]/20 text-[#FF9F0A] border border-[#FF9F0A]/50 disabled:opacity-50"
                    >
                      Disable
                    </button>
                  )}

                  <button
                    type="button"
                    onClick={() => loadSchema(plugin.id)}
                    disabled={schemaLoading[plugin.id]}
                    className="h-8 px-3 rounded-button text-[0.75rem] font-medium bg-surface-hover border border-border disabled:opacity-50"
                  >
                    {schemaLoading[plugin.id] ? "Loading..." : "Settings"}
                  </button>
                </div>

                {schemas[plugin.id] && (
                  <pre className="mt-3 p-3 rounded-button bg-surface-hover border border-border text-[0.6875rem] text-foreground-muted overflow-auto">
                    {JSON.stringify(schemas[plugin.id], null, 2)}
                  </pre>
                )}
              </article>
            );
          })}
        </div>
      </section>

      <section className="mb-8">
        <div className="text-[0.6875rem] font-semibold text-foreground-tertiary uppercase tracking-[0.05em] mb-3">
          Installed
        </div>
        {installedExternal.length === 0 ? (
          <div className="bg-surface-card border border-border rounded-card p-5 text-[0.8125rem] text-foreground-muted">
            No user plugins found. Drop plugin folders with a <code>plugin.toml</code> into the plugins directory.
          </div>
        ) : (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
            {installedExternal.map((plugin) => {
              const pluginBusy = busy[plugin.id];
              const ui = pluginWithStatus(plugin, pluginBusy);
              return (
                <article key={plugin.id} className="bg-surface-card border border-border rounded-card shadow-card p-5">
                  <div className="flex items-center justify-between gap-3">
                    <h3 className="text-[0.95rem] font-semibold">{plugin.name}</h3>
                    <span className={`text-[0.6875rem] font-semibold uppercase tracking-[0.04em] ${statusTone(ui.status)}`}>
                      {statusLabel(ui.status)}
                    </span>
                  </div>
                  <p className="text-[0.75rem] text-foreground-muted mt-1">{plugin.version}</p>
                  <p className="text-[0.8125rem] text-foreground-muted mt-3">{plugin.description}</p>
                </article>
              );
            })}
          </div>
        )}
      </section>

      <div className="flex justify-end">
        <button
          type="button"
          onClick={openPluginsFolder}
          className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-surface-hover border border-border"
        >
          Open Plugins Folder
        </button>
      </div>
    </div>
  );
}
