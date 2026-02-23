import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useProxies } from "../contexts/ProxyContext";
import ManageProxiesRowCard, {
  type ManageProxiesRowPlaceholder,
} from "../components/ManageProxiesRowCard";
import { extractCountryCode, getInitials } from "../utils/countryFlags";
import type { ProxyInstanceInfo } from "../types";

const MOCK_PROXIES: ManageProxiesRowPlaceholder[] = [
  {
    locationCode: "US",
    locationName: "United States (NY)",
    endpoint: "192.168.1.42:8080",
    typeProtocol: "Residential • HTTP/S",
    status: "active",
    latency: "42ms",
    latencyVariant: "ok",
    autoStartOnBoot: true,
  },
  {
    locationCode: "DE",
    locationName: "Germany (Frankfurt)",
    endpoint: "178.24.11.09:3128",
    typeProtocol: "Datacenter • SOCKS5",
    status: "active",
    latency: "124ms",
    latencyVariant: "slow",
    autoStartOnBoot: false,
  },
  {
    locationCode: "GB",
    locationName: "United Kingdom (London)",
    endpoint: "185.11.23.41:9050",
    typeProtocol: "Residential • SOCKS5",
    status: "active",
    latency: "38ms",
    latencyVariant: "ok",
    autoStartOnBoot: true,
  },
  {
    locationCode: "JP",
    locationName: "Japan (Tokyo)",
    endpoint: "104.22.14.99:8888",
    typeProtocol: "Mobile 4G • HTTP",
    status: "idle",
    latency: "Last: 82ms",
    latencyVariant: "last",
    autoStartOnBoot: false,
    opacity: 0.8,
  },
  {
    locationCode: "BR",
    locationName: "Brazil (São Paulo)",
    endpoint: "177.12.33.01:1080",
    typeProtocol: "Mobile 4G • SOCKS4",
    status: "offline",
    latency: "Timeout",
    latencyVariant: "offline",
    autoStartOnBoot: false,
    opacity: 0.6,
  },
];

function instanceToRow(inst: ProxyInstanceInfo): ManageProxiesRowPlaceholder {
  const code = extractCountryCode(inst.name) ?? getInitials(inst.name);
  const status: ManageProxiesRowPlaceholder["status"] =
    inst.status === "Running" || inst.status === "Starting"
      ? "active"
      : inst.status === "Stopped"
        ? "idle"
        : "offline";

  // Use upstream_latency_ms (from speed test) for displaying ms in the list.
  // Fall back to avg_latency_ms from proxied requests if upstream latency isn't set.
  const upstreamMs = inst.upstream_latency_ms ?? 0;
  const avgMs = inst.stats?.avg_latency_ms ?? 0;
  const displayMs = upstreamMs > 0 ? upstreamMs : avgMs;

  const latency =
    inst.status === "Running" && displayMs > 0
      ? `${Math.round(displayMs)}ms`
      : inst.status === "Running"
        ? "—"
        : inst.status === "Stopped"
          ? "—"
          : "Timeout";

  const latencyVariant: ManageProxiesRowPlaceholder["latencyVariant"] =
    inst.status === "Running" && displayMs > 0
      ? displayMs > 100 ? "slow" : "ok"
      : inst.status === "Stopped"
        ? "last"
        : "offline";

  const protocol =
    inst.local_protocol === "Socks5"
      ? "SOCKS5"
      : inst.local_protocol === "Socks4"
        ? "SOCKS4"
        : inst.local_protocol === "Https" || inst.local_protocol === "Http"
          ? "HTTP/S"
          : String(inst.local_protocol);
  const modeLabel =
    inst.mode === "Auto"
      ? "Auto Rotate"
      : inst.mode === "Tor"
        ? "Tor"
        : inst.mode;
  return {
    id: inst.id,
    locationCode: code,
    locationName: inst.name,
    endpoint: `${inst.bind_addr}:${inst.port}`,
    typeProtocol: `${modeLabel} • ${protocol}`,
    status,
    latency,
    latencyVariant,
    autoStartOnBoot: inst.auto_start_on_boot,
    opacity: status === "offline" ? 0.6 : status === "idle" ? 0.8 : undefined,
  };
}

function exportInstances(instances: ProxyInstanceInfo[]) {
  const lines = [
    "Name,Endpoint,Mode,Protocol,Status,Latency(ms)",
    ...instances.map((inst) => {
      const endpoint = `${inst.bind_addr}:${inst.port}`;
      const status = typeof inst.status === "string" ? inst.status : "Error";
      const latencyMs = inst.upstream_latency_ms > 0 ? inst.upstream_latency_ms : inst.stats?.avg_latency_ms ?? 0;
      return `"${inst.name}","${endpoint}","${inst.mode}","${inst.local_protocol}","${status}","${latencyMs}"`;
    }),
  ];
  const csv = lines.join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.style.display = "none";
  a.href = url;
  a.download = "relay-proxies.csv";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export default function ProxyPage() {
  const navigate = useNavigate();
  const { instances } = useProxies();
  const [search, setSearch] = useState("");

  const allRows: ManageProxiesRowPlaceholder[] =
    instances.length > 0
      ? instances.map(instanceToRow)
      : MOCK_PROXIES;

  const rows =
    search.trim() === ""
      ? allRows
      : allRows.filter((item) => {
          const q = search.toLowerCase();
          return (
            item.locationName.toLowerCase().includes(q) ||
            item.endpoint.toLowerCase().includes(q) ||
            item.typeProtocol.toLowerCase().includes(q)
          );
        });

  return (
    <div>
      <header className="mb-8">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <Link to="/" className="hover:text-foreground transition-colors">Home</Link>
          <span>/</span>
          <span className="text-foreground">Proxies</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em] text-foreground">
          Manage Proxies
        </h1>
      </header>

      <div className="flex justify-between items-center mb-6">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="bg-surface-hover border border-border rounded-button px-4 py-2 w-80 text-[0.8125rem] outline-none focus:border-border-focus"
          placeholder="Search by location, IP, or type..."
        />
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => exportInstances(instances)}
            className="h-9 px-4 rounded-button text-[0.8125rem] font-medium flex items-center gap-2 bg-surface-hover text-foreground border-0 cursor-pointer transition-all duration-200 hover:bg-border"
          >
            <svg
              width="13"
              height="13"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" />
            </svg>
            Export List
          </button>
          <button
            type="button"
            className="h-9 px-4 rounded-button text-[0.8125rem] font-medium flex items-center gap-2 bg-foreground text-white dark:bg-white dark:text-[#1C1C1E] border-0 cursor-pointer transition-all duration-200"
            onClick={() => navigate("/proxy/new")}
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M6 1V11M1 6H11" />
            </svg>
            New Proxy
          </button>
        </div>
      </div>

      {rows.length === 0 && search.trim() !== "" && (
        <div className="text-center py-12 text-foreground-muted text-[0.875rem]">
          No proxies match "{search}"
        </div>
      )}

      <div className="overflow-x-auto pb-4 -mx-4 px-4 sm:mx-0 sm:px-0">
        <div className="flex flex-col gap-3 min-w-[900px]">
          {rows.map((item, index) => (
            <ManageProxiesRowCard
              key={item.id ?? `${item.endpoint}-${index}`}
              item={item}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
