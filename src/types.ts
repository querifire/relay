export type ProxyProtocol = "Http" | "Https" | "Socks4" | "Socks5" | "Tor";

export type ProxyMode = "Auto" | "Manual" | "Tor";

export type ProxyStatusInfo =
  | "Stopped"
  | "Starting"
  | "Running"
  | { Error: string };

export type AnonymityLevel = "Transparent" | "Anonymous" | "Elite";

export interface Proxy {
  host: string;
  port: number;
  protocol: ProxyProtocol;
}

export interface ProxyChainConfig {
  enabled: boolean;
  proxies: Proxy[];
}

export interface ProxyStatsInfo {
  total_requests: number;
  successful_requests: number;
  avg_latency_ms: number;
  success_rate: number;
  total_bytes: number;
  /** Latency (ms) of the most recent proxied request — used for fluctuation chart. */
  last_request_latency_ms: number;
}

export interface ProxyInstanceInfo {
  id: string;
  name: string;
  bind_addr: string;
  port: number;
  mode: ProxyMode;
  status: ProxyStatusInfo;
  upstream: Proxy | null;
  local_protocol: ProxyProtocol;
  /** True if local proxy auth is configured (credentials not exposed to frontend). */
  has_auth: boolean;
  auto_rotate: boolean;
  auto_rotate_minutes: number | null;
  proxy_list: string;
  stats: ProxyStatsInfo;
  /** Latency (ms) of the upstream proxy from the last speed test. */
  upstream_latency_ms: number;
  /** Whether this instance auto-starts on application boot. */
  auto_start_on_boot: boolean;
  /** Anonymity level of the upstream proxy. */
  anonymity_level: AnonymityLevel | null;
  /** Proxy chain configuration. */
  proxy_chain: ProxyChainConfig | null;
}

export interface ProxyCacheStats {
  total: number;
  socks5: number;
  socks4: number;
  http: number;
  last_updated: number; // unix timestamp (seconds)
}

export interface ProxyListConfig {
  id: string;
  name: string;
  urls: string[];
  inline_proxies: string[];
}

export type Theme = "Dark" | "Light";

export type TlsFingerprintPreset = "Random" | "Chrome" | "Firefox" | "Safari" | "Default";

export interface TlsFingerprintConfig {
  enabled: boolean;
  preset: TlsFingerprintPreset;
}

export interface DnsResolverConfig {
  enabled: boolean;
  primary_server: string;
  fallback_servers: string[];
}

export interface KillSwitchConfig {
  enabled: boolean;
  active: boolean;
}

export interface AppSettings {
  theme: Theme;
  default_port: number;
  default_bind: string;
  concurrency: number;
  auto_rotate_minutes: number | null;
  tor_binary_path: string | null;
  tor_socks_port: number | null;
  dns_protection: DnsResolverConfig;
  kill_switch: KillSwitchConfig;
  tls_fingerprint: TlsFingerprintConfig;
  start_hidden: boolean;
}

export interface IpLeakResult {
  real_ip: string | null;
  proxy_ip: string | null;
  leak_detected: boolean;
  proxy_used: string | null;
}

export interface DnsLeakResult {
  dns_servers: string[];
  leak_detected: boolean;
}

export interface LeakTestResult {
  ip: IpLeakResult;
  dns: DnsLeakResult;
}
