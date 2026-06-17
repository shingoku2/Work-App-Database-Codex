export type MinerModel = "S21" | "S21+" | "S21 Pro" | "S21 XP";
export type MinerStatus = "In Service" | "Under Repair" | "RMA" | "Retired" | "Spare";
export type PartCategory = "Hashboard" | "Control Board" | "PSU" | "Fan" | "Cable" | "Misc";

export interface Miner {
  id: number;
  site_id: number;
  site_name: string | null;
  serial: string;
  model: MinerModel;
  firmware: string | null;
  client_name: string | null;
  miner_type: string | null;
  ip_address: string | null;
  mac_address: string | null;
  pickaxe: string | null;
  miner_state: string | null;
  miner_row: string | null;
  miner_index: string | null;
  miner_rack: string | null;
  miner_rack_group: string | null;
  location: string | null;
  status: MinerStatus;
  acquired_date: string | null;
  notes: string | null;
  version: number;
}

export interface Part {
  site_id: number;
  site_name: string | null;
  sku: string;
  name: string;
  category: PartCategory;
  qty_on_hand: number;
  reorder_threshold: number;
  supplier: string | null;
  unit_cost_cents: number;
  notes: string | null;
  version: number;
}

export interface DashboardSummary {
  unit_count: number;
  part_count: number;
  low_stock_count: number;
  units_by_status: Array<{ status: MinerStatus; count: number }>;
  low_stock_parts: Part[];
}

export type UserRole = "admin" | "user";

export interface User {
  id: number;
  site_id: number | null;
  site_name: string | null;
  username: string;
  display_name: string;
  role: UserRole;
  enabled: boolean;
  version: number;
}

export interface ServerInfo {
  product: string;
  version: string;
  api_version: string;
}

export interface PairingInfo {
  server: ServerInfo;
  certificate_pem: string;
  fingerprint_sha256: string;
}

export interface ConnectionState {
  paired: boolean;
  status: "unpaired" | "unauthenticated" | "authenticated" | "unavailable" | "repair_required";
  url: string | null;
  fingerprint_sha256: string | null;
  user: User | null;
  error: string | null;
}

export interface TunnelStatus {
  supported: boolean;
  configured: boolean;
  running: boolean;
  local_port_open: boolean;
  local_url: string;
  remote_target: string;
  process_id: number | null;
  config_path: string | null;
  error: string | null;
}

export interface TunnelConfigInput {
  ssh_destination: string;
  ssh_port?: number | null;
  identity_file?: string | null;
  local_port?: number | null;
  remote_host?: string | null;
  remote_port?: number | null;
}

export interface TunnelKeyInfo {
  identity_file: string;
  public_key_file: string;
  public_key: string;
}

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

export interface AuditLogEntry {
  id: number;
  user_id: number | null;
  username: string | null;
  action: string;
  target_type: string | null;
  target_id: string | null;
  target_serial: string | null;
  old_values: unknown | null;
  new_values: unknown | null;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

export interface AuditLogQuery {
  user_id?: number | null;
  action?: string | null;
  target_type?: string | null;
  target_id?: string | null;
  from?: string | null;
  to?: string | null;
  limit?: number | null;
  offset?: number | null;
}

// ---------------------------------------------------------------------------
// Webhooks
// ---------------------------------------------------------------------------

/** secret is always "********" when set; null when no secret configured */
export interface Webhook {
  id: number;
  name: string;
  url: string;
  secret: string | null;
  events: string[];
  enabled: boolean;
  version: number;
}

export interface CreateWebhook {
  name: string;
  url: string;
  secret: string | null;
  events: string[];
  enabled: boolean;
}

export interface UpdateWebhook {
  id: number;
  name: string;
  url: string;
  /** null / "" / "********" preserves existing secret; new non-empty value replaces it */
  secret: string | null;
  events: string[];
  enabled: boolean;
  version: number;
}

export interface WebhookDelivery {
  id: number;
  webhook_id: number;
  event: string;
  payload: unknown;
  response_status: number | null;
  response_body: string | null;
  success: boolean;
  error: string | null;
  attempts: number;
  created_at: string;
  delivered_at: string | null;
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

export interface Site {
  id: number;
  name: string;
  code: string;
  description: string | null;
  enabled: boolean;
  version: number;
}

export interface CreateSite {
  name: string;
  code: string;
  description: string | null;
  enabled: boolean;
}

export interface UpdateSite {
  id: number;
  name: string;
  code: string;
  description: string | null;
  enabled: boolean;
  version: number;
}

// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

export interface SubmitTunnelKeyRequest {
  label: string;
  public_key: string;
}

export interface TunnelKeyRequest {
  id: number;
  label: string;
  public_key: string;
  status: "pending" | "approved" | "rejected" | "revoked";
  note: string | null;
  status_token: string;
  fingerprint_sha256: string | null;
  created_at: string;
}

export interface ApproveTunnelKeyRequest {
  note: string | null;
}

export interface TunnelClientConfig {
  ssh_destination: string;
  ssh_port: number;
  local_port: number;
  remote_host: string;
  remote_port: number;
}

export interface TunnelKeyRequestStatus {
  id: number;
  status: "pending" | "approved" | "rejected" | "revoked";
  note: string | null;
  client_config: TunnelClientConfig | null;
}

export interface TunnelKeyOnboardingState {
  request_id: number | null;
  status_token: string | null;
  label: string;
  public_key: string;
  server_url: string;
  identity_file: string;
}
