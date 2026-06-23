import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ColumnDef } from "@tanstack/react-table";
import { useState } from "react";
import { DataTable } from "@/components/ui/DataTable";
import { Panel, fieldClass, primaryButtonClass, secondaryButtonClass, textareaClass } from "@/components/ui/Panel";
import { invalidateFleetData } from "@/lib/queryInvalidation";
import type { Part, PartCategory } from "@/types/db";
import { createPart, deletePart, listParts, updatePart } from "./partApi";

const categories: PartCategory[] = ["Hashboard", "Control Board", "PSU", "Fan", "Cable", "Misc"];

const emptyPart: Part = {
  site_id: 0,
  site_name: null,
  sku: "",
  name: "",
  category: "Misc",
  qty_on_hand: 0,
  reorder_threshold: 0,
  supplier: "",
  unit_cost_cents: 0,
  notes: "",
  version: 0,
};

export function InventoryView() {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<Part>(emptyPart);
  const [editingSku, setEditingSku] = useState<string | null>(null);
  const { data = [], error, isLoading } = useQuery({ queryKey: ["parts"], queryFn: listParts });

  const saveMutation = useMutation({
    mutationFn: () => {
      if (editingSku) {
        return updatePart(form);
      }
      const { site_name: _siteName, version: _version, site_id, ...input } = form;
      return createPart({ ...input, site_id: site_id || null });
    },
    onSuccess: async () => {
      setForm(emptyPart);
      setEditingSku(null);
      await invalidateFleetData(queryClient, "parts");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: ({ sku, version, site_id }: Pick<Part, "sku" | "version" | "site_id">) => deletePart(sku, version, site_id),
    onSuccess: async () => {
      await invalidateFleetData(queryClient, "parts");
    },
  });

  const columns: ColumnDef<Part>[] = [
    { accessorKey: "sku", header: "SKU" },
    { accessorKey: "name", header: "Part" },
    { accessorKey: "category", header: "Category" },
    {
      accessorKey: "qty_on_hand",
      header: "On Hand",
      cell: ({ row }) => (
        <span className={row.original.qty_on_hand <= row.original.reorder_threshold ? "text-amber-200" : ""}>
          {row.original.qty_on_hand}
        </span>
      ),
    },
    { accessorKey: "reorder_threshold", header: "Threshold" },
    { accessorKey: "supplier", header: "Supplier", cell: ({ row }) => row.original.supplier || "-" },
    { accessorKey: "unit_cost_cents", header: "Unit Cost", cell: ({ row }) => formatCurrency(row.original.unit_cost_cents) },
    {
      id: "actions",
      header: "",
      cell: ({ row }) => (
        <div className="flex gap-2">
          <button type="button" className={secondaryButtonClass} onClick={() => { setEditingSku(row.original.sku); setForm(row.original); }}>
            Edit
          </button>
          <button type="button" className={secondaryButtonClass} onClick={() => { if (window.confirm(`Delete part "${row.original.sku}"?`)) deleteMutation.mutate({ sku: row.original.sku, version: row.original.version, site_id: row.original.site_id }); }}>
            Delete
          </button>
        </div>
      ),
    },
  ];

  return (
    <section className="space-y-5">
      <div>
        <h2 className="text-2xl font-semibold">Parts Inventory</h2>
        <p className="text-sm text-slate-500">Spare part stock, reorder thresholds, suppliers, and unit costs.</p>
      </div>

      <Panel title={editingSku ? `Edit ${editingSku}` : "Add Part"}>
        <form className="grid grid-cols-4 gap-3" onSubmit={(event) => { event.preventDefault(); saveMutation.mutate(); }}>
          <input className={fieldClass} required disabled={Boolean(editingSku)} placeholder="SKU / part number" value={form.sku} onChange={(event) => setForm({ ...form, sku: event.target.value })} />
          <input className={fieldClass} required placeholder="Part name" value={form.name} onChange={(event) => setForm({ ...form, name: event.target.value })} />
          <select className={fieldClass} value={form.category} onChange={(event) => setForm({ ...form, category: event.target.value as PartCategory })}>
            {categories.map((category) => <option key={category}>{category}</option>)}
          </select>
          <input className={fieldClass} type="number" min="0" step="1" value={form.qty_on_hand} onChange={(event) => setForm({ ...form, qty_on_hand: Number(event.target.value) })} />
          <input className={fieldClass} type="number" min="0" step="1" placeholder="Reorder threshold" value={form.reorder_threshold} onChange={(event) => setForm({ ...form, reorder_threshold: Number(event.target.value) })} />
          <input className={fieldClass} placeholder="Supplier" value={form.supplier ?? ""} onChange={(event) => setForm({ ...form, supplier: event.target.value || null })} />
          <input
            className={fieldClass}
            type="number"
            min="0"
            step="0.01"
            placeholder="Unit cost"
            value={(form.unit_cost_cents / 100).toFixed(2)}
            onChange={(event) => setForm({ ...form, unit_cost_cents: Math.round(Number(event.target.value) * 100) })}
          />
          <textarea className={textareaClass} placeholder="Notes" value={form.notes ?? ""} onChange={(event) => setForm({ ...form, notes: event.target.value || null })} />
          <div className="col-span-4 flex items-center gap-2">
            <button className={primaryButtonClass} disabled={saveMutation.isPending}>{editingSku ? "Save Part" : "Create Part"}</button>
            {editingSku && <button type="button" className={secondaryButtonClass} onClick={() => { setEditingSku(null); setForm(emptyPart); }}>Cancel</button>}
            {(saveMutation.error || deleteMutation.error || error) && (
              <span className="text-sm text-red-300">{String(saveMutation.error || deleteMutation.error || error)}</span>
            )}
          </div>
        </form>
      </Panel>

      {isLoading ? <div className="text-slate-400">Loading inventory...</div> : <DataTable columns={columns} data={data} searchPlaceholder="Filter inventory" />}
    </section>
  );
}

function formatCurrency(cents: number): string {
  return `$${(cents / 100).toFixed(2)}`;
}
