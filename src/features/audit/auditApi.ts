import { command } from '@/lib/tauri';

export interface AuditLogEntry {
  id: number;
  user_id: number | null;
  username: string | null;
  action: string;
  target_type: string | null;
  target_id: string | null;
  target_serial: string | null;
  old_values: Record<string, unknown> | null;
  new_values: Record<string, unknown> | null;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

export interface AuditLogQuery {
  user_id?: number;
  action?: string;
  target_type?: string;
  target_id?: string;
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}

export async function listAuditLog(query: AuditLogQuery = {}): Promise<AuditLogEntry[]> {
  return command<AuditLogEntry[]>('list_audit_log', { query });
}