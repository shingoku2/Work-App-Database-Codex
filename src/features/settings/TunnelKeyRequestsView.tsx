import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Panel } from "@/components/ui/Panel";
import type { TunnelKeyRequestAdmin } from "@/types/db";
import {
  approveTunnelKeyRequest,
  listTunnelKeyRequests,
  rejectTunnelKeyRequest,
  revokeTunnelKeyRequest,
} from "@/features/connection/connectionApi";

function PendingRequestRow({
  req,
  onDone,
}: {
  req: TunnelKeyRequestAdmin;
  onDone: () => void;
}) {
  const [note, setNote] = useState("");
  const approve = useMutation({
    mutationFn: () =>
      approveTunnelKeyRequest(req.id, { note: note.trim() || null }),
    onSuccess: onDone,
  });
  const reject = useMutation({
    mutationFn: () =>
      rejectTunnelKeyRequest(req.id, { note: note.trim() || null }),
    onSuccess: onDone,
  });

  return (
    <li className="rounded-lg border border-white/10 bg-[#0b1219] p-4">
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-sm text-slate-100">{req.label}</span>
        <span className="text-xs text-slate-500">
          {new Date(req.created_at).toLocaleString()}
        </span>
      </div>
      {req.fingerprint_sha256 && (
        <div className="mb-2 font-mono text-xs text-sky-300">
          {req.fingerprint_sha256}
        </div>
      )}
      <textarea
        className="mb-3 h-16 w-full resize-none rounded border border-white/10 bg-black/30 p-2 font-mono text-xs text-slate-300"
        readOnly
        value={req.public_key}
      />
      <textarea
        className="mb-3 h-14 w-full resize-none rounded border border-white/10 bg-black/20 p-2 text-xs text-slate-200"
        placeholder="Optional note: user/device verified, ticket number, laptop tag..."
        value={note}
        onChange={(event) => setNote(event.target.value)}
      />
      <div className="flex gap-2">
        <button
          className="rounded bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
          disabled={approve.isPending}
          onClick={() => approve.mutate()}
        >
          Approve
        </button>
        <button
          className="rounded bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-600 disabled:opacity-50"
          disabled={reject.isPending}
          onClick={() => reject.mutate()}
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
  );
}

function RecentRequestRow({
  req,
  onDone,
}: {
  req: TunnelKeyRequestAdmin;
  onDone: () => void;
}) {
  const [note, setNote] = useState("");
  const revoke = useMutation({
    mutationFn: () =>
      revokeTunnelKeyRequest(req.id, { note: note.trim() || null }),
    onSuccess: onDone,
  });

  const statusClass =
    req.status === "approved"
      ? "text-emerald-400"
      : req.status === "revoked"
        ? "text-amber-400"
        : "text-red-400";

  return (
    <li className="rounded border border-white/10 bg-[#0b1219] px-4 py-3 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-mono text-slate-200">{req.label}</span>
        <span className={statusClass}>{req.status}</span>
      </div>
      {req.fingerprint_sha256 && (
        <div className="mt-1 font-mono text-xs text-slate-500">
          {req.fingerprint_sha256}
        </div>
      )}
      {req.note && (
        <p className="mt-1 text-xs text-slate-400">Note: {req.note}</p>
      )}
      {req.status === "approved" && (
        <div className="mt-3 space-y-2">
          <textarea
            className="h-12 w-full resize-none rounded border border-white/10 bg-black/20 p-2 text-xs text-slate-200"
            placeholder="Optional revocation note..."
            value={note}
            onChange={(event) => setNote(event.target.value)}
          />
          <button
            className="rounded bg-amber-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-600 disabled:opacity-50"
            disabled={revoke.isPending}
            onClick={() => {
              if (
                window.confirm(
                  `Revoke SSH tunnel access for ${req.label}? This removes the key from authorized_keys.`,
                )
              ) {
                revoke.mutate();
              }
            }}
          >
            Revoke
          </button>
          {revoke.error && (
            <p className="text-xs text-red-300">{String(revoke.error)}</p>
          )}
        </div>
      )}
    </li>
  );
}

export function TunnelKeyRequestsView() {
  const queryClient = useQueryClient();
  const { data: requests = [], isLoading, error } = useQuery({
    queryKey: ["tunnelKeyRequests"],
    queryFn: listTunnelKeyRequests,
    refetchInterval: 15_000,
  });

  const refresh = () =>
    queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] });

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
              <PendingRequestRow key={req.id} req={req} onDone={refresh} />
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
              <RecentRequestRow key={req.id} req={req} onDone={refresh} />
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export function TunnelKeyRequestsPanel() {
  return (
    <Panel title="SSH Tunnel Keys">
      <p className="mb-4 text-sm text-slate-400">
        Approve client SSH public keys for the restricted tunnel account. Never distribute private
        keys from this console.
      </p>
      <TunnelKeyRequestsView />
    </Panel>
  );
}
