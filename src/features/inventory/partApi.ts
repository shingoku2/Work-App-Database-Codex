import { command } from "@/lib/tauri";
import type { Part } from "@/types/db";

export type CreatePartInput = Omit<Part, "site_id" | "site_name" | "version"> & { site_id: number | null };

export async function listParts(): Promise<Part[]> {
  return command<Part[]>("list_parts");
}

export async function createPart(input: CreatePartInput): Promise<void> {
  return command<void>("create_part", { input });
}

export async function updatePart(input: Part): Promise<void> {
  return command<void>("update_part", { input });
}

export async function deletePart(sku: string, version: number, siteId?: number | null): Promise<void> {
  return command<void>("delete_part", { sku, version, site_id: siteId ?? null });
}
