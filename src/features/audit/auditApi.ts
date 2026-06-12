import { command } from "@/lib/tauri";
import type { AuditLogEntry, AuditLogQuery } from "@/types/db";

export async function listAuditLog(
  query: AuditLogQuery = {},
): Promise<AuditLogEntry[]> {
  return command<AuditLogEntry[]>("list_audit_log", { query });
}
