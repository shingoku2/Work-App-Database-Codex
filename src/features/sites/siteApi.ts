import { command } from "@/lib/tauri";
import type { CreateSite, Site, UpdateSite } from "@/types/db";

export async function listSites(): Promise<Site[]> {
  return command<Site[]>("list_sites");
}

export async function createSite(input: CreateSite): Promise<Site> {
  return command<Site>("create_site", { input });
}

export async function updateSite(input: UpdateSite): Promise<Site> {
  return command<Site>("update_site", { input });
}

export async function deleteSite(id: number, version: number): Promise<void> {
  return command<void>("delete_site", { id, version });
}
