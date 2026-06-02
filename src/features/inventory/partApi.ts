import { command } from "@/lib/tauri";
import type { Part } from "@/types/db";

export type CreatePartInput = Part;

export async function listParts(): Promise<Part[]> {
  return command<Part[]>("list_parts");
}

export async function createPart(input: CreatePartInput): Promise<void> {
  return command<void>("create_part", { input });
}

export async function updatePart(input: CreatePartInput): Promise<void> {
  return command<void>("update_part", { input });
}

export async function deletePart(sku: string): Promise<void> {
  return command<void>("delete_part", { sku });
}
