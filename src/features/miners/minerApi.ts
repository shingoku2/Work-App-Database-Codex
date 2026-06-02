import { command } from "@/lib/tauri";
import type { Miner } from "@/types/db";

export type CreateMinerInput = Omit<Miner, "id">;
export type UpdateMinerInput = Miner;

export async function listMiners(): Promise<Miner[]> {
  return command<Miner[]>("list_miners");
}

export async function createMiner(input: CreateMinerInput): Promise<number> {
  return command<number>("create_miner", { input });
}

export async function updateMiner(input: UpdateMinerInput): Promise<void> {
  return command<void>("update_miner", { input });
}

export async function importMiners(miners: CreateMinerInput): Promise<{ imported: number }>;
export async function importMiners(miners: CreateMinerInput[]): Promise<{ imported: number }>;
export async function importMiners(miners: CreateMinerInput | CreateMinerInput[]): Promise<{ imported: number }> {
  return command<{ imported: number }>("import_miners", { miners: Array.isArray(miners) ? miners : [miners] });
}

export async function deleteMiner(id: number): Promise<void> {
  return command<void>("delete_miner", { id });
}
