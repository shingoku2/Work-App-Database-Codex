import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  Panel,
  fieldClass,
  primaryButtonClass,
  secondaryButtonClass,
} from "@/components/ui/Panel";
import type { Site } from "@/types/db";
import { createSite, deleteSite, listSites, updateSite } from "./siteApi";

export function SitesView() {
  const queryClient = useQueryClient();
  const { data = [], error, isLoading } = useQuery({
    queryKey: ["sites"],
    queryFn: listSites,
  });
  const [form, setForm] = useState({
    name: "",
    code: "",
    description: "",
    enabled: true,
  });
  const createMutation = useMutation({
    mutationFn: () =>
      createSite({
        name: form.name,
        code: form.code,
        description: form.description.trim() || null,
        enabled: form.enabled,
      }),
    onSuccess: async () => {
      setForm({ name: "", code: "", description: "", enabled: true });
      await queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
  });

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Sites</h2>
        <p className="text-sm text-slate-500">
          Manage physical sites. Miners, parts, and users can be scoped to a
          site.
        </p>
      </div>
      <Panel title="Create Site">
        <form
          className="grid grid-cols-2 gap-3 sm:grid-cols-4"
          onSubmit={(e) => {
            e.preventDefault();
            createMutation.mutate();
          }}
        >
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
            placeholder="Code (e.g. DEP_TX05)"
            value={form.code}
            onChange={(e) => setForm({ ...form, code: e.target.value.toUpperCase() })}
          />
          <input
            className={fieldClass}
            placeholder="Description (optional)"
            value={form.description}
            onChange={(e) => setForm({ ...form, description: e.target.value })}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.enabled}
              onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            />
            Enabled
          </label>
          <button className={primaryButtonClass} disabled={createMutation.isPending}>
            Create Site
          </button>
          {createMutation.error && (
            <span className="col-span-3 text-sm text-red-300">
              {String(createMutation.error)}
            </span>
          )}
        </form>
      </Panel>
      <Panel title="Existing Sites">
        {isLoading ? (
          <div className="text-slate-400">Loading sites...</div>
        ) : (
          <div className="space-y-3">
            {data.map((site) => (
              <SiteRow key={site.id} site={site} />
            ))}
          </div>
        )}
        {error && (
          <div className="mt-3 text-sm text-red-300">{String(error)}</div>
        )}
      </Panel>
    </section>
  );
}

function SiteRow({ site }: { site: Site }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState(site);
  const updateMutation = useMutation({
    mutationFn: () => updateSite(draft),
    onSuccess: async (updated) => {
      setDraft(updated);
      await queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: () => deleteSite(site.id, site.version),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
  });

  return (
    <div className="grid grid-cols-[1fr_1fr_1fr_80px_auto] items-center gap-3 rounded-md border border-white/10 p-3">
      <div>
        <div className="font-medium">{site.name}</div>
        <div className="text-xs text-slate-500">
          code: {site.code} · v{site.version}
        </div>
      </div>
      <input
        className={fieldClass}
        value={draft.name}
        onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        placeholder="Name"
      />
      <input
        className={`${fieldClass} uppercase`}
        value={draft.code}
        onChange={(e) =>
          setDraft({ ...draft, code: e.target.value.toUpperCase() })
        }
        placeholder="Code"
      />
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={draft.enabled}
          onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
        />
        Enabled
      </label>
      <div className="flex gap-2">
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
            window.confirm(
              `Delete site "${site.name}"? This will fail if any miners, parts, or users are still assigned to it.`,
            ) && deleteMutation.mutate()
          }
        >
          Delete
        </button>
      </div>
      {(updateMutation.error || deleteMutation.error) && (
        <div className="col-span-5 text-sm text-red-300">
          {String(updateMutation.error || deleteMutation.error)}
        </div>
      )}
    </div>
  );
}
