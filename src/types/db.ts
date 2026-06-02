export type MinerModel = "S21" | "S21+" | "S21 Pro" | "S21 XP";
export type MinerStatus = "In Service" | "Under Repair" | "RMA" | "Retired" | "Spare";
export type PartCategory = "Hashboard" | "Control Board" | "PSU" | "Fan" | "Cable" | "Misc";

export interface Miner {
  id: number;
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
}

export interface Part {
  sku: string;
  name: string;
  category: PartCategory;
  qty_on_hand: number;
  reorder_threshold: number;
  supplier: string | null;
  unit_cost: number;
  notes: string | null;
}

export interface DashboardSummary {
  unit_count: number;
  part_count: number;
  low_stock_count: number;
  units_by_status: Array<{ status: MinerStatus; count: number }>;
  low_stock_parts: Part[];
}
