import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Panel, fieldClass } from "@/components/ui/Panel";
import type { AuditLogEntry } from "@/types/db";
import { listAuditLog } from "./auditApi";

export function AuditLogView() {
  const [actionFilter, setActionFilter] = useState("");
  const [targetTypeFilter, setTargetTypeFilter] = useState("");

  const query = {
    action: actionFilter.trim() || undefined,
    target_type: targetTypeFilter.trim() || undefined,
    limit: 200,
  };

  const { data = [], error, isLoading, refetch } = useQuery({
    queryKey: ["audit-log", query],
    queryFn: () => listAuditLog(query),
    staleTime: 30_000,
  });

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Audit Log</h2>
        <p className="text-sm text-slate-500">
          Recent server-side actions. Admin only.
        </p>
      </div>
      <Panel title="Filters">
        <div className="flex flex-wrap gap-3">
          <input
            className={fieldClass}
            placeholder="Action (e.g. miner.created)"
            value={actionFilter}
            onChange={(e) => setActionFilter(e.target.value)}
          />
          <input
            className={fieldClass}
            placeholder="Target type (e.g. miner, part, user)"
            value={targetTypeFilter}
            onChange={(e) => setTargetTypeFilter(e.target.value)}
          />
          <button
            type="button"
            className="rounded-md border border-white/10 px-3 py-2 text-sm text-slate-200 hover:bg-white/5"
            onClick={() => refetch()}
          >
            Refresh
          </button>
        </div>
      </Panel>
      {isLoading && <div className="text-slate-400">Loading audit log...</div>}
      {error && (
        <div className="text-sm text-red-300">{String(error)}</div>
      )}
      {!isLoading && data.length === 0 && (
        <div className="text-slate-500">No entries found.</div>
      )}
      {data.length > 0 && (
        <Panel title={`Entries (${data.length})`}>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-white/10 text-left text-xs text-slate-500">
                  <th className="pb-2 pr-4">Time</th>
                  <th className="pb-2 pr-4">User</th>
                  <th className="pb-2 pr-4">Action</th>
                  <th className="pb-2 pr-4">Target</th>
                  <th className="pb-2">IP</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/5">
                {data.map((entry) => (
                  <AuditRow key={entry.id} entry={entry} />
                ))}
              </tbody>
            </table>
          </div>
        </Panel>
      )}
    </section>
  );
}

function AuditRow({ entry }: { entry: AuditLogEntry }) {
  const ts = new Date(entry.created_at).toLocaleString();
  const target = [entry.target_type, entry.target_serial ?? entry.target_id]
    .filter(Boolean)
    .join(" ");
  return (
    <tr className="text-slate-300">
      <td className="py-1.5 pr-4 text-xs text-slate-500 whitespace-nowrap">{ts}</td>
      <td className="py-1.5 pr-4">{entry.username ?? "—"}</td>
      <td className="py-1.5 pr-4 font-mono text-xs text-sky-300">{entry.action}</td>
      <td className="py-1.5 pr-4 text-xs">{target || "—"}</td>
      <td className="py-1.5 text-xs text-slate-500">{entry.ip_address ?? "—"}</td>
    </tr>
  );
}
