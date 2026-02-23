import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
  useRef,
} from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProxyInstanceInfo, ProxyMode, ProxyProtocol, ProxyChainConfig } from "../types";

interface ProxyContextValue {
  /** All proxy instances (polled from backend). */
  instances: ProxyInstanceInfo[];
  /** True only until the first successful fetch completes. */
  loading: boolean;
  /** Set of instance IDs currently performing a short action (stop/delete). */
  busyIds: Set<string>;
  refresh: () => Promise<void>;
  createInstance: (
    name: string,
    bindAddr: string,
    port: number,
    mode: ProxyMode,
    localProtocol?: ProxyProtocol,
    authUsername?: string | null,
    authPassword?: string | null,
    autoRotate?: boolean,
    proxyList?: string,
    autoRotateMinutes?: number | null,
    proxyChain?: ProxyChainConfig | null,
  ) => Promise<void>;
  /** Fire-and-forget — backend tracks "Starting" status. */
  startInstance: (
    id: string,
    upstream?: { host: string; port: number; protocol: string },
  ) => void;
  stopInstance: (id: string) => Promise<void>;
  deleteInstance: (id: string) => Promise<void>;
  toggleAutoRotate: (id: string, enabled: boolean) => Promise<void>;
  updateAutoRotateMinutes: (id: string, minutes: number) => Promise<void>;
}

const ProxyContext = createContext<ProxyContextValue | null>(null);

export function ProxyProvider({ children }: { children: ReactNode }) {
  const [instances, setInstances] = useState<ProxyInstanceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const initialDone = useRef(false);

  const addBusy = (id: string) =>
    setBusyIds((prev) => new Set(prev).add(id));
  const removeBusy = (id: string) =>
    setBusyIds((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<ProxyInstanceInfo[]>("get_instances");
      setInstances(result);
    } catch (err) {
      console.error("Failed to fetch instances:", err);
    }
    if (!initialDone.current) {
      initialDone.current = true;
      setLoading(false);
    }
  }, []);

  // Initial fetch
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Adaptive polling: 3s when any instance is Running/Starting, 10s otherwise; pause when tab hidden
  useEffect(() => {
    const hasActive = instances.some(
      (i) => i.status === "Running" || i.status === "Starting"
    );
    const intervalMs = hasActive ? 3000 : 10000;

    const tick = () => {
      if (document.visibilityState === "hidden") return;
      refresh();
    };

    const id = setInterval(tick, intervalMs);
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") refresh();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      clearInterval(id);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [instances, refresh]);

  const createInstance = useCallback(
    async (
      name: string,
      bindAddr: string,
      port: number,
      mode: ProxyMode,
      localProtocol?: ProxyProtocol,
      authUsername?: string | null,
      authPassword?: string | null,
      autoRotate?: boolean,
      proxyList?: string,
      autoRotateMinutes?: number | null,
      proxyChain?: ProxyChainConfig | null,
    ) => {
      await invoke("create_instance", {
        name,
        bindAddr,
        port,
        mode,
        localProtocol: localProtocol ?? null,
        authUsername: authUsername ?? null,
        authPassword: authPassword ?? null,
        autoRotate: autoRotate ?? null,
        proxyList: proxyList ?? null,
        autoRotateMinutes: autoRotateMinutes ?? null,
        proxyChain: proxyChain ?? null,
      });
      await refresh();
    },
    [refresh],
  );

  const startInstance = useCallback(
    (
      id: string,
      upstream?: { host: string; port: number; protocol: string },
    ) => {
      const params: Record<string, unknown> = { id };
      if (upstream) {
        params.upstreamHost = upstream.host;
        params.upstreamPort = upstream.port;
        params.upstreamProtocol = upstream.protocol;
      }
      invoke("start_instance", params)
        .then(() => refresh())
        .catch(() => refresh());
      setTimeout(() => refresh(), 300);
    },
    [refresh],
  );

  const stopInstance = useCallback(
    async (id: string) => {
      addBusy(id);
      try {
        await invoke("stop_instance", { id });
        await refresh();
      } finally {
        removeBusy(id);
      }
    },
    [refresh],
  );

  const deleteInstance = useCallback(
    async (id: string) => {
      addBusy(id);
      try {
        await invoke("delete_instance", { id });
        await refresh();
      } finally {
        removeBusy(id);
      }
    },
    [refresh],
  );

  const toggleAutoRotate = useCallback(
    async (id: string, enabled: boolean) => {
      try {
        await invoke("toggle_auto_rotate", { id, enabled });
        await refresh();
      } catch (err) {
        console.error("Failed to toggle auto-rotate:", err);
      }
    },
    [refresh],
  );

  const updateAutoRotateMinutes = useCallback(
    async (id: string, minutes: number) => {
      try {
        await invoke("update_auto_rotate_minutes", { id, minutes });
        await refresh();
      } catch (err) {
        console.error("Failed to update auto-rotate minutes:", err);
      }
    },
    [refresh],
  );

  const value = useMemo(
    () => ({
      instances,
      loading,
      busyIds,
      refresh,
      createInstance,
      startInstance,
      stopInstance,
      deleteInstance,
      toggleAutoRotate,
      updateAutoRotateMinutes,
    }),
    [
      instances,
      loading,
      busyIds,
      refresh,
      createInstance,
      startInstance,
      stopInstance,
      deleteInstance,
      toggleAutoRotate,
      updateAutoRotateMinutes,
    ]
  );

  return (
    <ProxyContext.Provider value={value}>
      {children}
    </ProxyContext.Provider>
  );
}

export function useProxies() {
  const ctx = useContext(ProxyContext);
  if (!ctx) throw new Error("useProxies must be used within <ProxyProvider>");
  return ctx;
}
