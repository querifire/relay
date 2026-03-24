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
  
  last_request_latency_ms: number;
}

export interface ConnectionLogEntry {
  timestamp_ms: number;
  target_host: string;
  protocol: string;
  bytes_sent: number;
  bytes_received: number;
  duration_ms: number;
  success: boolean;
  country_code?: string;
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
  
  has_auth: boolean;
  auto_rotate: boolean;
  auto_rotate_minutes: number | null;
  proxy_list: string;
  stats: ProxyStatsInfo;
  
  upstream_latency_ms: number;
  
  auto_start_on_boot: boolean;
  
  anonymity_level: AnonymityLevel | null;
  
  proxy_chain: ProxyChainConfig | null;

  /** GeoIP for current upstream (from backend when available). */
  upstream_country?: CountryInfo | null;
}

export interface ProxyCacheStats {
  total: number;
  socks5: number;
  socks4: number;
  http: number;
  last_updated: number; 
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
  tor_config: TorConfig;
  dns_protection: DnsResolverConfig;
  kill_switch: KillSwitchConfig;
  tls_fingerprint: TlsFingerprintConfig;
  start_hidden: boolean;
  notifications: NotificationSettings;
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

export type PluginType = "builtin" | "external";

export type PluginStatus =
  | "not_installed"
  | "installing"
  | "installed"
  | "enabled"
  | "error";

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  plugin_type: PluginType;
  installed: boolean;
  enabled: boolean;
  last_error: string | null;
}

export interface PluginUiInfo extends PluginInfo {
  status: PluginStatus;
}

export type BridgeType = "Obfs4" | "MeekAzure" | "Snowflake" | "WebTunnel" | "Custom";

export interface TorConfig {
  binary_path: string | null;
  socks_port: number;
  use_bridges: boolean;
  bridge_type: BridgeType;
  custom_bridges: string[];
  exit_nodes: string | null;
  entry_nodes: string | null;
  exclude_nodes: string | null;
  strict_nodes: boolean;
  custom_torrc: string | null;
}

export interface CountryInfo {
  country_code: string;
  country_name: string | null;
}

export interface SystemProxyInfo {
  enabled: boolean;
  host: string | null;
  port: number | null;
}

export interface Profile {
  id: string;
  name: string;
  description: string | null;
  settings: AppSettings;
  instances: string[];
  created_at: number;
  updated_at: number;
}

export interface SaveProfileRequest {
  id: string | null;
  name: string;
  description: string | null;
  settings: AppSettings;
  instances: string[];
}

export interface RoutingRule {
  id: string;
  name: string;
  domains: string[];
  proxy_instance_id: string | null;
  enabled: boolean;
}

export interface SaveRoutingRuleRequest {
  id: string | null;
  name: string;
  domains: string[];
  proxy_instance_id: string | null;
  enabled: boolean;
}

export interface ExportResult {
  path: string;
}

export interface NotificationSettings {
  enabled: boolean;
  proxy_start: boolean;
  proxy_stop: boolean;
  proxy_error: boolean;
  ip_changed: boolean;
  kill_switch: boolean;
  leak: boolean;
  tor: boolean;
}

export type ScheduleRepeat = "Once" | "Daily";

export type ScheduleAction =
  | { StartInstance: { instance_id: string } }
  | { StopInstance: { instance_id: string } }
  | { ChangeIp: { instance_id: string } };

export interface Schedule {
  id: string;
  name: string;
  time: string;
  action: ScheduleAction;
  repeat: ScheduleRepeat;
  enabled: boolean;
  last_run_day: number | null;
}

export interface SaveScheduleRequest {
  id: string | null;
  name: string;
  time: string;
  action: ScheduleAction;
  repeat: ScheduleRepeat;
  enabled: boolean;
}

export interface ProxyBandwidthDto {
  id: string;
  name: string;
  total_bytes: number;
  total_requests: number;
  successful_requests: number;
  avg_latency_ms: number;
  success_rate: number;
}

export interface BandwidthStatsDto {
  total_bytes: number;
  total_requests: number;
  per_proxy: ProxyBandwidthDto[];
}
