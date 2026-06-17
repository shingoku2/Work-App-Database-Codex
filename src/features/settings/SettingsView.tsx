import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  Panel,
  codeClass,
  fieldClass,
  primaryButtonClass,
  secondaryButtonClass,
} from "@/components/ui/Panel";
import type { User, Webhook, WebhookDelivery } from "@/types/db";
import { changePassword, logout, unpairServer } from "@/features/connection/connectionApi";
import {
  createWebhook,
  deleteWebhook,
  listWebhookDeliveries,
  listWebhooks,
  updateWebhook,
} from "./webhookApi";

const ALL_EVENTS = [
  "miner.created",
  "miner.updated",
  "miner.deleted",
  "part.created",
  "part.updated",
  "part.deleted",
  "user.created",
  "user.updated",
] as const;

export function SettingsView({ user, serverUrl }: { user: User; serverUrl: string }) {
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const refresh = async () => queryClient.invalidateQueries({ queryKey: ["connection"] });
  const passwordMutation = useMutation({
    mutationFn: () => changePassword(currentPassword, newPassword),
    onSuccess: refresh,
  });
  const logoutMutation = useMutation({ mutationFn: logout, onSuccess: refresh });
  const unpairMutation = useMutation({ mutationFn: unpairServer, onSuccess: refresh });

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Settings</h2>
        <p className="text-sm text-slate-500">
          Signed in as {user.display_name} ({user.username}).
        </p>
      </div>
      <Panel title="Server">
        <div className="flex items-center justify-between gap-4">
          <code className="text-sky-200">{serverUrl}</code>
          <button
            className={secondaryButtonClass}
            disabled={unpairMutation.isPending}
            onClick={() =>
              window.confirm(
                "Forget this server and remove the saved session?",
              ) && unpairMutation.mutate()
            }
          >
            Forget Server
          </button>
        </div>
      </Panel>
      <Panel title="Change Password">
        <form
          className="flex flex-wrap gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            passwordMutation.mutate();
          }}
        >
          <input
            className={fieldClass}
            required
            type="password"
            placeholder="Current password"
            value={currentPassword}
            onChange={(event) => setCurrentPassword(event.target.value)}
          />
          <input
            className={fieldClass}
            required
            minLength={12}
            type="password"
            placeholder="New password"
            value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)}
          />
          <button className={primaryButtonClass} disabled={passwordMutation.isPending}>
            Change Password
          </button>
        </form>
        {passwordMutation.error && (
          <div className="mt-3 text-sm text-red-300">
            {String(passwordMutation.error)}
          </div>
        )}
      </Panel>
      {user.role === "admin" && <WebhooksPanel />}
      {user.role === "admin" && (
        <Panel title="Backup / Restore">
          <p className="mb-3 text-sm text-slate-400">
            Backup and restore run server-side via CLI. Run these commands on the
            server host as the service account.
          </p>
          <div className="space-y-2 text-sm">
            <div>
              <span className="text-slate-500">Backup:</span>
              <pre className={`${codeClass} mt-1 block whitespace-pre-wrap p-3`}>
                {`antminer-fleet-server backup --output /backups/fleet-$(date +%F).dump --format custom`}
              </pre>
            </div>
            <div>
              <span className="text-slate-500">Restore:</span>
              <pre className={`${codeClass} mt-1 block whitespace-pre-wrap p-3`}>
                {`antminer-fleet-server restore /backups/fleet-<date>.dump --clean`}
              </pre>
            </div>
          </div>
        </Panel>
      )}
      {user.role === "admin" && (
        <Panel title="SSH Tunnel Keys">
          <p className="mb-3 text-sm text-slate-400">
            Manage pending tunnel key requests from the dedicated Tunnel Keys page in the sidebar.
          </p>
        </Panel>
      )}
      <button
        className={secondaryButtonClass}
        disabled={logoutMutation.isPending}
        onClick={() => logoutMutation.mutate()}
      >
        Sign Out
      </button>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Webhook management panel
// ---------------------------------------------------------------------------

function WebhooksPanel() {
  const queryClient = useQueryClient();
  const { data = [], error, isLoading } = useQuery({
    queryKey: ["webhooks"],
    queryFn: listWebhooks,
  });
  const [form, setForm] = useState({
    name: "",
    url: "",
    secret: "",
    enabled: true,
    events: [] as string[],
  });
  const createMutation = useMutation({
    mutationFn: () =>
      createWebhook({
        name: form.name,
        url: form.url,
        secret: form.secret.trim() || null,
        events: form.events,
        enabled: form.enabled,
      }),
    onSuccess: async () => {
      setForm({ name: "", url: "", secret: "", enabled: true, events: [] });
      await queryClient.invalidateQueries({ queryKey: ["webhooks"] });
    },
  });

  function toggleEvent(event: string) {
    setForm((f) => ({
      ...f,
      events: f.events.includes(event)
        ? f.events.filter((e) => e !== event)
        : [...f.events, event],
    }));
  }

  return (
    <Panel title="Webhooks">
      <p className="mb-4 text-sm text-slate-400">
        Outbound webhooks fire on mutation events. Secrets are never shown after
        save.
      </p>
      <form
        className="space-y-4 mb-6"
        onSubmit={(e) => {
          e.preventDefault();
          createMutation.mutate();
        }}
      >
        <div className="grid grid-cols-2 gap-3">
          <input
            className={fieldClass}
            required
            placeholder="Name"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
          />
          <input
            className={fieldClass}
            required
            type="url"
            placeholder="https://example.com/hook"
            value={form.url}
            onChange={(e) => setForm({ ...form, url: e.target.value })}
          />
          <input
            className={fieldClass}
            type="password"
            placeholder="Secret (optional)"
            value={form.secret}
            onChange={(e) => setForm({ ...form, secret: e.target.value })}
            autoComplete="new-password"
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.enabled}
              onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            />
            Enabled
          </label>
        </div>
        <div>
          <p className="mb-2 text-xs text-slate-500">Events</p>
          <div className="flex flex-wrap gap-2">
            {ALL_EVENTS.map((ev) => (
              <label key={ev} className="flex items-center gap-1.5 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={form.events.includes(ev)}
                  onChange={() => toggleEvent(ev)}
                />
                <span className={codeClass}>{ev}</span>
              </label>
            ))}
          </div>
        </div>
        <button className={primaryButtonClass} disabled={createMutation.isPending}>
          Add Webhook
        </button>
        {createMutation.error && (
          <div className="text-sm text-red-300">{String(createMutation.error)}</div>
        )}
      </form>
      {isLoading && <div className="text-slate-400">Loading webhooks...</div>}
      {error && <div className="text-sm text-red-300">{String(error)}</div>}
      <div className="space-y-3">
        {data.map((wh) => (
          <WebhookRow key={wh.id} webhook={wh} />
        ))}
      </div>
    </Panel>
  );
}

function WebhookRow({ webhook }: { webhook: Webhook }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState({
    ...webhook,
    secret: "", // blank = preserve existing
  });
  const [showDeliveries, setShowDeliveries] = useState(false);
  const { data: deliveries = [], isLoading: loadingDeliveries } = useQuery({
    queryKey: ["webhook-deliveries", webhook.id],
    queryFn: () => listWebhookDeliveries(webhook.id),
    enabled: showDeliveries,
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      updateWebhook({
        id: webhook.id,
        name: draft.name,
        url: draft.url,
        // empty string → preserve; "********" → preserve; new value → replace
        secret: draft.secret.trim() || null,
        events: draft.events,
        enabled: draft.enabled,
        version: webhook.version,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["webhooks"] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteWebhook(webhook.id, webhook.version),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["webhooks"] });
    },
  });

  function toggleEvent(event: string) {
    setDraft((d) => ({
      ...d,
      events: d.events.includes(event)
        ? d.events.filter((e) => e !== event)
        : [...d.events, event],
    }));
  }

  return (
    <div className="rounded-md border border-white/10 p-4 space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <input
          className={fieldClass}
          value={draft.name}
          onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          placeholder="Name"
        />
        <input
          className={fieldClass}
          type="url"
          value={draft.url}
          onChange={(e) => setDraft({ ...draft, url: e.target.value })}
          placeholder="URL"
        />
        <input
          className={fieldClass}
          type="password"
          value={draft.secret}
          onChange={(e) => setDraft({ ...draft, secret: e.target.value })}
          placeholder={webhook.secret ? "Leave blank to keep existing secret" : "No secret set"}
          autoComplete="new-password"
        />
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
          />
          Enabled
        </label>
      </div>
      <div>
        <p className="mb-2 text-xs text-slate-500">Events</p>
        <div className="flex flex-wrap gap-2">
          {ALL_EVENTS.map((ev) => (
            <label key={ev} className="flex items-center gap-1.5 text-xs cursor-pointer">
              <input
                type="checkbox"
                checked={draft.events.includes(ev)}
                onChange={() => toggleEvent(ev)}
              />
              <span className={codeClass}>{ev}</span>
            </label>
          ))}
        </div>
      </div>
      <div className="flex gap-2 flex-wrap">
        <button
          className={secondaryButtonClass}
          disabled={updateMutation.isPending}
          onClick={() => updateMutation.mutate()}
        >
          Save
        </button>
        <button
          className="rounded-md border border-red-400/30 px-3 py-2 text-sm text-red-300 hover:bg-red-400/10 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={deleteMutation.isPending}
          onClick={() =>
            window.confirm(`Delete webhook "${webhook.name}"?`) &&
            deleteMutation.mutate()
          }
        >
          Delete
        </button>
        <button
          className={secondaryButtonClass}
          onClick={() => setShowDeliveries((v) => !v)}
        >
          {showDeliveries ? "Hide" : "Show"} Deliveries
        </button>
      </div>
      {(updateMutation.error || deleteMutation.error) && (
        <div className="text-sm text-red-300">
          {String(updateMutation.error || deleteMutation.error)}
        </div>
      )}
      {showDeliveries && (
        <div className="mt-2 space-y-2">
          {loadingDeliveries ? (
            <div className="text-xs text-slate-400">Loading...</div>
          ) : deliveries.length === 0 ? (
            <div className="text-xs text-slate-500">No deliveries recorded.</div>
          ) : (
            deliveries.map((d) => <DeliveryRow key={d.id} delivery={d} />)
          )}
        </div>
      )}
    </div>
  );
}

function DeliveryRow({ delivery }: { delivery: WebhookDelivery }) {
  const ts = delivery.delivered_at
    ? new Date(delivery.delivered_at).toLocaleString()
    : new Date(delivery.created_at).toLocaleString();
  return (
    <div className="flex items-start gap-3 rounded border border-white/5 bg-white/5 p-2 text-xs">
      <span
        className={`rounded px-1.5 py-0.5 font-mono ${
          delivery.success ? "text-emerald-300" : "text-red-300"
        }`}
      >
        {delivery.success ? "OK" : "FAIL"}
      </span>
      <span className="text-slate-400 whitespace-nowrap">{ts}</span>
      <span className={codeClass}>{delivery.event}</span>
      {delivery.response_status && (
        <span className="text-slate-400">HTTP {delivery.response_status}</span>
      )}
      {delivery.error && (
        <span className="text-red-300 truncate max-w-xs">{delivery.error}</span>
      )}
    </div>
  );
}
