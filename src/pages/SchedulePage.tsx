import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useProxies } from "../contexts/ProxyContext";
import type { Schedule, ScheduleAction } from "../types";
import CustomSelect from "../components/CustomSelect";

type ActionKind = "start" | "stop" | "change_ip";

function describeAction(action: ScheduleAction): string {
  if ("StartInstance" in action) return "Start";
  if ("StopInstance" in action) return "Stop";
  if ("ChangeIp" in action) return "Change IP";
  return "Unknown";
}

function actionInstanceId(action: ScheduleAction): string {
  if ("StartInstance" in action) return action.StartInstance.instance_id;
  if ("StopInstance" in action) return action.StopInstance.instance_id;
  if ("ChangeIp" in action) return action.ChangeIp.instance_id;
  return "";
}

export default function SchedulePage() {
  const { instances } = useProxies();
  const [schedules, setSchedules] = useState<Schedule[]>([]);
  const [name, setName] = useState("");
  const [time, setTime] = useState("09:00");
  const [instanceId, setInstanceId] = useState("");
  const [action, setAction] = useState<ActionKind>("start");
  const [repeat, setRepeat] = useState<"Once" | "Daily">("Daily");
  const [saving, setSaving] = useState(false);

  const instanceOptions = useMemo(
    () => instances.map((inst) => ({ value: inst.id, label: inst.name })),
    [instances],
  );
  const actionOptions = [
    { value: "start", label: "Start proxy" },
    { value: "stop", label: "Stop proxy" },
    { value: "change_ip", label: "Change IP" },
  ];
  const repeatOptions = [
    { value: "Daily", label: "Daily" },
    { value: "Once", label: "Once" },
  ];

  const load = async () => {
    const data = await invoke<Schedule[]>("list_schedules");
    setSchedules(data);
  };

  useEffect(() => {
    load().catch(() => {});
  }, []);

  const buildAction = (): ScheduleAction => {
    if (action === "start") return { StartInstance: { instance_id: instanceId } };
    if (action === "stop") return { StopInstance: { instance_id: instanceId } };
    return { ChangeIp: { instance_id: instanceId } };
  };

  const save = async () => {
    if (!name.trim() || !instanceId) return;
    setSaving(true);
    try {
      await invoke("save_schedule", {
        req: {
          id: null,
          name: name.trim(),
          time,
          action: buildAction(),
          repeat,
          enabled: true,
        },
      });
      setName("");
      await load();
    } catch (e) {
      console.error("save_schedule failed:", e);
    } finally {
      setSaving(false);
    }
  };

  const remove = async (id: string) => {
    await invoke("delete_schedule", { id });
    await load();
  };

  const toggleEnabled = async (schedule: Schedule) => {
    await invoke("save_schedule", {
      req: {
        id: schedule.id,
        name: schedule.name,
        time: schedule.time,
        action: schedule.action,
        repeat: schedule.repeat,
        enabled: !schedule.enabled,
      },
    });
    await load();
  };

  const getInstanceName = (id: string) =>
    instances.find((i) => i.id === id)?.name ?? id.slice(0, 8);

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Schedule</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">Schedule</h1>
      </header>

      {}
      <div className="bg-surface-card border border-border rounded-card p-5 mb-6">
        <h3 className="text-[0.95rem] font-semibold mb-4">New schedule</h3>
        <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Name (e.g. Morning start)"
            className="px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors"
          />
          <input
            type="time"
            value={time}
            onChange={(e) => setTime(e.target.value)}
            className="px-3 py-2.5 text-[0.875rem] bg-surface border border-border rounded-button outline-none focus:border-border-focus transition-colors"
          />
          <CustomSelect
            options={instanceOptions}
            value={instanceId}
            onChange={setInstanceId}
            placeholder="Select instance…"
          />
          <CustomSelect
            options={actionOptions}
            value={action}
            onChange={(v) => setAction(v as ActionKind)}
            placeholder="Action…"
          />
          <div className="flex gap-2">
            <div className="flex-1">
              <CustomSelect
                options={repeatOptions}
                value={repeat}
                onChange={(v) => setRepeat(v as "Once" | "Daily")}
              />
            </div>
            <button
              onClick={save}
              disabled={saving || !name.trim() || !instanceId}
              className="h-10 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-surface hover:opacity-80 disabled:opacity-50 transition-all"
            >
              {saving ? "…" : "Add"}
            </button>
          </div>
        </div>
      </div>

      {}
      <div className="bg-surface-card border border-border rounded-card p-5">
        <h3 className="text-[0.95rem] font-semibold mb-4">
          Schedules
          {schedules.length > 0 && (
            <span className="ml-2 text-[0.75rem] font-normal text-foreground-muted">
              ({schedules.length})
            </span>
          )}
        </h3>
        {schedules.length === 0 ? (
          <p className="text-[0.8125rem] text-foreground-muted">
            No schedules yet. Add one above to automate proxy actions.
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {schedules.map((s) => {
              const iid = actionInstanceId(s.action);
              return (
                <div
                  key={s.id}
                  className={`flex items-center justify-between px-3 py-3 rounded-button border transition-colors ${
                    s.enabled
                      ? "border-border bg-surface"
                      : "border-border/50 bg-surface/50 opacity-60"
                  }`}
                >
                  <div className="min-w-0">
                    <div className="text-[0.875rem] font-medium">{s.name}</div>
                    <div className="text-[0.75rem] text-foreground-muted mt-0.5">
                      {s.time} · {describeAction(s.action)}{" "}
                      <span className="font-medium text-foreground">
                        {getInstanceName(iid)}
                      </span>{" "}
                      · {s.repeat}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 ml-4 shrink-0">
                    <button
                      onClick={() => toggleEnabled(s)}
                      className={`h-7 px-2.5 rounded-button text-[0.6875rem] font-medium border transition-colors ${
                        s.enabled
                          ? "border-border text-foreground-muted hover:bg-surface-hover"
                          : "border-border text-foreground-muted hover:bg-surface-hover"
                      }`}
                    >
                      {s.enabled ? "Disable" : "Enable"}
                    </button>
                    <button
                      onClick={() => remove(s.id)}
                      className="h-7 px-2.5 rounded-button text-[0.6875rem] bg-[rgba(255,59,48,0.1)] text-[#FF3B30] hover:bg-[rgba(255,59,48,0.2)] transition-colors"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
