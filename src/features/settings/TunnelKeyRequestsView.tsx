import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Panel,
  primaryButtonClass,
  secondaryButtonClass,
} from "@/components/ui/Panel";
import type { ApproveTunnelKeyRequest, TunnelKeyRequest } from "@/types/db";
import {
  approveTunnelKeyRequest,
  listTunnelKeyRequests,
  rejectTunnelKeyRequest,
} from "@/features/connection/connectionApi";

export function TunnelKeyRequestsView() {
  const queryClient = useQueryClient();
  const { data: requests = [], isLoading, error } = useQuery({
    queryKey: ["tunnelKeyRequests"],
    queryFn: listTunnelKeyRequests,
    refetchInterval: 15_000,
  });

  const approve = useMutation({
    mutationFn: (id: number) => approveTunnelKeyRequest(id, { note: null }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] }),
  });

  const reject = useMutation({
    mutationFn: (id: number) => rejectTunnelKeyRequest(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] }),
  });

  if (isLoading) return <p className="text-sm text-slate-400">Loading...</p>;
  if (error)
    return (
      <p className="text-sm text-red-300">
        Failed to load requests: {String(error)}
      </p>
    );

  const pending = requests.filter((r) => r.status === "pending");
  const recent = requests.filter((r) => r.status !== "pending");

  return (
    <div className="space-y-6">
      <section>
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
          Pending ({pending.length})
        </h2>
        {pending.length === 0 ? (
          <p className="text-sm text-slate-500">No pending key requests.</p>
        ) : (
          <ul className="space-y-3">
            {pending.map((req) => (
              <li
                key={req.id}
                className="rounded-lg border border-white/10 bg-[#0b1219] p-4"
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="font-mono text-sm text-slate-100">
                    {req.label}
                  </span>
                  <span className="text-xs text-slate-500">
                    {new Date(req.created_at).toLocaleString()}
                  </span>
                </div>
                <textarea
                  className="mb-3 h-16 w-full resize-none rounded border border-white/10 bg-black/30 p-2 font-mono text-xs text-slate-300"
                  readOnly
                  value={req.public_key}
                />
                <div className="flex gap-2">
                  <button
                    className="rounded bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
                    disabled={approve.isPending}
                    onClick={() => approve.mutate(req.id)}
                  >
                    Approve
                  </button>
                  <button
                    className="rounded bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-600 disabled:opacity-50"
                    disabled={reject.isPending}
                    onClick={() => reject.mutate(req.id)}
                  >
                    Reject
                  </button>
                </div>
                {(approve.error || reject.error) && (
                  <p className="mt-2 text-xs text-red-300">
                    {String(approve.error ?? reject.error)}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {recent.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
            Recent
          </h2>
          <ul className="space-y-2">
            {recent.map((req) => (
              <li
                key={req.id}
                className="flex items-center justify-between rounded border border-white/10 bg-[#0b1219] px-4 py-2 text-sm"
              >
                <span className="font-mono text-slate-200">{req.label}</span>
                <span
                  className={
                    req.status === "approved"
                      ? "text-emerald-400"
                      : "text-red-400"
                  }
                >
                  {req.status}
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}