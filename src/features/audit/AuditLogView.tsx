import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Panel, fieldClass, secondaryButtonClass } from '@/components/ui/Panel';
import { listAuditLog, type AuditLogEntry, type AuditLogQuery } from './auditApi';

const ACTIONS = [
  'login', 'logout', 'change_password', 'create_user', 'update_user', 'reset_user_password',
  'create_miner', 'update_miner', 'delete_miner', 'import_miners',
  'create_part', 'update_part', 'delete_part',
] as const;

const TARGET_TYPES = ['user', 'miner', 'part'] as const;

export function AuditLogView() {
  const [query, setQuery] = useState<AuditLogQuery>({
    limit: 50,
    offset: 0,
  });
  const [page, setPage] = useState(0);

  const { data = [], isLoading, error, refetch } = useQuery({
    queryKey: ['audit-log', query],
    queryFn: () => listAuditLog(query),
  });

  const handleFilterChange = (key: keyof AuditLogQuery, value: string | number | undefined) => {
    setQuery(prev => ({ ...prev, [key]: value, offset: 0 }));
    setPage(0);
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    setQuery(prev => ({ ...prev, offset: newPage * (query.limit ?? 50) }));
  };

  const formatValues = (values: Record<string, unknown> | null) => {
    if (!values) return '-';
    return Object.entries(values)
      .map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
      .join(', ');
  };

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Audit Log</h2>
        <p className="text-sm text-slate-500">View all administrative actions and changes.</p>
      </div>

      <Panel title="Filters">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div>
            <label className="block text-xs text-slate-400 mb-1">User ID</label>
            <input
              className={fieldClass}
              type="number"
              placeholder="All users"
              value={query.user_id ?? ''}
              onChange={e => handleFilterChange('user_id', Number(e.target.value) || undefined)}
            />
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">Action</label>
            <select
              className={fieldClass}
              value={query.action ?? ''}
              onChange={e => handleFilterChange('action', e.target.value || undefined)}
            >
              <option value="">All actions</option>
              {ACTIONS.map(action => (
                <option key={action} value={action}>{action}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">Target Type</label>
            <select
              className={fieldClass}
              value={query.target_type ?? ''}
              onChange={e => handleFilterChange('target_type', e.target.value || undefined)}
            >
              <option value="">All types</option>
              {TARGET_TYPES.map(type => (
                <option key={type} value={type}>{type}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">Target ID</label>
            <input
              className={fieldClass}
              placeholder="Filter by target ID"
              value={query.target_id ?? ''}
              onChange={e => handleFilterChange('target_id', e.target.value || undefined)}
            />
          </div>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div>
            <label className="block text-xs text-slate-400 mb-1">From (ISO date)</label>
            <input
              className={fieldClass}
              type="datetime-local"
              value={query.from ?? ''}
              onChange={e => handleFilterChange('from', e.target.value || undefined)}
            />
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">To (ISO date)</label>
            <input
              className={fieldClass}
              type="datetime-local"
              value={query.to ?? ''}
              onChange={e => handleFilterChange('to', e.target.value || undefined)}
            />
          </div>
          <div className="flex items-end">
            <button className={secondaryButtonClass} onClick={() => refetch()}>Refresh</button>
          </div>
        </div>
      </Panel>

      <Panel title="Entries">
        {isLoading ? (
          <div className="text-slate-400">Loading audit log...</div>
        ) : error ? (
          <div className="text-sm text-red-300">{String(error)}</div>
        ) : data.length === 0 ? (
          <div className="text-slate-400">No audit log entries found.</div>
        ) : (
          <>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-white/10 text-left text-slate-400">
                    <th className="pb-2 pr-4">Time</th>
                    <th className="pb-2 pr-4">User</th>
                    <th className="pb-2 pr-4">Action</th>
                    <th className="pb-2 pr-4">Target</th>
                    <th className="pb-2 pr-4">IP</th>
                    <th className="pb-2">Changes</th>
                  </tr>
                </thead>
                <tbody>
                  {data.map((entry) => (
                    <tr key={entry.id} className="border-b border-white/5 hover:bg-white/5">
                      <td className="py-2 pr-4 text-slate-300 font-mono text-xs">
                        {new Date(entry.created_at).toLocaleString()}
                      </td>
                      <td className="py-2 pr-4">
                        {entry.username ? (
                          <span className="text-sky-200">{entry.username}</span>
                        ) : (
                          <span className="text-slate-500">System</span>
                        )}
                      </td>
                      <td className="py-2 pr-4 text-emerald-300 font-medium">{entry.action}</td>
                      <td className="py-2 pr-4 text-slate-300">
                        {entry.target_type && entry.target_id && (
                          <>
                            {entry.target_type}:{' '}
                            <span className="font-mono">{entry.target_id}</span>
                            {entry.target_serial && (
                              <span className="text-slate-500 ml-1">({entry.target_serial})</span>
                            )}
                          </>
                        )}
                      </td>
                      <td className="py-2 pr-4 text-slate-500 font-mono text-xs">
                        {entry.ip_address ?? '-'}
                      </td>
                      <td className="py-2 pr-4 text-slate-400 max-w-md truncate">
                        {entry.old_values || entry.new_values ? (
                          <>
                            {entry.old_values && (
                              <span className="text-red-300">[-] {formatValues(entry.old_values)}</span>
                            )}
                            {entry.new_values && (
                              <span className="text-emerald-300 ml-2">[+] {formatValues(entry.new_values)}</span>
                            )}
                          </>
                        ) : (
                          '-'
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="flex items-center justify-between pt-2">
              <div className="text-sm text-slate-500">
                Showing {data.length} entries (page {page + 1})
              </div>
              <div className="flex gap-2">
                <button
                  className={secondaryButtonClass}
                  disabled={page === 0}
                  onClick={() => handlePageChange(page - 1)}
                >
                  Previous
                </button>
                <button
                  className={secondaryButtonClass}
                  disabled={data.length < (query.limit ?? 50)}
                  onClick={() => handlePageChange(page + 1)}
                >
                  Next
                </button>
              </div>
            </div>
          </>
        )}
      </Panel>
    </section>
  );
}