import { command } from "@/lib/tauri";
import type { DashboardSummary } from "@/types/db";

export async function getDashboardSummary(): Promise<DashboardSummary> {
  return command<DashboardSummary>("get_dashboard_summary");
}
