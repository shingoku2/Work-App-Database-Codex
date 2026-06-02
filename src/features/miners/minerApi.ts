import { command } from "@/lib/tauri";
import type { Miner } from "@/types/db";

export type CreateMinerInput = Omit<Miner, "id">;
export type UpdateMinerInput = Miner;

export interface MinerImportResult {
  imported: number;
  updated: number;
  skipped: number;
}

export async function listMiners(): Promise<Miner[]> {
  return command<Miner[]>("list_miners");
}

export async function createMiner(input: CreateMinerInput): Promise<number> {
  return command<number>("create_miner", { input });
}

export async function updateMiner(input: UpdateMinerInput): Promise<void> {
  return command<void>("update_miner", { input });
}

export async function importMiners(miners: CreateMinerInput): Promise<MinerImportResult>;
export async function importMiners(miners: CreateMinerInput[]): Promise<MinerImportResult>;
export async function importMiners(miners: CreateMinerInput | CreateMinerInput[]): Promise<MinerImportResult> {
  return command<MinerImportResult>("import_miners", { miners: Array.isArray(miners) ? miners : [miners] });
}

export async function deleteMiner(id: number): Promise<void> {
  return command<void>("delete_miner", { id });
}
