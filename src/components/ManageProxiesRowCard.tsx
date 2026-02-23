import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";

export type PlaceholderStatus = "active" | "idle" | "offline";
export type LatencyVariant = "ok" | "slow" | "offline" | "last";

export interface ManageProxiesRowPlaceholder {
  locationCode: string;
  locationName: string;
  endpoint: string;
  typeProtocol: string;
  status: PlaceholderStatus;
  latency: string;
  latencyVariant?: LatencyVariant;
  autoStartOnBoot: boolean;
  opacity?: number;
  /** When set, card is clickable and navigates to /proxy/:id */
  id?: string;
}

interface Props {
  item: ManageProxiesRowPlaceholder;
}

function StatusBadgePlaceholder({ status }: { status: PlaceholderStatus }) {
  const classes =
    status === "active"
      ? "bg-[rgba(52,199,89,0.1)] text-[#34C759]"
      : status === "idle"
        ? "bg-[rgba(142,142,147,0.1)] text-foreground-muted"
        : "bg-[rgba(255,59,48,0.1)] text-[#FF3B30]";
  const label = status === "active" ? "Active" : status === "idle" ? "Idle" : "Offline";
  return (
    <span
      className={`inline-flex w-fit px-2.5 py-1 rounded-badge text-[0.6875rem] font-semibold ${classes}`}
    >
      {label}
    </span>
  );
}

function LatencyDot({ variant }: { variant: LatencyVariant }) {
  const bg =
    variant === "ok"
      ? "bg-[#34C759]"
      : variant === "slow"
        ? "bg-[#FF9F0A]"
        : "bg-[#FF3B30]";
  return <span className={`w-1.5 h-1.5 rounded-full ${bg}`} />;
}

export default function ManageProxiesRowCard({ item }: Props) {
  const navigate = useNavigate();
  const [autoStart, setAutoStart] = useState(item.autoStartOnBoot);
  const variant: LatencyVariant =
    item.latencyVariant ?? (item.status === "offline" ? "offline" : item.latency.includes("ms") && parseInt(item.latency, 10) > 100 ? "slow" : "ok");
  const showDot =
    (item.status === "active" || item.status === "offline") && !item.latency.startsWith("Last:");

  const handleCardClick = () => {
    if (item.id) navigate(`/proxy/${item.id}`);
  };

  const handleAutoStartToggle = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const newValue = !autoStart;
    setAutoStart(newValue);
    if (item.id) {
      try {
        await invoke("toggle_auto_start_on_boot", { id: item.id, enabled: newValue });
      } catch (err) {
        console.error("Failed to toggle auto-start:", err);
        // Revert on error
        setAutoStart(!newValue);
      }
    }
  };

  return (
    <div
      role={item.id ? "button" : undefined}
      tabIndex={item.id ? 0 : undefined}
      onClick={item.id ? handleCardClick : undefined}
      onKeyDown={item.id ? (e) => { if (e.key === "Enter" || e.key === " ") handleCardClick(); } : undefined}
      className={`bg-surface border border-border rounded-card px-6 py-4 grid grid-cols-[240px_1fr_180px_180px_140px] items-center gap-6 transition-all duration-200 hover:shadow-card hover:border-border-focus ${item.id ? "cursor-pointer" : ""}`}
      style={item.opacity !== undefined ? { opacity: item.opacity } : undefined}
    >
      <div className="flex flex-col gap-1">
        <div className="text-[0.6875rem] text-foreground-tertiary font-semibold uppercase">
          Location
        </div>
        <div className="flex items-center gap-2.5">
          <div className="w-6 h-6 rounded-full bg-[#F0F0F0] flex items-center justify-center text-[0.6875rem] font-bold text-foreground dark:text-[#1C1C1E]">
            {item.locationCode}
          </div>
          <span className="text-[0.875rem] font-medium">{item.locationName}</span>
        </div>
      </div>

      <div className="flex flex-col gap-1">
        <div className="text-[0.6875rem] text-foreground-tertiary font-semibold uppercase">
          Endpoint
        </div>
        <div className="text-[0.8125rem] font-medium font-mono">{item.endpoint}</div>
      </div>

      <div className="flex flex-col gap-1">
        <div className="text-[0.6875rem] text-foreground-tertiary font-semibold uppercase">
          Type / Protocol
        </div>
        <div className="text-[0.875rem] font-medium">{item.typeProtocol}</div>
      </div>

      <div className="flex flex-col gap-1">
        <div className="text-[0.6875rem] text-foreground-tertiary font-semibold uppercase">
          Status
        </div>
        <StatusBadgePlaceholder status={item.status} />
        <div className="flex items-center gap-1.5 text-[0.75rem] text-foreground-muted">
          {showDot && variant !== "last" && <LatencyDot variant={variant} />}
          <span>{item.latency}</span>
        </div>
      </div>

      <div className="flex items-center gap-2.5">
        <button
          type="button"
          role="switch"
          aria-checked={autoStart}
          className={`w-8 h-[1.125rem] rounded-[1.25rem] relative cursor-pointer transition-colors duration-300 flex-shrink-0 ${
            autoStart ? "bg-foreground" : "bg-border"
          }`}
          onClick={handleAutoStartToggle}
        >
          <span
            className={`absolute top-0.5 left-0.5 w-3.5 h-3.5 rounded-full shadow-sm transition-transform duration-300 ${
              autoStart ? "bg-white dark:bg-[#1C1C1E]" : "bg-white"
            }`}
            style={{ transform: autoStart ? "translateX(0.875rem)" : "translateX(0)" }}
          />
        </button>
        <span className="text-[0.6875rem] text-foreground-muted leading-tight">
          Auto-start
          <br />
          on boot
        </span>
      </div>
    </div>
  );
}
