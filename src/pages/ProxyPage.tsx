import { useState, useEffect, useRef } from "react";
import { Link, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useProxies } from "../contexts/ProxyContext";
import ManageProxiesRowCard, {
  type ManageProxiesRowPlaceholder,
} from "../components/ManageProxiesRowCard";
import { codeToFlag, extractCountryCode, getInitials } from "../utils/countryFlags";
import type { Proxy, ProxyInstanceInfo } from "../types";

const GEO_FILTER_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "All countries" },
  { value: "US", label: "United States (US)" },
  { value: "GB", label: "United Kingdom (GB)" },
  { value: "DE", label: "Germany (DE)" },
  { value: "FR", label: "France (FR)" },
  { value: "NL", label: "Netherlands (NL)" },
  { value: "CA", label: "Canada (CA)" },
  { value: "AU", label: "Australia (AU)" },
  { value: "JP", label: "Japan (JP)" },
  { value: "SG", label: "Singapore (SG)" },
  { value: "RU", label: "Russia (RU)" },
  { value: "BR", label: "Brazil (BR)" },
  { value: "IN", label: "India (IN)" },
  { value: "PL", label: "Poland (PL)" },
  { value: "CH", label: "Switzerland (CH)" },
  { value: "SE", label: "Sweden (SE)" },
  { value: "NO", label: "Norway (NO)" },
  { value: "FI", label: "Finland (FI)" },
  { value: "DK", label: "Denmark (DK)" },
  { value: "UA", label: "Ukraine (UA)" },
  { value: "TR", label: "Turkey (TR)" },
];

const STARTING_TO_CONNECTING_MS = 2000;

function isErrorStatus(s: ProxyInstanceInfo["status"]): s is { Error: string } {
  return typeof s === "object" && s !== null && "Error" in s;
}

function instanceToRow(
  inst: ProxyInstanceInfo,
  startingSince?: number
): ManageProxiesRowPlaceholder {
  const code =
    inst.upstream_country?.country_code != null && inst.upstream_country.country_code !== ""
      ? codeToFlag(inst.upstream_country.country_code) ||
        inst.upstream_country.country_code
      : extractCountryCode(inst.name) ?? getInitials(inst.name);

  let status: ManageProxiesRowPlaceholder["status"];
  if (inst.status === "Running") {
    status = "active";
  } else if (inst.status === "Starting") {
    const elapsed = startingSince ? Date.now() - startingSince : 0;
    status = elapsed >= STARTING_TO_CONNECTING_MS ? "connecting" : "starting";
  } else if (inst.status === "Stopped") {
    status = "idle";
  } else {
    status = "offline";
  }

  const upstreamMs = inst.upstream_latency_ms ?? 0;
  const avgMs = inst.stats?.avg_latency_ms ?? 0;
  const displayMs = upstreamMs > 0 ? upstreamMs : avgMs;

  let latency: string;
  if (inst.status === "Running") {
    latency = displayMs > 0 ? `${Math.round(displayMs)}ms` : "—";
  } else if (inst.status === "Starting") {
    latency = "Connecting...";
  } else if (inst.status === "Stopped") {
    latency = "—";
  } else if (isErrorStatus(inst.status)) {
    latency = inst.status.Error;
  } else {
    latency = "Error";
  }

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

export default function ProxyPage() {
  const navigate = useNavigate();
  const { instances } = useProxies();
  const [search, setSearch] = useState("");
  const [countryFilter, setCountryFilter] = useState("");
  const [poolMatchCount, setPoolMatchCount] = useState<number | null>(null);
  const [startingSince, setStartingSince] = useState<Record<string, number>>({});
  const prevInstancesRef = useRef<ProxyInstanceInfo[]>([]);

  useEffect(() => {
    const now = Date.now();
    setStartingSince((prev) => {
      const next: Record<string, number> = {};
      for (const inst of instances) {
        if (inst.status === "Starting") {
          const wasStarting =
            prevInstancesRef.current.find((p) => p.id === inst.id)?.status === "Starting";
          next[inst.id] = wasStarting && prev[inst.id] != null ? prev[inst.id] : now;
        }
      }
      return Object.keys(next).length ? { ...prev, ...next } : {};
    });
    prevInstancesRef.current = instances;
  }, [instances]);

  useEffect(() => {
    if (instances.length !== 1 || !countryFilter) {
      setPoolMatchCount(null);
      return;
    }
    let cancelled = false;
    const id = instances[0].id;
    invoke<Proxy[]>("filter_proxies_by_countries", {
      id,
      countryCodes: [countryFilter],
    })
      .then((proxies) => {
        if (!cancelled) setPoolMatchCount(proxies.length);
      })
      .catch(() => {
        if (!cancelled) setPoolMatchCount(null);
      });
    return () => {
      cancelled = true;
    };
  }, [instances, countryFilter]);

  const allRows: ManageProxiesRowPlaceholder[] = instances.map((inst) =>
    instanceToRow(inst, startingSince[inst.id])
  );

  const searchFiltered =
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

  const rows =
    countryFilter === ""
      ? searchFiltered
      : searchFiltered.filter((item) => {
          const inst = instances.find((i) => i.id === item.id);
          return inst?.upstream_country?.country_code === countryFilter;
        });

  const showEmptyState = instances.length === 0;
  const showNoSearchResults =
    rows.length === 0 &&
    instances.length > 0 &&
    (search.trim() !== "" || countryFilter !== "");

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

      <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4 mb-6">
        <div className="flex flex-col sm:flex-row gap-3 sm:items-center flex-wrap">
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="bg-surface-hover border border-border rounded-button px-4 py-2 w-full sm:w-80 text-[0.8125rem] outline-none focus:border-border-focus"
            placeholder="Search by location, IP, or type..."
          />
          <select
            value={countryFilter}
            onChange={(e) => setCountryFilter(e.target.value)}
            className="bg-surface-hover border border-border rounded-button px-4 py-2 text-[0.8125rem] outline-none focus:border-border-focus min-w-[12rem]"
            aria-label="Filter by upstream country"
          >
            {GEO_FILTER_OPTIONS.map((o) => (
              <option key={o.value || "all"} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          {countryFilter !== "" && poolMatchCount !== null && instances.length === 1 && (
            <span className="text-[0.75rem] text-foreground-muted">
              Cached pool: {poolMatchCount} proxy{poolMatchCount === 1 ? "" : "ies"} match (same protocol)
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
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

      {showNoSearchResults && (
        <div className="text-center py-12 text-foreground-muted text-[0.875rem]">
          {countryFilter !== "" && search.trim() === ""
            ? `No proxies with upstream in ${countryFilter}`
            : `No proxies match your filters`}
        </div>
      )}

      {showEmptyState ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="w-14 h-14 mb-5 rounded-card bg-surface-hover border border-border flex items-center justify-center">
            <svg
              width="28"
              height="28"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="text-foreground-muted"
            >
              <path d="M20 7h-9" />
              <path d="M14 17H5" />
              <circle cx="17" cy="17" r="3" />
              <circle cx="7" cy="7" r="3" />
            </svg>
          </div>
          <h3 className="text-[1rem] font-medium text-foreground mb-1">
            No proxy instances
          </h3>
          <p className="text-[0.8125rem] text-foreground-muted mb-6 max-w-sm">
            Create your first proxy to get started. You can add residential, datacenter, or Tor proxies.
          </p>
          <button
            type="button"
            onClick={() => navigate("/proxy/new")}
            className="h-9 px-4 rounded-button text-[0.8125rem] font-medium bg-foreground text-white dark:bg-white dark:text-[#1C1C1E] border-0 cursor-pointer transition-all duration-200 hover:opacity-90"
          >
            <span className="flex items-center gap-2">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M6 1V11M1 6H11" />
              </svg>
              New Proxy
            </span>
          </button>
        </div>
      ) : (
        <div className="overflow-x-auto pb-4 -mx-4 px-4 sm:mx-0 sm:px-0">
          <div className="flex flex-col gap-3 min-w-[900px]">
            {rows.map((item, index) => (
              <ManageProxiesRowCard
                key={item.id ?? `${item.endpoint}-${index}`}
                item={item}
                index={index}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
