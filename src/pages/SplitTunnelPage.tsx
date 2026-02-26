import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useProxies } from "../contexts/ProxyContext";
import type { RoutingRule } from "../types";
import CustomSelect from "../components/CustomSelect";
import CustomCheckbox from "../components/CustomCheckbox";

export default function SplitTunnelPage() {
  const { instances } = useProxies();
  const [rules, setRules] = useState<RoutingRule[]>([]);
  const [domainsInput, setDomainsInput] = useState("");
  const [instanceId, setInstanceId] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = async () => {
    const data = await invoke<RoutingRule[]>("list_split_tunnel_rules");
    setRules(data);
  };

  useEffect(() => {
    load().catch(() => {});
  }, []);

  const activeInstances = useMemo(
    () => instances.filter((i) => i.status === "Running" || i.status === "Stopped"),
    [instances],
  );

  const instanceOptions = useMemo(
    () =>
      activeInstances.map((inst) => ({
        value: inst.id,
        label: `${inst.name} (${inst.bind_addr}:${inst.port})`,
      })),
    [activeInstances],
  );

  const saveRule = async () => {
    const domains = domainsInput
      .split(/[,\n]/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (!instanceId || domains.length === 0) return;
    setSaving(true);
    try {
      await invoke("save_split_tunnel_rule", {
        req: {
          id: null,
          name: domains.slice(0, 3).join(", "),
          domains,
          proxy_instance_id: instanceId || null,
          enabled,
        },
      });
      setDomainsInput("");
      await load();
    } finally {
      setSaving(false);
    }
  };

  const removeRule = async (id: string) => {
    await invoke("delete_split_tunnel_rule", { id });
    await load();
  };

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Split Tunnel</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">Split Tunnel</h1>
      </header>

      <div className="bg-surface-card border border-border rounded-card p-5 mb-6">
        <h3 className="text-[0.95rem] font-semibold mb-4">New rule</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <textarea
            value={domainsInput}
            onChange={(e) => setDomainsInput(e.target.value)}
            placeholder="example.com, *.google.com"
            className="md:col-span-2 min-h-24 px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus"
          />
          <div className="flex flex-col gap-3">
            <CustomSelect
              options={instanceOptions}
              value={instanceId}
              onChange={setInstanceId}
              placeholder="Select proxy instance"
            />
            <CustomCheckbox
              checked={enabled}
              onChange={setEnabled}
              label="Enabled"
            />
            <button
              onClick={saveRule}
              disabled={saving}
              className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface disabled:opacity-50"
            >
              {saving ? "Saving..." : "Save rule"}
            </button>
          </div>
        </div>
      </div>

      <div className="bg-surface-card border border-border rounded-card p-5">
        <h3 className="text-[0.95rem] font-semibold mb-4">Rules</h3>
        {rules.length === 0 ? (
          <p className="text-[0.8125rem] text-foreground-muted">No split tunnel rules yet.</p>
        ) : (
          <div className="flex flex-col gap-2">
            {rules.map((rule) => (
              <div
                key={rule.id}
                className="flex items-center justify-between gap-3 px-3 py-2.5 bg-surface border border-border rounded-button"
              >
                <div>
                  <div className="text-[0.8125rem] font-medium">{rule.domains.join(", ")}</div>
                  <div className="text-[0.75rem] text-foreground-muted">
                    instance: {instances.find((i) => i.id === rule.proxy_instance_id)?.name ?? rule.proxy_instance_id}
                  </div>
                </div>
                <button
                  onClick={() => removeRule(rule.id)}
                  className="h-8 px-3 rounded-button text-[0.75rem] bg-[rgba(255,59,48,0.1)] text-[#FF3B30]"
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
