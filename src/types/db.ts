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
