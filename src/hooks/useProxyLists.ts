import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProxyCacheStats, ProxyListConfig } from "../types";

export type { ProxyListConfig };

export interface ProxySource {
  id: string;
  name: string;
  url: string; // first URL or inline summary
  proxyCount: number;
  status: "Syncing" | "Idle" | "Offline";
  isDefault: boolean;
}

export function useProxyLists() {
  const [lists, setLists] = useState<ProxyListConfig[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<ProxyListConfig[]>("get_proxy_lists");
      setLists(result);
    } catch {
      /* backend may not be ready */
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const saveList = useCallback(async (config: ProxyListConfig) => {
    try {
      const updated = await invoke<ProxyListConfig[]>("save_proxy_list", {
        config,
      });
      setLists(updated);
      return updated;
    } catch (err) {
      console.error("Failed to save proxy list:", err);
      throw err;
    }
  }, []);

  const deleteList = useCallback(async (id: string) => {
    try {
      const updated = await invoke<ProxyListConfig[]>("delete_proxy_list", {
        id,
      });
      setLists(updated);
      return updated;
    } catch (err) {
      console.error("Failed to delete proxy list:", err);
      throw err;
    }
  }, []);

  return { lists, loading, refresh, saveList, deleteList };
}

export function useProxyCacheStats() {
  const [stats, setStats] = useState<ProxyCacheStats | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await invoke<ProxyCacheStats>("get_proxy_cache_stats");
      setStats(result);
    } catch {
      /* backend may not be ready yet */
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 10_000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { stats, refresh };
}

export async function fetchProxyLists(): Promise<ProxyListConfig[]> {
  try {
    return await invoke<ProxyListConfig[]>("get_proxy_lists");
  } catch {
    return [];
  }
}
