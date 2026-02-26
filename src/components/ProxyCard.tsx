import { useState, memo } from "react";
import { useNavigate } from "react-router-dom";
import { motion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import type { ProxyInstanceInfo } from "../types";
import StatusBadge from "./StatusBadge";
import AnonymityBadge from "./AnonymityBadge";
import { detectCountryFlag, getInitials } from "../utils/countryFlags";

interface Props {
  instance: ProxyInstanceInfo;
  busy: boolean;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onDelete: (id: string) => void;
  
  index?: number;
}

const protocolDisplayMap: Record<string, string> = {
  Http: "HTTP/S",
  Https: "HTTP/S",
  Socks4: "SOCKS4",
  Socks5: "SOCKS5",
  Tor: "TOR",
};

function getProtocol(inst: ProxyInstanceInfo): string {
  if (inst.mode === "Tor") return "TOR";
  if (inst.upstream) {
    return protocolDisplayMap[inst.upstream.protocol] ?? inst.upstream.protocol;
  }
  return protocolDisplayMap[inst.local_protocol] ?? inst.local_protocol;
}

function getModeLabel(inst: ProxyInstanceInfo): string {
  if (inst.mode === "Auto") return "Auto Rotate";
  if (inst.mode === "Manual") return "Manual";
  if (inst.mode === "Tor") return "Tor Network";
  return inst.mode;
}

function getLatencyDotColor(inst: ProxyInstanceInfo): string {
  if (inst.status === "Running") return "#34C759";
  if (inst.status === "Starting") return "#FF9F0A";
  if (inst.status === "Stopped") return "#8E8E93";
  return "#FF3B30"; 
}

function getLatencyText(inst: ProxyInstanceInfo): string {
  if (inst.status === "Running") return "Connected";
  if (inst.status === "Starting") return "Connecting...";
  if (inst.status === "Stopped") return "--";
  return "Error";
}

function getCardOpacity(inst: ProxyInstanceInfo): string {
  if (typeof inst.status === "object" && "Error" in inst.status) return "opacity-60";
  if (inst.status === "Stopped") return "";
  return "";
}

function ProxyCard({
  instance,
  busy: _busy,
  onStart: _onStart,
  onStop: _onStop,
  onDelete: _onDelete,
  index = 0,
}: Props) {
  const navigate = useNavigate();

  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<number | "fail" | null>(null);
  const [changingIp, setChangingIp] = useState(false);

  const countryFlag = detectCountryFlag(instance.name);
  const initials = getInitials(instance.name);
  const avatarContent = countryFlag ?? initials;
  const protocol = getProtocol(instance);
  const modeLabel = getModeLabel(instance);
  const latencyDot = getLatencyDotColor(instance);
  const latencyText = getLatencyText(instance);
  const opacityClass = getCardOpacity(instance);

  const ipAddress = instance.mode === "Tor"
    ? instance.bind_addr
    : instance.upstream
      ? instance.upstream.host
      : instance.bind_addr;
  const port = instance.mode === "Tor"
    ? instance.port
    : instance.upstream
      ? instance.upstream.port
      : instance.port;

  const isRunning = instance.status === "Running";

  const handleTestConnection = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (testing) return;
    setTesting(true);
    setTestResult(null);
    try {
      const latency = await invoke<number>("test_connection", { id: instance.id });
      setTestResult(latency);
      setTimeout(() => setTestResult(null), 4000);
    } catch {
      setTestResult("fail");
      setTimeout(() => setTestResult(null), 4000);
    } finally {
      setTesting(false);
    }
  };

  const handleChangeIp = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (changingIp) return;
    setChangingIp(true);
    try {
      await invoke("change_ip", { id: instance.id });
    } catch (err) {
      console.error("Change IP failed:", err);
    } finally {
      setChangingIp(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: "easeOut", delay: index * 0.05 }}
      onClick={() => navigate(`/proxy/${instance.id}`)}
      className={`col-span-12 md:col-span-6 xl:col-span-4 bg-surface-card border border-border rounded-card p-5 transition-all duration-200 cursor-pointer relative hover:border-border-focus hover:-translate-y-0.5 hover:shadow-float ${opacityClass}`}
    >
      {}
      <div className="flex justify-between items-center mb-4">
        <div className="flex items-center gap-2 font-medium text-[0.875rem] min-w-0">
          <div className="w-5 h-5 bg-surface-hover rounded-full grid place-items-center text-[0.625rem] font-semibold text-foreground-muted shrink-0">
            {avatarContent}
          </div>
          <span className="truncate">{instance.name}</span>
          {isRunning && instance.mode !== "Tor" && (
            <div className="flex items-center gap-1 shrink-0 ml-1">
              <button
                onClick={handleTestConnection}
                disabled={testing}
                className={`px-1.5 py-0.5 text-[0.625rem] font-medium rounded transition-colors ${
                  testResult === "fail"
                    ? "bg-red-500/15 text-red-400"
                    : testResult !== null
                      ? "bg-emerald-500/15 text-emerald-400"
                      : "bg-surface-hover hover:bg-border text-foreground-muted"
                } disabled:opacity-50`}
              >
                {testing
                  ? "..."
                  : testResult === "fail"
                    ? "Error"
                    : testResult !== null
                      ? `${testResult}ms`
                      : "Test"}
              </button>
              <button
                onClick={handleChangeIp}
                disabled={changingIp}
                className="px-1.5 py-0.5 text-[0.625rem] font-medium rounded bg-surface-hover hover:bg-border text-foreground-muted transition-colors disabled:opacity-50"
              >
                {changingIp ? "..." : "Change IP"}
              </button>
            </div>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <AnonymityBadge level={instance.anonymity_level} />
          <StatusBadge status={instance.status} />
        </div>
      </div>

      {}
      <div className="flex flex-col gap-2">
        <div className="flex justify-between text-[0.8125rem]">
          <span className="text-foreground-muted">IP Address</span>
          <span className="font-mono text-foreground bg-surface-hover px-1.5 py-0.5 rounded text-[0.75rem]">
            {ipAddress}
          </span>
        </div>
        <div className="flex justify-between text-[0.8125rem]">
          <span className="text-foreground-muted">Port</span>
          <span className="font-mono text-foreground bg-surface-hover px-1.5 py-0.5 rounded text-[0.75rem]">
            {port}
          </span>
        </div>
        <div className="flex justify-between text-[0.8125rem]">
          <span className="text-foreground-muted">Protocol</span>
          <span className="font-mono text-foreground bg-surface-hover px-1.5 py-0.5 rounded text-[0.75rem]">
            {protocol}
          </span>
        </div>
      </div>

      {}
      <div className="mt-4 pt-4 border-t border-border flex justify-between items-center text-[0.75rem] text-foreground-muted">
        <div>{modeLabel}</div>
        <div className="flex items-center">
          <span
            className="w-1.5 h-1.5 rounded-full inline-block mr-1"
            style={{ background: latencyDot }}
          />
          {latencyText}
        </div>
      </div>
    </motion.div>
  );
}

export default memo(ProxyCard);
